use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{Datelike, Local, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

use crate::error::{AppError, AppResult};
use crate::models::{
    AssignRequest, BalancePoint, BalanceSummary, BatchAssignRequest, BudgetGroup, BudgetGroupInput,
    BudgetPost, BudgetPostInput, BudgetValueRowInput, MatcherRule, MatcherRuleInput, PageRequest,
    PagedTransactions, ProgressSummary, ReportGroupSummary, ReportPostSummary, ReportPreview,
    RuleMatchStat, SettingsPayload, TransactionFilters, TransactionView, Note, NoteAssignmentInput,
    ReportCategoryAssignment,
    ReconciliationItem, ReconciliationSummary,
};
use crate::balance::build_balance_curve;
use crate::parsing::parse_danish_decimal;
use crate::parsing::parse_kontingent_info;

pub async fn init_pool(db_path: &Path) -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    ensure_active_year(&pool).await?;
    Ok(pool)
}

pub fn clubs_root_path() -> AppResult<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        let mut dir = PathBuf::from(home);
        dir.push("Library");
        dir.push("Application Support");
        dir.push("forening-regnskab");
        dir.push("clubs");
        std::fs::create_dir_all(&dir)?;
        return Ok(dir);
    }
    let mut dir = std::env::current_dir()?;
    dir.push("clubs");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn club_dir_path(slug: &str) -> AppResult<PathBuf> {
    let mut dir = clubs_root_path()?;
    dir.push(slug);
    Ok(dir)
}

pub fn club_db_path(slug: &str) -> AppResult<PathBuf> {
    let mut dir = club_dir_path(slug)?;
    dir.push("forening_regnskab.sqlite");
    Ok(dir)
}

pub fn ensure_club_db_path(slug: &str) -> AppResult<PathBuf> {
    let mut dir = club_dir_path(slug)?;
    std::fs::create_dir_all(&dir)?;
    dir.push("forening_regnskab.sqlite");
    Ok(dir)
}

pub fn list_clubs() -> AppResult<Vec<String>> {
    let root = clubs_root_path()?;
    let mut clubs = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        let db_path = entry.path().join("forening_regnskab.sqlite");
        if db_path.is_file() {
            clubs.push(slug);
        }
    }
    clubs.sort();
    Ok(clubs)
}

