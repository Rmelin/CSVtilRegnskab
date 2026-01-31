use std::path::PathBuf;

use chrono::Local;
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use std::str::FromStr;
use std::process::Command;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

use crate::db;
use crate::error::{AppError, AppResult};
use crate::models::TransactionFilters;

pub async fn export_csv(pool: &SqlitePool, year: i32) -> AppResult<PathBuf> {
    let rows = sqlx::query(
        "SELECT t.booking_date, t.value_date, t.text, \
            CAST(t.amount AS TEXT) as amount, CAST(t.balance AS TEXT) as balance, t.own_reference, \
            bg.name as budget_group_name, bp.name as budget_post_name, \
            sbp.name as suggested_budget_post_name, mr.name as matched_rule_name, t.confirmed \
         FROM transactions t \
         LEFT JOIN budget_posts bp ON t.assigned_budget_post_id = bp.id \
         LEFT JOIN budget_groups bg ON bp.group_id = bg.id \
         LEFT JOIN budget_posts sbp ON t.suggested_budget_post_id = sbp.id \
         LEFT JOIN matcher_rules mr ON t.matched_rule_id = mr.id \
         WHERE substr(t.booking_date, 1, 4) = ? \
         ORDER BY t.booking_date, t.id",
    )
    .bind(year.to_string())
    .fetch_all(pool)
    .await?;

    let mut path = default_download_dir().unwrap_or_else(std::env::temp_dir);
    let date_stamp = Local::now().format("%Y%m%d");
    path.push(format!(
        "foreningsregnskab_export_{}_{}.csv",
        date_stamp, year
    ));

    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    let mut writer = csv::WriterBuilder::new()
        .delimiter(b';')
        .quote_style(csv::QuoteStyle::Always)
        .from_path(&path)?;

    writer.write_record([
        "Dato",
        "Valoer",
        "Tekst",
        "Beloeb",
        "Saldo",
        "Egen bilagsreference",
        "budget_group_name",
        "budget_post_name",
        "suggested_budget_post_name",
        "matched_rule_name",
        "confirmed",
    ])?;

    for row in rows {
        let amount: String = row.try_get("amount")?;
        let balance: String = row.try_get("balance")?;
        writer.write_record([
            row.try_get::<String, _>("booking_date")?,
            row.try_get::<String, _>("value_date")?,
            row.try_get::<String, _>("text")?,
            amount,
            balance,
            row.try_get::<Option<String>, _>("own_reference")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("budget_group_name")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("budget_post_name")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("suggested_budget_post_name")?.unwrap_or_default(),
            row.try_get::<Option<String>, _>("matched_rule_name")?.unwrap_or_default(),
            (row.try_get::<i64, _>("confirmed")? == 1).to_string(),
        ])?;
    }

    writer.flush()?;
    Ok(path)
}

pub async fn export_kontering_csv(
    pool: &SqlitePool,
    filters: TransactionFilters,
) -> AppResult<PathBuf> {
    let mut base = QueryBuilder::<Sqlite>::new(
        "SELECT t.booking_date, t.text, CAST(t.amount AS TEXT) as amount, \
         bp.name as budget_post_name \
         FROM transactions t \
         LEFT JOIN budget_posts bp ON t.assigned_budget_post_id = bp.id \
         LEFT JOIN matcher_rules mr ON t.matched_rule_id = mr.id \
         WHERE 1=1",
    );

    db::apply_filters(&mut base, &filters);
    base.push(" ORDER BY t.booking_date, t.id");
    let rows = base.build().fetch_all(pool).await?;

    let mut path = default_download_dir().unwrap_or_else(std::env::temp_dir);
    let date_stamp = Local::now().format("%Y%m%d");
    let year_label = filters
        .year
        .map(|value| value.to_string())
        .unwrap_or_else(|| "alle".to_string());
    path.push(format!(
        "foreningsregnskab_kontering_{}_{}.csv",
        date_stamp, year_label
    ));

    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    let mut writer = csv::WriterBuilder::new()
        .delimiter(b';')
        .quote_style(csv::QuoteStyle::Always)
        .from_path(&path)?;

    writer.write_record(["Dato", "Tekst", "Beloeb", "Budgetpost"])?;

    for row in rows {
        writer.write_record([
            row.try_get::<String, _>("booking_date")?,
            row.try_get::<String, _>("text")?,
            row.try_get::<String, _>("amount")?,
            row.try_get::<Option<String>, _>("budget_post_name")?.unwrap_or_default(),
        ])?;
    }

    writer.flush()?;
    Ok(path)
}

