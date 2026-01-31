use std::path::Path;

use chrono::{Datelike, Utc};
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::matcher::apply_matcher_rules;
use crate::models::ImportSummary;
use crate::parsing::{parse_danish_date, parse_danish_decimal};

pub async fn import_csv(pool: &SqlitePool, path: &Path) -> AppResult<ImportSummary> {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("import.csv")
        .to_string();

    let imported_at = Utc::now().to_rfc3339();
    let mut tx = pool.begin().await?;
    let result = sqlx::query(
        "INSERT INTO imported_files (filename, imported_at) VALUES (?, ?)",
    )
    .bind(&filename)
    .bind(&imported_at)
    .execute(&mut *tx)
    .await?;
    let imported_file_id = result.last_insert_rowid();

    let bytes = std::fs::read(path)?;
    let decoded = match std::str::from_utf8(&bytes) {
        Ok(text) => text.to_string(),
        Err(_) => {
            let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
            text.to_string()
        }
    };
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(true)
        .from_reader(decoded.as_bytes());
    let headers = reader.headers()?.clone();
    let has_supplement = headers
        .iter()
        .any(|header| header.to_lowercase().contains("supp"));
    let has_currency = headers
        .iter()
        .any(|header| header.to_lowercase().contains("valuta"));
    let append_supplement = has_supplement || has_currency;

    let mut imported = 0u64;
    let mut duplicates = 0u64;

    let active_year = crate::db::get_active_year(pool).await?;
    for record in reader.records() {
        let record = record?;
        if record.len() < 7 {
            return Err(AppError::Parse("CSV row has too few columns".to_string()));
        }
        let booking_date = parse_danish_date(&record[0])?;
        if booking_date.year() != active_year {
            return Err(AppError::Parse(format!(
                "CSV year {} does not match active year {}",
                booking_date.year(),
                active_year
            )));
        }
        let value_date = parse_danish_date(&record[1])?;
        let mut text = record[2].trim().to_string();
        if append_supplement {
            let supplement = record.get(6).map(|value| value.trim()).unwrap_or("");
            if !supplement.is_empty() {
                text = format!("{} {}", text, supplement);
            }
        }
        let amount = parse_danish_decimal(&record[4])?;
        let balance = parse_danish_decimal(&record[5])?;
        let own_reference = if append_supplement {
            None
        } else {
            let own_reference = record.get(6).map(|value| value.trim()).unwrap_or("");
            if own_reference.is_empty() {
                None
            } else {
                Some(own_reference.to_string())
            }
        };

        let result = sqlx::query(
            "INSERT OR IGNORE INTO transactions \
             (imported_file_id, booking_date, value_date, text, amount, balance, own_reference) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(imported_file_id)
        .bind(booking_date)
        .bind(value_date)
        .bind(text)
        .bind(amount.to_string())
        .bind(balance.to_string())
        .bind(own_reference)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 1 {
            imported += 1;
        } else {
            duplicates += 1;
        }
    }

    tx.commit().await?;
    apply_matcher_rules(pool).await?;

    Ok(ImportSummary {
        imported,
        duplicates,
        imported_file_id,
    })
}