pub fn delete_club(slug: &str) -> AppResult<()> {
    let dir = club_dir_path(slug)?;
    if !dir.exists() {
        return Err(AppError::Parse("Klubben findes ikke".to_string()));
    }
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

pub fn rename_club(from_slug: &str, to_slug: &str) -> AppResult<()> {
    let from_dir = club_dir_path(from_slug)?;
    let to_dir = club_dir_path(to_slug)?;
    if !from_dir.exists() {
        return Err(AppError::Parse("Klubben findes ikke".to_string()));
    }
    if to_dir.exists() {
        return Err(AppError::Parse("Målet findes allerede".to_string()));
    }
    std::fs::rename(from_dir, to_dir)?;
    Ok(())
}

pub async fn list_transactions(
    pool: &SqlitePool,
    filters: TransactionFilters,
    page: PageRequest,
) -> AppResult<PagedTransactions> {
    let mut base = QueryBuilder::new(
        "SELECT t.id, t.booking_date, t.value_date, t.text, \
         CAST(t.amount AS TEXT) as amount, CAST(t.balance AS TEXT) as balance, t.own_reference, \
         t.suggested_budget_post_id, t.assigned_budget_post_id, t.matched_rule_id, t.confirmed, \
         bg.name as budget_group_name, bp.name as budget_post_name, sbp.name as suggested_budget_post_name, \
         mr.name as matched_rule_name \
         FROM transactions t \
         LEFT JOIN budget_posts bp ON t.assigned_budget_post_id = bp.id \
         LEFT JOIN budget_groups bg ON bp.group_id = bg.id \
         LEFT JOIN budget_posts sbp ON t.suggested_budget_post_id = sbp.id \
         LEFT JOIN matcher_rules mr ON t.matched_rule_id = mr.id \
         WHERE 1=1",
    );

    apply_filters(&mut base, &filters);

    base.push(" ORDER BY t.booking_date DESC, t.id DESC ");
    base.push(" LIMIT ");
    base.push_bind(page.page_size as i64);
    base.push(" OFFSET ");
    base.push_bind((page.page * page.page_size) as i64);

    let rows = base.build().fetch_all(pool).await?;
    let mut items = Vec::new();
    for row in rows {
        let text: String = row.try_get("text")?;
        let matched_rule_name: Option<String> = row.try_get("matched_rule_name")?;
        let kontingent = parse_kontingent_info(&text, matched_rule_name.as_deref());
        let (kontingent_member_id, kontingent_member_name) = match kontingent {
            Some(info) => (Some(info.member_id), Some(info.member_name)),
            None => (None, None),
        };
        let amount_raw: String = row.try_get("amount")?;
        let balance_raw: String = row.try_get("balance")?;
        let amount = Decimal::from_str(&amount_raw)
            .map_err(|_| AppError::Parse("Invalid amount".to_string()))?;
        let balance = Decimal::from_str(&balance_raw)
            .map_err(|_| AppError::Parse("Invalid balance".to_string()))?;
        items.push(TransactionView {
            id: row.try_get("id")?,
            booking_date: row.try_get::<NaiveDate, _>("booking_date")?,
            value_date: row.try_get::<NaiveDate, _>("value_date")?,
            text: text.clone(),
            amount,
            balance,
            own_reference: row.try_get("own_reference")?,
            suggested_budget_post_id: row.try_get("suggested_budget_post_id")?,
            assigned_budget_post_id: row.try_get("assigned_budget_post_id")?,
            matched_rule_id: row.try_get("matched_rule_id")?,
            confirmed: row.try_get::<i64, _>("confirmed")? == 1,
            budget_group_name: row.try_get("budget_group_name")?,
            budget_post_name: row.try_get("budget_post_name")?,
            suggested_budget_post_name: row.try_get("suggested_budget_post_name")?,
            matched_rule_name,
            kontingent_member_id,
            kontingent_member_name,
        });
    }

    let total = count_transactions(pool, &filters).await?;

    Ok(PagedTransactions {
        items,
        page: page.page,
        page_size: page.page_size,
        total,
    })
}

async fn count_transactions(pool: &SqlitePool, filters: &TransactionFilters) -> AppResult<u64> {
    let mut base = QueryBuilder::new(
        "SELECT COUNT(*) as count FROM transactions t \
         LEFT JOIN matcher_rules mr ON t.matched_rule_id = mr.id \
         WHERE 1=1",
    );
    apply_filters(&mut base, filters);
    let row = base.build().fetch_one(pool).await?;
    let count: i64 = row.try_get("count")?;
    Ok(count as u64)
}

pub(crate) fn apply_filters<'a>(
    builder: &mut QueryBuilder<'a, Sqlite>,
    filters: &'a TransactionFilters,
) {
    if let Some(search) = &filters.search {
        builder.push(" AND t.text LIKE ");
        builder.push_bind(format!("%{}%", search));
    }
    if let Some(true) = filters.missing_assignment {
        builder.push(" AND t.assigned_budget_post_id IS NULL AND t.confirmed = 0");
    }
    if let Some(true) = filters.suggested_only {
        builder.push(" AND t.suggested_budget_post_id IS NOT NULL AND t.confirmed = 0");
    }
    if let Some(true) = filters.kontingent_only {
        builder.push(" AND mr.name = 'Kontingent - SE MEDD.'");
    }
    if let Some(rule_ids) = &filters.matched_rule_ids {
        if !rule_ids.is_empty() {
            builder.push(" AND t.matched_rule_id IN (");
            let mut separated = builder.separated(",");
            for id in rule_ids {
                separated.push_bind(id);
            }
            builder.push(")");
        }
    }
    if let Some(direction) = &filters.direction {
        if direction == "income" {
            builder.push(" AND t.amount > 0");
        } else if direction == "expense" {
            builder.push(" AND t.amount < 0");
        }
    }
    if let Some(year) = filters.year {
        builder.push(" AND substr(t.booking_date, 1, 4) = ");
        builder.push_bind(year.to_string());
    }
    if let Some(post_id) = filters.budget_post_id {
        builder.push(" AND t.assigned_budget_post_id = ");
        builder.push_bind(post_id);
    }
}

pub async fn get_progress(pool: &SqlitePool) -> AppResult<ProgressSummary> {
    let row = sqlx::query(
        "SELECT \
            COUNT(*) as total, \
            SUM(CASE WHEN confirmed = 1 THEN 1 ELSE 0 END) as confirmed, \
            SUM(CASE WHEN confirmed = 0 AND suggested_budget_post_id IS NOT NULL THEN 1 ELSE 0 END) \
                as suggested_pending \
         FROM transactions",
    )
    .fetch_one(pool)
    .await?;

    Ok(ProgressSummary {
        total: row.try_get::<i64, _>("total")? as u64,
        confirmed: row.try_get::<i64, _>("confirmed")? as u64,
        suggested_pending: row.try_get::<i64, _>("suggested_pending")? as u64,
    })
}

pub async fn get_reconciliation_summary(pool: &SqlitePool, year: i32) -> AppResult<ReconciliationSummary> {
    let (start_balance, end_balance, movements) = get_balance_summary(pool, year).await?;
    let preview = get_report_preview(pool, year).await?;
    let result = parse_decimal(&preview.result)?;
    let difference = movements - result;

    let rows = sqlx::query(
        "SELECT id, booking_date, text, CAST(amount AS TEXT) as amount \
         FROM transactions \
         WHERE substr(booking_date, 1, 4) = ? AND assigned_budget_post_id IS NULL \
         ORDER BY booking_date, id",
    )
    .bind(year.to_string())
    .fetch_all(pool)
    .await?;

    let mut unassigned = Vec::new();
    for row in rows {
        unassigned.push(ReconciliationItem {
            id: row.try_get("id")?,
            booking_date: row.try_get::<String, _>("booking_date")?,
            text: row.try_get::<String, _>("text")?,
            amount: row.try_get::<String, _>("amount")?,
        });
    }

    Ok(ReconciliationSummary {
        year,
        bank_movements: movements.to_string(),
        result: result.to_string(),
        difference: difference.to_string(),
        unassigned,
    })
}