pub async fn export_report_html(pool: &SqlitePool, year: i32) -> AppResult<PathBuf> {
    let settings = db::get_settings_for_year(pool, year).await?;
    let preview = db::get_report_preview(pool, year).await?;
    let notes = db::list_notes(pool, year).await?;
    let balance_curve = db::get_balance_curve(pool, year).await?;

    let mut path = default_download_dir().unwrap_or_else(std::env::temp_dir);
    let date_stamp = Local::now().format("%Y%m%d");
    path.push(format!(
        "foreningsregnskab_report_{}_{}.html",
        date_stamp, year
    ));

    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    let html = build_report_html(&settings, &preview, &notes, &balance_curve)?;
    std::fs::write(&path, html)?;
    Ok(path)
}

pub async fn export_report_pdf(
    pool: &SqlitePool,
    year: i32,
    club_slug: Option<&str>,
) -> AppResult<PathBuf> {
    let settings = db::get_settings_for_year(pool, year).await?;
    let preview = db::get_report_preview(pool, year).await?;
    let notes = db::list_notes(pool, year).await?;
    let balance_curve = db::get_balance_curve(pool, year).await?;
    let html = build_report_html(&settings, &preview, &notes, &balance_curve)?;

    let mut temp_path = std::env::temp_dir();
    let temp_slug = club_slug.unwrap_or("foreningsregnskab");
    temp_path.push(format!(
        "{}_report_{}_source.html",
        temp_slug, year
    ));
    std::fs::write(&temp_path, html)?;

    let mut output = default_download_dir().unwrap_or_else(std::env::temp_dir);
    let date_stamp = Local::now().format("%Y%m%d");
    let output_slug = club_slug.unwrap_or("foreningsregnskab");
    output.push(format!(
        "{}_report_{}_{}.pdf",
        output_slug, date_stamp, year
    ));
    if output.exists() {
        std::fs::remove_file(&output)?;
    }

    let chrome = find_chrome()?;
    let file_url = format!("file://{}", temp_path.to_string_lossy());
    let status = Command::new(&chrome)
        .arg("--headless")
        .arg("--disable-gpu")
        .arg("--no-sandbox")
        .arg(format!("--print-to-pdf={}", output.to_string_lossy()))
        .arg("--print-to-pdf-no-header")
        .arg("--no-pdf-header-footer")
        .arg("--disable-pdf-header-footer")
        .arg(&file_url)
        .status()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                AppError::Parse(format!(
                    "PDF renderer not found. Set CHROME_PATH or install Chrome. Tried: {}",
                    chrome.to_string_lossy()
                ))
            } else {
                AppError::Parse(format!("PDF renderer error: {}", err))
            }
        })?;
    if !status.success() {
        return Err(AppError::Parse("PDF renderer failed".to_string()));
    }
    Ok(output)
}

fn default_download_dir() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let mut path = PathBuf::from(home);
        path.push("Downloads");
        return Some(path);
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let mut path = PathBuf::from(home);
        path.push("Downloads");
        return Some(path);
    }
    None
}