pub async fn assign_transaction(
    pool: &SqlitePool,
    request: AssignRequest,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE transactions \
         SET assigned_budget_post_id = ?, confirmed = ? \
         WHERE id = ?",
    )
    .bind(request.budget_post_id)
    .bind(request.confirm as i64)
    .bind(request.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn batch_assign(pool: &SqlitePool, request: BatchAssignRequest) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    if request.accept_suggested.unwrap_or(false) {
        let mut builder = QueryBuilder::new(
            "UPDATE transactions SET assigned_budget_post_id = suggested_budget_post_id, confirmed = ",
        );
        builder.push_bind(request.confirm.unwrap_or(false) as i64);
        builder.push(" WHERE id IN (");
        let mut separated = builder.separated(",");
        for id in &request.ids {
            separated.push_bind(id);
        }
        builder.push(") AND suggested_budget_post_id IS NOT NULL");
        builder.build().execute(&mut *tx).await?;
    }

    if let Some(budget_post_id) = request.budget_post_id {
        let mut builder = QueryBuilder::new(
            "UPDATE transactions SET assigned_budget_post_id = ",
        );
        builder.push_bind(budget_post_id);
        builder.push(", confirmed = ");
        builder.push_bind(request.confirm.unwrap_or(false) as i64);
        builder.push(" WHERE id IN (");
        let mut separated = builder.separated(",");
        for id in &request.ids {
            separated.push_bind(id);
        }
        builder.push(")");
        builder.build().execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn list_budget_groups(pool: &SqlitePool) -> AppResult<Vec<BudgetGroup>> {
    let groups = sqlx::query_as::<_, BudgetGroup>(
        "SELECT id, name, sort_order FROM budget_groups ORDER BY sort_order, name",
    )
    .fetch_all(pool)
    .await?;
    Ok(groups)
}

pub async fn create_budget_group(
    pool: &SqlitePool,
    input: BudgetGroupInput,
) -> AppResult<i64> {
    let result = sqlx::query(
        "INSERT INTO budget_groups (name, sort_order) VALUES (?, ?)",
    )
    .bind(input.name)
    .bind(input.sort_order)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_budget_group(
    pool: &SqlitePool,
    group: BudgetGroup,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE budget_groups SET name = ?, sort_order = ? WHERE id = ?",
    )
    .bind(group.name)
    .bind(group.sort_order)
    .bind(group.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_budget_group(pool: &SqlitePool, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM budget_groups WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_budget_posts(pool: &SqlitePool) -> AppResult<Vec<BudgetPost>> {
    let posts = sqlx::query_as::<_, BudgetPost>(
        "SELECT id, group_id, name, sort_order, post_type, note_number FROM budget_posts ORDER BY sort_order, name",
    )
    .fetch_all(pool)
    .await?;
    Ok(posts)
}

pub async fn create_budget_post(pool: &SqlitePool, input: BudgetPostInput) -> AppResult<i64> {
    let result = sqlx::query(
        "INSERT INTO budget_posts (group_id, name, sort_order, post_type, note_number) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(input.group_id)
    .bind(input.name)
    .bind(input.sort_order)
    .bind(input.post_type)
    .bind(input.note_number)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_budget_post(pool: &SqlitePool, post: BudgetPost) -> AppResult<()> {
    sqlx::query(
        "UPDATE budget_posts SET group_id = ?, name = ?, sort_order = ?, post_type = ?, note_number = ? WHERE id = ?",
    )
    .bind(post.group_id)
    .bind(post.name)
    .bind(post.sort_order)
    .bind(post.post_type)
    .bind(post.note_number)
    .bind(post.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_budget_post(pool: &SqlitePool, id: i64) -> AppResult<()> {
    sqlx::query("DELETE FROM budget_posts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_rules(pool: &SqlitePool) -> AppResult<Vec<MatcherRule>> {
    let rules = sqlx::query_as::<_, MatcherRule>(
        "SELECT id, name, regex_pattern, default_budget_post_id, direction, enabled, priority \
         FROM matcher_rules ORDER BY priority, name",
    )
    .fetch_all(pool)
    .await?;
    Ok(rules)
}

pub async fn list_rule_stats(pool: &SqlitePool) -> AppResult<Vec<RuleMatchStat>> {
    let rows = sqlx::query(
        "SELECT mr.id, mr.name, COUNT(t.id) as count, \
            SUM(CASE WHEN t.confirmed = 0 AND t.assigned_budget_post_id IS NULL THEN 1 ELSE 0 END) \
                as open_count \
         FROM matcher_rules mr \
         LEFT JOIN transactions t ON t.matched_rule_id = mr.id \
         GROUP BY mr.id, mr.name \
         HAVING COUNT(t.id) > 0 \
         ORDER BY mr.name",
    )
    .fetch_all(pool)
    .await?;
    let mut stats = Vec::new();
    for row in rows {
        stats.push(RuleMatchStat {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            count: row.try_get::<i64, _>("count")? as u64,
            open_count: row.try_get::<i64, _>("open_count")? as u64,
        });
    }
    Ok(stats)
}

pub async fn create_rule(pool: &SqlitePool, input: MatcherRuleInput) -> AppResult<i64> {
    let result = sqlx::query(
        "INSERT INTO matcher_rules (name, regex_pattern, default_budget_post_id, direction, enabled, priority) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(input.name)
    .bind(input.regex_pattern)
    .bind(input.default_budget_post_id)
    .bind(input.direction)
    .bind(input.enabled as i64)
    .bind(input.priority)
    .execute(pool)
    .await?;
    Ok(result.last_insert_rowid())
}

pub async fn update_rule(pool: &SqlitePool, rule: MatcherRule) -> AppResult<()> {
    sqlx::query(
        "UPDATE matcher_rules \
         SET name = ?, regex_pattern = ?, default_budget_post_id = ?, direction = ?, enabled = ?, priority = ? \
         WHERE id = ?",
    )
    .bind(rule.name)
    .bind(rule.regex_pattern)
    .bind(rule.default_budget_post_id)
    .bind(rule.direction)
    .bind(rule.enabled as i64)
    .bind(rule.priority)
    .bind(rule.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_rule(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE transactions SET matched_rule_id = NULL WHERE matched_rule_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM matcher_rules WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn test_rule(regex_pattern: String, sample_text: String) -> AppResult<Vec<String>> {
    let regex = regex::Regex::new(&regex_pattern)?;
    if let Some(captures) = regex.captures(&sample_text) {
        let mut values = Vec::new();
        for i in 0..captures.len() {
            if let Some(value) = captures.get(i) {
                values.push(value.as_str().to_string());
            }
        }
        Ok(values)
    } else {
        Ok(Vec::new())
    }
}

pub async fn list_years(pool: &SqlitePool) -> AppResult<Vec<i32>> {
    let rows = sqlx::query(
        "SELECT DISTINCT year FROM (
            SELECT CAST(substr(booking_date, 1, 4) AS INTEGER) AS year FROM transactions
            UNION
            SELECT year FROM budget_values
            UNION
            SELECT year FROM settings_year
            UNION
            SELECT CAST(value AS INTEGER) AS year FROM app_settings WHERE key = 'active_year'
        ) ORDER BY year",
    )
    .fetch_all(pool)
    .await?;
    let mut years = Vec::new();
    for row in rows {
        let year: i64 = row.try_get("year")?;
        years.push(year as i32);
    }
    Ok(years)
}

pub async fn get_balance_curve(pool: &SqlitePool, year: i32) -> AppResult<Vec<BalancePoint>> {
    let start = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| AppError::Parse("Invalid year".to_string()))?;
    let end = NaiveDate::from_ymd_opt(year, 12, 31)
        .ok_or_else(|| AppError::Parse("Invalid year".to_string()))?;

    let initial_row = sqlx::query(
        "SELECT CAST(balance AS TEXT) as balance \
         FROM transactions \
         WHERE booking_date < ? \
         ORDER BY booking_date DESC, id DESC \
         LIMIT 1",
    )
    .bind(start.to_string())
    .fetch_optional(pool)
    .await?;

    let initial_balance = if let Some(row) = initial_row {
        let value: String = row.try_get("balance")?;
        parse_decimal(&value)?
    } else {
        let first_row = sqlx::query(
            "SELECT CAST(amount AS TEXT) as amount, CAST(balance AS TEXT) as balance \
             FROM transactions \
             WHERE booking_date BETWEEN ? AND ? \
             ORDER BY booking_date ASC, id ASC \
             LIMIT 1",
        )
        .bind(start.to_string())
        .bind(end.to_string())
        .fetch_optional(pool)
        .await?;
        if let Some(first_row) = first_row {
            let amount_str: String = first_row.try_get("amount")?;
            let balance_str: String = first_row.try_get("balance")?;
            let amount = parse_decimal(&amount_str)?;
            let balance = parse_decimal(&balance_str)?;
            balance - amount
        } else {
            Decimal::new(0, 2)
        }
    };

    let rows = sqlx::query(
        "SELECT booking_date, value_date, CAST(balance AS TEXT) as balance, id \
         FROM transactions \
         WHERE booking_date BETWEEN ? AND ? \
         ORDER BY booking_date ASC, value_date ASC, id ASC",
    )
    .bind(start.to_string())
    .bind(end.to_string())
    .fetch_all(pool)
    .await?;

    let mut parsed = Vec::new();
    for row in rows {
        let date_str: String = row.try_get("booking_date")?;
        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|_| AppError::Parse("Invalid date".to_string()))?;
        let value_date_str: String = row.try_get("value_date")?;
        let value_date = NaiveDate::parse_from_str(&value_date_str, "%Y-%m-%d")
            .map_err(|_| AppError::Parse("Invalid date".to_string()))?;
        let balance_str: String = row.try_get("balance")?;
        let balance = parse_decimal(&balance_str)?;
        let id: i64 = row.try_get("id")?;
        parsed.push((date, value_date, balance, id));
    }

    let curve = build_balance_curve(start, end, initial_balance, parsed);
    Ok(curve
        .into_iter()
        .map(|(date, balance)| BalancePoint {
            date: date.to_string(),
            balance: balance.to_f64().unwrap_or(0.0),
        })
        .collect())
}

pub async fn get_active_year(pool: &SqlitePool) -> AppResult<i32> {
    let row = sqlx::query("SELECT value FROM app_settings WHERE key = 'active_year'")
        .fetch_optional(pool)
        .await?;
    if let Some(row) = row {
        let value: String = row.try_get("value")?;
        if let Ok(year) = value.parse::<i32>() {
            return Ok(year);
        }
    }
    let year = Local::now().year() - 1;
    set_active_year(pool, year).await?;
    Ok(year)
}

pub async fn set_active_year(pool: &SqlitePool, year: i32) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES ('active_year', ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(year.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

async fn ensure_active_year(pool: &SqlitePool) -> AppResult<()> {
    let _ = get_active_year(pool).await?;
    Ok(())
}

pub async fn get_report_preview(pool: &SqlitePool, year: i32) -> AppResult<ReportPreview> {
    let posts = sqlx::query(
        "SELECT bp.id as post_id, bp.name as post_name, bp.sort_order as post_sort, bp.post_type as post_type, bp.note_number as note_number, \
            bp.group_id as group_id, bg.name as group_name, bg.sort_order as group_sort \
         FROM budget_posts bp \
         LEFT JOIN budget_groups bg ON bp.group_id = bg.id \
         ORDER BY bg.sort_order, bp.sort_order",
    )
    .fetch_all(pool)
    .await?;

    let actuals = sqlx::query(
        "SELECT t.assigned_budget_post_id as post_id, CAST(t.amount AS TEXT) as amount \
         FROM transactions t \
         WHERE substr(t.booking_date, 1, 4) = ? AND t.assigned_budget_post_id IS NOT NULL",
    )
    .bind(year.to_string())
    .fetch_all(pool)
    .await?;

    let budgets = sqlx::query(
        "SELECT budget_post_id as post_id, year, amount \
         FROM budget_values \
         WHERE year IN (?, ?)",
    )
    .bind(year)
    .bind(year + 1)
    .fetch_all(pool)
    .await?;


    let mut actual_map: BTreeMap<i64, (Decimal, u64)> = BTreeMap::new();

    for row in actuals {
        let amount_raw: String = row.try_get("amount")?;
        let amount = parse_decimal(&amount_raw)?;
        let post_id: i64 = row.try_get("post_id")?;
        let entry = actual_map.entry(post_id).or_insert((Decimal::new(0, 2), 0));
        entry.0 += amount;
        entry.1 += 1;
    }

    let mut budget_map: BTreeMap<(i64, i32), Option<Decimal>> = BTreeMap::new();
    for row in budgets {
        let post_id: i64 = row.try_get("post_id")?;
        let year_value: i32 = row.try_get("year")?;
        let amount_raw: Option<String> = row.try_get("amount")?;
        let amount = match amount_raw {
            Some(value) if !value.trim().is_empty() => Some(parse_decimal(&value)?),
            _ => None,
        };
        budget_map.insert((post_id, year_value), amount);
    }

    let mut total_income = Decimal::new(0, 2);
    let mut total_expense = Decimal::new(0, 2);
    let mut budget_current_total_income = Decimal::new(0, 2);
    let mut budget_current_total_expense = Decimal::new(0, 2);
    let mut budget_next_total_income = Decimal::new(0, 2);
    let mut budget_next_total_expense = Decimal::new(0, 2);

    let mut grouped_income: BTreeMap<(i64, String, Option<i64>), Vec<(i64, ReportPostSummary)>> =
        BTreeMap::new();
    let mut grouped_expense: BTreeMap<(i64, String, Option<i64>), Vec<(i64, ReportPostSummary)>> =
        BTreeMap::new();

    for row in posts {
        let post_id: i64 = row.try_get("post_id")?;
        let post_name: String = row.try_get("post_name")?;
        let post_sort: i64 = row.try_get("post_sort")?;
        let group_name: Option<String> = row.try_get("group_name")?;
        let group_id: Option<i64> = row.try_get("group_id")?;
        let post_type: Option<String> = row.try_get("post_type")?;
        let note_number: Option<i64> = row.try_get("note_number")?;
        let group_sort: Option<i64> = row.try_get("group_sort")?;
        let group_name = group_name.unwrap_or_else(|| "Uden gruppe".to_string());
        let post_type = post_type.unwrap_or_else(|| "expense".to_string());
        let group_sort = group_sort.unwrap_or(9999);

        let (actual_total, actual_count) = actual_map
            .get(&post_id)
            .cloned()
            .unwrap_or((Decimal::new(0, 2), 0));

        if post_type == "income" {
            total_income += actual_total;
        } else {
            total_expense += actual_total;
        }


        let budget_current = budget_map.get(&(post_id, year)).cloned().unwrap_or(None);
        let budget_next = budget_map
            .get(&(post_id, year + 1))
            .cloned()
            .unwrap_or(None);

        if let Some(value) = budget_current {
            if post_type == "income" {
                budget_current_total_income += value;
            } else {
                budget_current_total_expense += value;
            }
        }
        if let Some(value) = budget_next {
            if post_type == "income" {
                budget_next_total_income += value;
            } else {
                budget_next_total_expense += value;
            }
        }

        let item = ReportPostSummary {
            name: post_name,
            total: actual_total.to_string(),
            count: actual_count,
            budget_current: budget_current.map(|value| value.to_string()),
            budget_next: budget_next.map(|value| value.to_string()),
            post_id,
            editable: true,
            post_type: post_type.clone(),
            note_number,
        };

        let is_income = post_type == "income";

        let target = if is_income {
            grouped_income.entry((group_sort, group_name, group_id)).or_default()
        } else {
            grouped_expense.entry((group_sort, group_name, group_id)).or_default()
        };
        target.push((post_sort, ReportPostSummary { ..item }));
    }

    let mut income_groups = grouped_income
        .into_iter()
        .map(|((_, group_name, group_id), mut posts)| {
            posts.sort_by_key(|(sort, _)| *sort);
            ReportGroupSummary {
                group_id,
                name: group_name,
                posts: posts.into_iter().map(|(_, post)| post).collect(),
            }
        })
        .collect::<Vec<_>>();
    let mut expense_groups = grouped_expense
        .into_iter()
        .map(|((_, group_name, group_id), mut posts)| {
            posts.sort_by_key(|(sort, _)| *sort);
            ReportGroupSummary {
                group_id,
                name: group_name,
                posts: posts.into_iter().map(|(_, post)| post).collect(),
            }
        })
        .collect::<Vec<_>>();

    let (start_balance, end_balance, movements) = get_balance_summary(pool, year).await?;

    Ok(ReportPreview {
        year,
        total_income: total_income.to_string(),
        total_expense: total_expense.to_string(),
        result: (total_income + total_expense).to_string(),
        budget_current_total_income: budget_current_total_income.to_string(),
        budget_current_total_expense: budget_current_total_expense.to_string(),
        budget_current_result: (budget_current_total_income + budget_current_total_expense).to_string(),
        budget_next_total_income: budget_next_total_income.to_string(),
        budget_next_total_expense: budget_next_total_expense.to_string(),
        budget_next_result: (budget_next_total_income + budget_next_total_expense).to_string(),
        income_groups,
        expense_groups,
        balance: BalanceSummary {
            start_balance: start_balance.to_string(),
            movements: movements.to_string(),
            end_balance: end_balance.to_string(),
        },
    })
}

fn parse_decimal(value: &str) -> AppResult<Decimal> {
    Decimal::from_str(value).map_err(|_| AppError::Parse("Invalid decimal".to_string()))
}

pub async fn get_balance_summary(
    pool: &SqlitePool,
    year: i32,
) -> AppResult<(Decimal, Decimal, Decimal)> {
    let start = NaiveDate::from_ymd_opt(year, 1, 1)
        .ok_or_else(|| AppError::Parse("Invalid year".to_string()))?;
    let end = NaiveDate::from_ymd_opt(year, 12, 31)
        .ok_or_else(|| AppError::Parse("Invalid year".to_string()))?;

    let initial_row = sqlx::query(
        "SELECT CAST(balance AS TEXT) as balance \
         FROM transactions \
         WHERE booking_date < ? \
         ORDER BY booking_date DESC, id DESC \
         LIMIT 1",
    )
    .bind(start.to_string())
    .fetch_optional(pool)
    .await?;

    let initial_balance = if let Some(row) = initial_row {
        let value: String = row.try_get("balance")?;
        parse_decimal(&value)?
    } else {
        let first_row = sqlx::query(
            "SELECT CAST(amount AS TEXT) as amount, CAST(balance AS TEXT) as balance \
             FROM transactions \
             WHERE booking_date BETWEEN ? AND ? \
             ORDER BY booking_date ASC, id ASC \
             LIMIT 1",
        )
        .bind(start.to_string())
        .bind(end.to_string())
        .fetch_optional(pool)
        .await?;
        if let Some(first_row) = first_row {
            let amount_str: String = first_row.try_get("amount")?;
            let balance_str: String = first_row.try_get("balance")?;
            let amount = parse_decimal(&amount_str)?;
            let balance = parse_decimal(&balance_str)?;
            balance - amount
        } else {
            Decimal::new(0, 2)
        }
    };

    let rows = sqlx::query(
        "SELECT booking_date, value_date, CAST(balance AS TEXT) as balance, id \
         FROM transactions \
         WHERE booking_date BETWEEN ? AND ? \
         ORDER BY booking_date ASC, value_date ASC, id ASC",
    )
    .bind(start.to_string())
    .bind(end.to_string())
    .fetch_all(pool)
    .await?;

    let mut parsed = Vec::new();
    for row in rows {
        let date_str: String = row.try_get("booking_date")?;
        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|_| AppError::Parse("Invalid date".to_string()))?;
        let value_date_str: String = row.try_get("value_date")?;
        let value_date = NaiveDate::parse_from_str(&value_date_str, "%Y-%m-%d")
            .map_err(|_| AppError::Parse("Invalid date".to_string()))?;
        let balance_str: String = row.try_get("balance")?;
        let balance = parse_decimal(&balance_str)?;
        let id: i64 = row.try_get("id")?;
        parsed.push((date, value_date, balance, id));
    }

    let curve = build_balance_curve(start, end, initial_balance, parsed);
    if curve.is_empty() {
        return Ok((Decimal::new(0, 2), Decimal::new(0, 2), Decimal::new(0, 2)));
    }
    let start_balance = curve.first().map(|(_, value)| *value).unwrap_or(initial_balance);
    let end_balance = curve.last().map(|(_, value)| *value).unwrap_or(initial_balance);
    let movements = end_balance - start_balance;
    Ok((start_balance, end_balance, movements))
}

pub async fn get_settings(pool: &SqlitePool) -> AppResult<SettingsPayload> {
    get_settings_for_year(pool, get_active_year(pool).await?).await
}

pub async fn get_settings_for_year(
    pool: &SqlitePool,
    year: i32,
) -> AppResult<SettingsPayload> {
    let rows = sqlx::query("SELECT key, value FROM settings_year WHERE year = ?")
        .bind(year)
        .fetch_all(pool)
        .await?;
    let mut payload = SettingsPayload {
        chair: None,
        vice_chair: None,
        treasurer: None,
        secretary: None,
        auditor_one: None,
        auditor_two: None,
        board_member_one: None,
        board_member_two: None,
        board_member_three: None,
        board_member_four: None,
        pdf_title_line1: None,
        pdf_title_line2: None,
        signatures_enabled: None,
    };
    for row in rows {
        let key: String = row.try_get("key")?;
        let value: Option<String> = row.try_get("value")?;
        match key.as_str() {
            "chair" => payload.chair = value,
            "vice_chair" => payload.vice_chair = value,
            "treasurer" => payload.treasurer = value,
            "secretary" => payload.secretary = value,
            "auditor_one" => payload.auditor_one = value,
            "auditor_two" => payload.auditor_two = value,
            "board_member_one" => payload.board_member_one = value,
            "board_member_two" => payload.board_member_two = value,
            "board_member_three" => payload.board_member_three = value,
            "board_member_four" => payload.board_member_four = value,
            "pdf_title_line1" => payload.pdf_title_line1 = value,
            "pdf_title_line2" => payload.pdf_title_line2 = value,
            "signatures_enabled" => {
                payload.signatures_enabled = value.as_deref().map(|v| v == "true")
            }
            _ => {}
        }
    }
    Ok(payload)
}

pub async fn save_settings(pool: &SqlitePool, payload: SettingsPayload) -> AppResult<()> {
    save_settings_for_year(pool, get_active_year(pool).await?, payload).await
}

pub async fn save_settings_for_year(
    pool: &SqlitePool,
    year: i32,
    payload: SettingsPayload,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    let entries = [
        ("chair", payload.chair),
        ("vice_chair", payload.vice_chair),
        ("treasurer", payload.treasurer),
        ("secretary", payload.secretary),
        ("auditor_one", payload.auditor_one),
        ("auditor_two", payload.auditor_two),
        ("board_member_one", payload.board_member_one),
        ("board_member_two", payload.board_member_two),
        ("board_member_three", payload.board_member_three),
        ("board_member_four", payload.board_member_four),
        ("pdf_title_line1", payload.pdf_title_line1),
        ("pdf_title_line2", payload.pdf_title_line2),
        (
            "signatures_enabled",
            payload
                .signatures_enabled
                .map(|value| if value { "true".to_string() } else { "false".to_string() }),
        ),
    ];
    for (key, value) in entries {
        sqlx::query(
            "INSERT INTO settings_year (year, key, value) VALUES (?, ?, ?) \
             ON CONFLICT(year, key) DO UPDATE SET value = excluded.value",
        )
        .bind(year)
        .bind(key)
        .bind(value)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn copy_settings_year(
    pool: &SqlitePool,
    from_year: i32,
    to_year: i32,
) -> AppResult<()> {
    let settings = get_settings_for_year(pool, from_year).await?;
    save_settings_for_year(pool, to_year, settings).await?;
    copy_notes_year(pool, from_year, to_year).await?;
    copy_report_categories_year(pool, from_year, to_year).await
}

pub async fn list_notes(pool: &SqlitePool, year: i32) -> AppResult<Vec<Note>> {
    let rows = sqlx::query("SELECT note_number, body FROM notes WHERE year = ? ORDER BY note_number ASC")
        .bind(year)
        .fetch_all(pool)
        .await?;
    let mut notes = Vec::new();
    for row in rows {
        notes.push(Note {
            note_number: row.try_get::<i64, _>("note_number")?,
            body: row.try_get::<Option<String>, _>("body")?.unwrap_or_default(),
        });
    }
    Ok(notes)
}

pub async fn save_notes(pool: &SqlitePool, year: i32, notes: Vec<Note>) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM notes WHERE year = ?")
        .bind(year)
        .execute(&mut *tx)
        .await?;
    for note in notes {
        let body = note.body.trim().to_string();
        if note.note_number < 1 || body.is_empty() {
            continue;
        }
        sqlx::query(
            "INSERT INTO notes (year, note_number, body) VALUES (?, ?, ?) \
             ON CONFLICT(year, note_number) DO UPDATE SET body = excluded.body",
        )
        .bind(year)
        .bind(note.note_number)
        .bind(body)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn save_note_assignments(
    pool: &SqlitePool,
    assignments: Vec<NoteAssignmentInput>,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    for assignment in assignments {
        sqlx::query("UPDATE budget_posts SET note_number = ? WHERE id = ?")
            .bind(assignment.note_number)
            .bind(assignment.post_id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn list_report_categories(
    pool: &SqlitePool,
    year: i32,
) -> AppResult<Vec<ReportCategoryAssignment>> {
    let groups = sqlx::query("SELECT id, name FROM budget_groups ORDER BY sort_order, name")
        .fetch_all(pool)
        .await?;
    let posts = sqlx::query(
        "SELECT group_id, post_type FROM budget_posts WHERE group_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;
    let assignments = sqlx::query(
        "SELECT group_id, category_key FROM report_category_map WHERE year = ?",
    )
    .bind(year)
    .fetch_all(pool)
    .await?;

    let mut mapping = std::collections::HashMap::new();
    for row in assignments {
        let group_id: i64 = row.try_get("group_id")?;
        let category_key: String = row.try_get("category_key")?;
        mapping.insert(group_id, category_key);
    }

    let mut direction_map: std::collections::HashMap<i64, (bool, bool)> = std::collections::HashMap::new();
    for row in posts {
        let group_id: i64 = row.try_get("group_id")?;
        let post_type: Option<String> = row.try_get("post_type")?;
        let entry = direction_map.entry(group_id).or_insert((false, false));
        if post_type.as_deref() == Some("income") {
            entry.0 = true;
        } else {
            entry.1 = true;
        }
    }

    let mut result = Vec::new();
    for row in groups {
        let group_id: i64 = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let category_key = mapping.get(&group_id).cloned().or_else(|| {
            let (has_income, has_expense) = direction_map.get(&group_id).copied().unwrap_or((false, false));
            let direction = if has_income && !has_expense {
                "income"
            } else if has_expense && !has_income {
                "expense"
            } else {
                "expense"
            };
            default_category_for_group(&name, direction)
        });
        result.push(ReportCategoryAssignment {
            group_id,
            category_key,
        });
    }
    Ok(result)
}

pub async fn save_report_categories(
    pool: &SqlitePool,
    year: i32,
    assignments: Vec<ReportCategoryAssignment>,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM report_category_map WHERE year = ?")
        .bind(year)
        .execute(&mut *tx)
        .await?;
    for assignment in assignments {
        let Some(category_key) = assignment.category_key else {
            continue;
        };
        if category_key.trim().is_empty() {
            continue;
        }
        sqlx::query(
            "INSERT INTO report_category_map (year, group_id, category_key) VALUES (?, ?, ?)",
        )
        .bind(year)
        .bind(assignment.group_id)
        .bind(category_key)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn copy_report_categories_year(
    pool: &SqlitePool,
    from_year: i32,
    to_year: i32,
) -> AppResult<()> {
    let items = sqlx::query(
        "SELECT group_id, category_key FROM report_category_map WHERE year = ?",
    )
    .bind(from_year)
    .fetch_all(pool)
    .await?;
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM report_category_map WHERE year = ?")
        .bind(to_year)
        .execute(&mut *tx)
        .await?;
    for row in items {
        let group_id: i64 = row.try_get("group_id")?;
        let category_key: String = row.try_get("category_key")?;
        sqlx::query(
            "INSERT INTO report_category_map (year, group_id, category_key) VALUES (?, ?, ?)",
        )
        .bind(to_year)
        .bind(group_id)
        .bind(category_key)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn default_category_for_group(name: &str, direction: &str) -> Option<String> {
    let key = match name.trim() {
        "Kontingent" | "Vennerne" => "income_membership",
        "Kommunen" => "income_public",
        "Træningssamling" | "Egne stævner" => {
            if direction == "income" {
                "income_events"
            } else {
                "expense_competitions"
            }
        }
        "Sociale arrangementer" => {
            if direction == "income" {
                "income_events"
            } else {
                "expense_social"
            }
        }
        "Sponsor" => "income_sponsors",
        "Diverse" => {
            if direction == "income" {
                "income_other"
            } else {
                "expense_other"
            }
        }
        "Bankgebyrer" | "Bestyrelse og møder" | "Holdsport / systemer" => "expense_admin",
        "Startgebyrer" | "Stævneudgifter (ikke Startgebyrer)" => "expense_competitions",
        "Træningsudstyr" | "Klubtøj" | "Materialer" | "Egen vedligeholdelse af klubben" => {
            "expense_training"
        }
        "Støtte til løftere" | "Kurser" => "expense_support",
        "DVF & DGI" | "Hovedforeningen" => "expense_contributions",
        "Forårstur" => "expense_social",
        "Uforudsete udgifter" => "expense_other",
        _ => {
            if direction == "income" {
                "income_other"
            } else {
                "expense_other"
            }
        }
    };
    Some(key.to_string())
}

pub async fn copy_notes_year(pool: &SqlitePool, from_year: i32, to_year: i32) -> AppResult<()> {
    let notes = list_notes(pool, from_year).await?;
    save_notes(pool, to_year, notes).await
}

pub async fn save_budget_values(
    pool: &SqlitePool,
    year: i32,
    rows: Vec<BudgetValueRowInput>,
) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    for row in rows {
        let budget_current = match row.budget_current {
            Some(value) if !value.trim().is_empty() => Some(parse_danish_decimal(&value)?.to_string()),
            _ => None,
        };
        let budget_next = match row.budget_next {
            Some(value) if !value.trim().is_empty() => Some(parse_danish_decimal(&value)?.to_string()),
            _ => None,
        };

        upsert_budget_value(&mut tx, row.post_id, year, budget_current, None).await?;
        upsert_budget_value(&mut tx, row.post_id, year + 1, budget_next, None).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn reset_data(pool: &SqlitePool) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM transactions").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM imported_files").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM budget_values").execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn upsert_budget_value(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    post_id: i64,
    year: i32,
    amount: Option<String>,
    note: Option<String>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO budget_values (budget_post_id, year, amount, note) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(budget_post_id, year) DO UPDATE SET amount = excluded.amount, note = excluded.note",
    )
    .bind(post_id)
    .bind(year)
    .bind(amount)
    .bind(note)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

impl From<AppError> for String {
    fn from(value: AppError) -> Self {
        value.to_string()
    }
}