fn build_report_html(
    settings: &crate::models::SettingsPayload,
    preview: &crate::models::ReportPreview,
    notes: &[crate::models::Note],
    balance_curve: &[crate::models::BalancePoint],
) -> AppResult<String> {
    let title_line1 = escape_html(settings.pdf_title_line1.as_deref().unwrap_or(""));
    let title_line2 = escape_html(settings.pdf_title_line2.as_deref().unwrap_or(""));
    let total_income = parse_decimal(&preview.total_income)?;
    let total_expense = parse_decimal(&preview.total_expense)?;
    let result_total = parse_decimal(&preview.result)?;
    let balance_start = parse_decimal(&preview.balance.start_balance)?;
    let balance_movements = parse_decimal(&preview.balance.movements)?;
    let balance_end = parse_decimal(&preview.balance.end_balance)?;

    let summary_cards = format!(
        "<div class=\"summary\">\n\
            <div class=\"summary-item\"><span>Indtægter</span><strong>{}</strong></div>\n\
            <div class=\"summary-item\"><span>Udgifter</span><strong>{}</strong></div>\n\
            <div class=\"summary-item\"><span>Resultat</span><strong>{}</strong></div>\n\
        </div>",
        format_kr(total_income),
        format_kr(total_expense),
        format_kr(result_total)
    );

    let income_table = render_group_table(
        "Indtægter",
        preview.year,
        &preview.income_groups,
        &preview.total_income,
        &preview.budget_current_total_income,
        &preview.budget_next_total_income,
    )?;
    let expense_table = render_group_table(
        "Udgifter",
        preview.year,
        &preview.expense_groups,
        &preview.total_expense,
        &preview.budget_current_total_expense,
        &preview.budget_next_total_expense,
    )?;

    let balance_svg = render_balance_svg(balance_curve);
    let balance_table = render_balance_table_html(
        preview.year,
        &format_kr(balance_start),
        &format_kr(balance_movements),
        &format_kr(balance_end),
    );

    let notes_html = if notes.is_empty() {
        "<p>Ingen noter.</p>".to_string()
    } else {
        notes
            .iter()
            .map(|note| {
                format!(
                    "<div class=\"note\"><h4>Note {}</h4><p>{}</p></div>",
                    note.note_number,
                    escape_html(note.body.as_str())
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let signature_block = if settings.signatures_enabled.unwrap_or(true) {
        render_signature_block_html(settings)
    } else {
        String::new()
    };

    let signature_section = if signature_block.is_empty() {
        String::new()
    } else {
        signature_block
    };

    let html = format!(
        "<!doctype html>\n\
<html lang=\"da\">\n\
<head>\n\
  <meta charset=\"utf-8\" />\n\
  <title>Foreningsregnskab {year}</title>\n\
  <style>{}</style>\n\
</head>\n\
<body>\n\
  <section class=\"page\">\n\
    <header class=\"report-header\">\n\
      <h1>{}</h1>\n\
      <h2>{}</h2>\n\
      <p>Regnskab for året {}</p>\n\
    </header>\n\
    {}\n\
    <div class=\"tables\">\n\
      {}\n\
      {}\n\
    </div>\n\
  </section>\n\
  <section class=\"page page-break\">\n\
    <h2>Kontobevægelser</h2>\n\
    <div class=\"chart\">{}\n\
    </div>\n\
    {}\n\
  </section>\n\
  <section class=\"page page-break\">\n\
    <h2>Noter til regnskabet</h2>\n\
    {}\n\
    {}\n\
  </section>\n\
</body>\n\
</html>",
        report_css(),
        title_line1,
        title_line2,
        preview.year,
        summary_cards,
        income_table,
        expense_table,
        balance_svg,
        balance_table,
        notes_html,
        signature_section,
        year = preview.year
    );

    Ok(html)
}

fn render_group_table(
    title: &str,
    year: i32,
    groups: &[crate::models::ReportGroupSummary],
    total_actual: &str,
    total_current: &str,
    total_next: &str,
) -> AppResult<String> {
    let mut rows = String::new();
    for group in groups {
        let subtotal = sum_group_totals(&group.posts)?;
        rows.push_str(&format!(
            "<tr class=\"group-row\"><td><strong>{}</strong></td><td class=\"num\"><strong>{}</strong></td><td class=\"num\"><strong>{}</strong></td><td class=\"num\"><strong>{}</strong></td></tr>",
            escape_html(group.name.as_str()),
            format_kr(subtotal.0),
            format_kr(subtotal.1),
            format_kr(subtotal.2)
        ));
        for post in &group.posts {
            let note = post.note_number.map(|value| format!("({})", value)).unwrap_or_default();
            rows.push_str(&format!(
                "<tr><td>{}</td><td class=\"num\">{}{} </td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
                escape_html(post.name.as_str()),
                format_amount(&post.total)?,
                note,
                format_amount(post.budget_current.as_deref().unwrap_or(""))?,
                format_amount(post.budget_next.as_deref().unwrap_or(""))?
            ));
        }
    }

    let totals = format!(
        "<tr class=\"total-row\"><td><strong>I alt</strong></td><td class=\"num\"><strong>{}</strong></td><td class=\"num\"><strong>{}</strong></td><td class=\"num\"><strong>{}</strong></td></tr>",
        format_amount(total_actual)?,
        format_amount(total_current)?,
        format_amount(total_next)?
    );

    Ok(format!(
        "<div class=\"table-block\"><h3>{}</h3><table><thead><tr><th>Post</th><th>Regnskab {}</th><th>Budget {}</th><th>Budget {}</th></tr></thead><tbody>{}</tbody><tfoot>{}</tfoot></table></div>",
        title,
        year,
        year,
        year + 1,
        rows,
        totals
    ))
}

fn render_balance_svg(points: &[crate::models::BalancePoint]) -> String {
    if points.is_empty() {
        return "<div class=\"empty\">Ingen saldodata.</div>".to_string();
    }
    let width = 900.0;
    let height = 240.0;
    let padding = 20.0;
    let mut min_value = f64::MAX;
    let mut max_value = f64::MIN;
    for point in points {
        if point.balance < min_value {
            min_value = point.balance;
        }
        if point.balance > max_value {
            max_value = point.balance;
        }
    }
    if (max_value - min_value).abs() < f64::EPSILON {
        max_value += 1.0;
        min_value -= 1.0;
    }
    if min_value > 0.0 {
        min_value = 0.0;
    }
    let range = max_value - min_value;

    let mut path = String::new();
    for (index, point) in points.iter().enumerate() {
        let x = padding + (index as f64 / (points.len() as f64 - 1.0).max(1.0)) * (width - 2.0 * padding);
        let ratio = (point.balance - min_value) / range;
        let y = (height - padding) - ratio * (height - 2.0 * padding);
        if index == 0 {
            path.push_str(&format!("M {:.2} {:.2}", x, y));
        } else {
            path.push_str(&format!(" L {:.2} {:.2}", x, y));
        }
    }

    let months = [
        "Jan", "Feb", "Mar", "Apr", "Maj", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dec",
    ];
    let mut month_labels = String::new();
    for (index, label) in months.iter().enumerate() {
        let x = padding + (index as f64 / 11.0) * (width - 2.0 * padding);
        let y = height - 4.0;
        month_labels.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" font-size=\"10\" fill=\"#5b6f60\" text-anchor=\"middle\">{}</text>",
            x, y, label
        ));
    }

    let mut y_labels = String::new();
    for i in 0..=4 {
        let value = min_value + (range / 4.0) * i as f64;
        let ratio = if range == 0.0 { 0.0 } else { (value - min_value) / range };
        let y = (height - padding) - ratio * (height - 2.0 * padding);
        let label = format_kr_decimal(value);
        y_labels.push_str(&format!(
            "<text x=\"{:.2}\" y=\"{:.2}\" font-size=\"10\" fill=\"#5b6f60\" text-anchor=\"start\">{}</text>",
            padding + 4.0,
            y + 3.0,
            label
        ));
    }

    format!(
        "<svg viewBox=\"0 0 {width} {height}\" xmlns=\"http://www.w3.org/2000/svg\">\n\
            <line x1=\"{padding}\" y1=\"{axis_y}\" x2=\"{axis_x}\" y2=\"{axis_y}\" stroke=\"#cfd6d1\" stroke-width=\"1\" />\n\
            <line x1=\"{padding}\" y1=\"{padding}\" x2=\"{padding}\" y2=\"{axis_y}\" stroke=\"#cfd6d1\" stroke-width=\"1\" />\n\
            {y_labels}\n\
            {month_labels}\n\
            <path d=\"{path}\" fill=\"none\" stroke=\"#fab387\" stroke-width=\"2\" />\n\
        </svg>",
        width = width,
        height = height,
        padding = padding,
        axis_y = height - padding,
        axis_x = width - padding,
        y_labels = y_labels,
        month_labels = month_labels,
        path = path
    )
}

fn format_kr_decimal(value: f64) -> String {
    let decimal = Decimal::from_f64(value).unwrap_or_else(|| Decimal::new(0, 2));
    format_kr(decimal)
}

fn render_balance_table_html(year: i32, start: &str, movements: &str, end: &str) -> String {
    format!(
        "<table class=\"balance\">\n\
            <thead>\n\
              <tr><th colspan=\"3\">BALANCE PR. 31.12.{}</th><th></th><th colspan=\"3\"></th></tr>\n\
              <tr><th>AKTIVER</th><th></th><th class=\"num\">Kr.</th><th></th><th>PASSIVER</th><th></th><th class=\"num\">Kr.</th></tr>\n\
            </thead>\n\
            <tbody>\n\
              <tr><td><strong>Bankbeholdning</strong></td><td>Primo</td><td class=\"num\">{}</td><td></td><td><strong>Egenkapital</strong></td><td>Primo</td><td class=\"num\">{}</td></tr>\n\
              <tr><td></td><td>Bevægelser</td><td class=\"num\">{}</td><td></td><td></td><td>Bevægelser</td><td class=\"num\">{}</td></tr>\n\
              <tr><td></td><td>Ultimo</td><td class=\"num\">{}</td><td></td><td></td><td>Ultimo</td><td class=\"num\">{}</td></tr>\n\
              <tr><td><strong>Aktiver i alt</strong></td><td></td><td class=\"num\"><strong>{}</strong></td><td></td><td><strong>Passiver i alt</strong></td><td></td><td class=\"num\"><strong>{}</strong></td></tr>\n\
            </tbody>\n\
        </table>",
        year, start, start, movements, movements, end, end, end, end
    )
}

fn render_signature_block_html(settings: &crate::models::SettingsPayload) -> String {
    format!(
        "<div class=\"signatures\">\n\
          <p>Regnskabet er gennemgået af revisor. Bankkontoen stemmer med regnskabet, og der er ingen bemærkninger.</p>\n\
          <table>\n\
            <tbody>\n\
              <tr><td>Formand</td><td>{}</td><td>Bestyrelsesmedlem</td><td>{}</td><td>Kasser</td><td>{}</td></tr>\n\
              <tr class=\"line\"><td></td><td>____________________</td><td></td><td>____________________</td><td></td><td>____________________</td></tr>\n\
              <tr><td>Bestyrelsesmedlem</td><td>{}</td><td>Bestyrelsesmedlem</td><td>{}</td><td>Bestyrelsesmedlem</td><td>{}</td></tr>\n\
              <tr class=\"line\"><td></td><td>____________________</td><td></td><td>____________________</td><td></td><td>____________________</td></tr>\n\
              <tr><td>Revisor</td><td>{}</td><td>Revisor</td><td>{}</td><td></td><td></td></tr>\n\
              <tr class=\"line\"><td></td><td>____________________</td><td></td><td>____________________</td><td></td><td></td></tr>\n\
            </tbody>\n\
          </table>\n\
        </div>",
        escape_html(settings.chair.as_deref().unwrap_or("")),
        escape_html(settings.board_member_one.as_deref().unwrap_or("")),
        escape_html(settings.treasurer.as_deref().unwrap_or("")),
        escape_html(settings.board_member_two.as_deref().unwrap_or("")),
        escape_html(settings.board_member_three.as_deref().unwrap_or("")),
        escape_html(settings.board_member_four.as_deref().unwrap_or("")),
        escape_html(settings.auditor_one.as_deref().unwrap_or("")),
        escape_html(settings.auditor_two.as_deref().unwrap_or(""))
    )
}

fn sum_group_totals(
    posts: &[crate::models::ReportPostSummary],
) -> AppResult<(Decimal, Decimal, Decimal)> {
    let mut actual_total = Decimal::new(0, 2);
    let mut budget_current = Decimal::new(0, 2);
    let mut budget_next = Decimal::new(0, 2);
    for post in posts {
        actual_total += parse_decimal(&post.total)?;
        if let Some(value) = post.budget_current.as_deref() {
            if !value.trim().is_empty() {
                budget_current += parse_decimal(value)?;
            }
        }
        if let Some(value) = post.budget_next.as_deref() {
            if !value.trim().is_empty() {
                budget_next += parse_decimal(value)?;
            }
        }
    }
    Ok((actual_total, budget_current, budget_next))
}

fn parse_decimal(value: &str) -> AppResult<Decimal> {
    Decimal::from_str(value).map_err(|_| AppError::Parse("Invalid decimal".to_string()))
}

fn format_kr(value: Decimal) -> String {
    format!("{} Kr.", format_danish_decimal(value))
}

fn format_amount(value: &str) -> AppResult<String> {
    if value.trim().is_empty() {
        return Ok("".to_string());
    }
    let decimal = parse_decimal(value)?;
    Ok(format_danish_decimal(decimal))
}

fn format_danish_decimal(value: Decimal) -> String {
    let sign = if value.is_sign_negative() { "-" } else { "" };
    let value = value.abs();
    let integer = value.trunc();
    let fractional = (value.fract() * Decimal::new(100, 0))
        .round()
        .to_i32()
        .unwrap_or(0);
    let mut integer_str = integer.to_string();
    let mut formatted = String::new();
    while integer_str.len() > 3 {
        let split_at = integer_str.len() - 3;
        let chunk = integer_str.split_off(split_at);
        formatted = format!(".{}{}", chunk, formatted);
    }
    formatted = format!("{}{}", integer_str, formatted);
    format!("{}{},{}", sign, formatted, format!("{:02}", fractional))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn report_css() -> String {
    r#"
    :root { --ink: #24352b; --muted: #5b6f60; --line: #d5ddd8; --accent: #3b5d4a; }
    @page { size: A4; margin: 10mm; }
    body { font-family: "Arial", sans-serif; color: var(--ink); margin: 0; padding: 12px; }
    h1 { font-size: 20px; margin: 0; }
    h2 { font-size: 14px; margin: 2px 0 6px; }
    h3 { font-size: 13px; margin: 8px 0 4px; }
    h4 { font-size: 12px; margin: 6px 0 4px; }
    p { margin: 4px 0; }
    .page { page-break-inside: avoid; }
    .page-break { page-break-before: always; margin-top: 8px; }
    .report-header { margin-bottom: 8px; }
    .summary { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; margin: 8px 0; }
    .summary-item { border: 1px solid var(--line); padding: 6px; border-radius: 6px; }
    .summary-item span { display: block; font-size: 11px; color: var(--muted); }
    .summary-item strong { font-size: 13px; }
    .tables { display: grid; gap: 12px; }
    table { width: 100%; border-collapse: collapse; font-size: 11px; margin: 0; }
    th, td { padding: 4px 6px; border-bottom: 1px solid var(--line); }
    th { text-align: left; background: #f6f8f7; }
    .num { text-align: right; white-space: nowrap; }
    .group-row td { background: #eef3f0; }
    .total-row td { background: #e6ece8; }
    .chart { margin: 6px 0 10px; }
    .balance th { text-transform: uppercase; font-size: 10px; }
    .balance td { border-bottom: none; }
    .signatures { margin-top: 10px; }
    .signatures table td { border-bottom: none; padding: 3px 6px; }
    .signatures .line td { padding-bottom: 8px; }
    .empty { color: var(--muted); }
    @media print {
      body { padding: 0; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
      .page-break { margin-top: 0; }
      h2 { margin-top: 0; }
    }
    "#.to_string()
}

fn find_chrome() -> AppResult<std::path::PathBuf> {
    if let Ok(path) = std::env::var("CHROME_PATH") {
        let candidate = std::path::PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ];
    for candidate in candidates {
        let path = std::path::PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Ok(std::path::PathBuf::from("google-chrome"))
}
