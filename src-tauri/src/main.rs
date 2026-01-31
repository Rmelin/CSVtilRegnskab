use std::path::PathBuf;

use tauri::State;
use tokio::sync::RwLock;

use forening_regnskab::db;
use forening_regnskab::error::AppResult;
use forening_regnskab::export;
use forening_regnskab::importer;
use forening_regnskab::matcher;
use forening_regnskab::models::{
    ActiveYear, AssignRequest, BatchAssignRequest, BudgetGroup, BudgetGroupInput, BudgetPost,
    BudgetPostInput, BudgetValueRowInput, ExportRequest, ExportResult, ImportSummary,
    ListYearsResult, MatcherRule, MatcherRuleInput, PageRequest, PagedTransactions,
    ProgressSummary, ReportPreview, RuleMatchStat, RuleTestRequest, RuleTestResult,
    SettingsPayload, TransactionFilters, BalancePoint, Note, NoteAssignmentInput,
    ReportCategoryAssignment,
    ReconciliationSummary,
};
use sqlx::SqlitePool;

struct AppState {
    pool: RwLock<Option<SqlitePool>>,
    active_club: RwLock<Option<String>>,
}

impl AppState {
    async fn pool(&self) -> Result<SqlitePool, String> {
        self.pool
            .read()
            .await
            .clone()
            .ok_or_else(|| "Ingen aktiv klub valgt".to_string())
    }
}

#[tauri::command]
async fn import_csv(state: State<'_, AppState>, path: String) -> Result<ImportSummary, String> {
    let pool = state.pool().await?;
    importer::import_csv(&pool, PathBuf::from(path).as_path())
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_transactions(
    state: State<'_, AppState>,
    filters: TransactionFilters,
    page: PageRequest,
) -> Result<PagedTransactions, String> {
    let pool = state.pool().await?;
    db::list_transactions(&pool, filters, page)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_progress(state: State<'_, AppState>) -> Result<ProgressSummary, String> {
    let pool = state.pool().await?;
    db::get_progress(&pool)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_reconciliation_summary(
    state: State<'_, AppState>,
    year: i32,
) -> Result<ReconciliationSummary, String> {
    let pool = state.pool().await?;
    db::get_reconciliation_summary(&pool, year)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn assign_transaction(
    state: State<'_, AppState>,
    request: AssignRequest,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::assign_transaction(&pool, request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn batch_assign(
    state: State<'_, AppState>,
    request: BatchAssignRequest,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::batch_assign(&pool, request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_budget_groups(state: State<'_, AppState>) -> Result<Vec<BudgetGroup>, String> {
    let pool = state.pool().await?;
    db::list_budget_groups(&pool)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn create_budget_group(
    state: State<'_, AppState>,
    input: BudgetGroupInput,
) -> Result<i64, String> {
    let pool = state.pool().await?;
    db::create_budget_group(&pool, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn update_budget_group(
    state: State<'_, AppState>,
    input: BudgetGroup,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::update_budget_group(&pool, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn delete_budget_group(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = state.pool().await?;
    db::delete_budget_group(&pool, id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_budget_posts(state: State<'_, AppState>) -> Result<Vec<BudgetPost>, String> {
    let pool = state.pool().await?;
    db::list_budget_posts(&pool)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn create_budget_post(
    state: State<'_, AppState>,
    input: BudgetPostInput,
) -> Result<i64, String> {
    let pool = state.pool().await?;
    db::create_budget_post(&pool, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn update_budget_post(state: State<'_, AppState>, input: BudgetPost) -> Result<(), String> {
    let pool = state.pool().await?;
    db::update_budget_post(&pool, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn delete_budget_post(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = state.pool().await?;
    db::delete_budget_post(&pool, id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_rules(state: State<'_, AppState>) -> Result<Vec<MatcherRule>, String> {
    let pool = state.pool().await?;
    db::list_rules(&pool)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_rule_stats(state: State<'_, AppState>) -> Result<Vec<RuleMatchStat>, String> {
    let pool = state.pool().await?;
    db::list_rule_stats(&pool)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn create_rule(
    state: State<'_, AppState>,
    input: MatcherRuleInput,
) -> Result<i64, String> {
    let pool = state.pool().await?;
    db::create_rule(&pool, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn update_rule(state: State<'_, AppState>, input: MatcherRule) -> Result<(), String> {
    let pool = state.pool().await?;
    db::update_rule(&pool, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn delete_rule(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let pool = state.pool().await?;
    db::delete_rule(&pool, id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn test_rule(request: RuleTestRequest) -> Result<RuleTestResult, String> {
    let captures = db::test_rule(request.regex_pattern, request.sample_text)
        .await
        .map_err(|err| err.to_string())?;
    Ok(RuleTestResult {
        matched: !captures.is_empty(),
        captures,
    })
}

#[tauri::command]
async fn export_csv(state: State<'_, AppState>, request: ExportRequest) -> Result<ExportResult, String> {
    let pool = state.pool().await?;
    let path = export::export_csv(&pool, request.year)
        .await
        .map_err(|err| err.to_string())?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn export_report_html(state: State<'_, AppState>, request: ExportRequest) -> Result<ExportResult, String> {
    let pool = state.pool().await?;
    let path = export::export_report_html(&pool, request.year)
        .await
        .map_err(|err| err.to_string())?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn generate_pdf(state: State<'_, AppState>, request: ExportRequest) -> Result<ExportResult, String> {
    let pool = state.pool().await?;
    let active_slug = state.active_club.read().await.clone();
    let path = export::export_report_pdf(&pool, request.year, active_slug.as_deref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn export_kontering_csv(
    state: State<'_, AppState>,
    filters: TransactionFilters,
) -> Result<ExportResult, String> {
    let pool = state.pool().await?;
    let path = export::export_kontering_csv(&pool, filters)
        .await
        .map_err(|err| err.to_string())?;
    Ok(ExportResult {
        path: path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn list_years(state: State<'_, AppState>) -> Result<ListYearsResult, String> {
    let pool = state.pool().await?;
    let years = db::list_years(&pool)
        .await
        .map_err(|err| err.to_string())?;
    Ok(ListYearsResult { years })
}

#[tauri::command]
async fn get_report_preview(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ReportPreview, String> {
    let pool = state.pool().await?;
    db::get_report_preview(&pool, request.year)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_balance_curve(
    state: State<'_, AppState>,
    year: i32,
) -> Result<Vec<BalancePoint>, String> {
    let pool = state.pool().await?;
    db::get_balance_curve(&pool, year)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_active_year(state: State<'_, AppState>) -> Result<ActiveYear, String> {
    let pool = state.pool().await?;
    let year = db::get_active_year(&pool)
        .await
        .map_err(|err| err.to_string())?;
    Ok(ActiveYear { year })
}

#[tauri::command]
async fn set_active_year(
    state: State<'_, AppState>,
    payload: ActiveYear,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::set_active_year(&pool, payload.year)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_budget_values(
    state: State<'_, AppState>,
    request: ExportRequest,
    rows: Vec<BudgetValueRowInput>,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::save_budget_values(&pool, request.year, rows)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn reset_data(state: State<'_, AppState>) -> Result<(), String> {
    let pool = state.pool().await?;
    db::reset_data(&pool)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<SettingsPayload, String> {
    let pool = state.pool().await?;
    db::get_settings(&pool)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn get_settings_for_year(
    state: State<'_, AppState>,
    payload: ActiveYear,
) -> Result<SettingsPayload, String> {
    let pool = state.pool().await?;
    db::get_settings_for_year(&pool, payload.year)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_settings(
    state: State<'_, AppState>,
    payload: SettingsPayload,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::save_settings(&pool, payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_settings_for_year(
    state: State<'_, AppState>,
    year: ActiveYear,
    payload: SettingsPayload,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::save_settings_for_year(&pool, year.year, payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_notes(state: State<'_, AppState>, year: i32) -> Result<Vec<Note>, String> {
    let pool = state.pool().await?;
    db::list_notes(&pool, year)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_notes(
    state: State<'_, AppState>,
    year: i32,
    notes: Vec<Note>,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::save_notes(&pool, year, notes)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_note_assignments(
    state: State<'_, AppState>,
    assignments: Vec<NoteAssignmentInput>,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::save_note_assignments(&pool, assignments)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_report_categories(
    state: State<'_, AppState>,
    year: i32,
) -> Result<Vec<ReportCategoryAssignment>, String> {
    let pool = state.pool().await?;
    db::list_report_categories(&pool, year)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_report_categories(
    state: State<'_, AppState>,
    year: i32,
    assignments: Vec<ReportCategoryAssignment>,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::save_report_categories(&pool, year, assignments)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn copy_settings_year(
    state: State<'_, AppState>,
    from_year: ActiveYear,
    to_year: ActiveYear,
) -> Result<(), String> {
    let pool = state.pool().await?;
    db::copy_settings_year(&pool, from_year.year, to_year.year)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn run_matcher(state: State<'_, AppState>) -> Result<(), String> {
    let pool = state.pool().await?;
    matcher::apply_matcher_rules(&pool)
        .await
        .map_err(|err| err.to_string())
}

fn validate_slug(slug: &str) -> Result<String, String> {
    let trimmed = slug.trim();
    if trimmed.is_empty() {
        return Err("Klubnavn mangler".to_string());
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err("Klubnavn må kun indeholde små bogstaver, tal og bindestreger".to_string());
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
async fn list_clubs() -> Result<Vec<String>, String> {
    db::list_clubs().map_err(|err| err.to_string())
}

#[tauri::command]
async fn create_club(slug: String) -> Result<(), String> {
    let slug = validate_slug(&slug)?;
    let db_path = db::club_db_path(&slug)?;
    if db_path.exists() {
        return Err("Klubben findes allerede".to_string());
    }
    let db_path = db::ensure_club_db_path(&slug)?;
    let pool = db::init_pool(&db_path).await.map_err(|err| err.to_string())?;
    drop(pool);
    Ok(())
}

#[tauri::command]
async fn delete_club(state: State<'_, AppState>, slug: String) -> Result<(), String> {
    let slug = validate_slug(&slug)?;
    let active_slug = state.active_club.read().await.clone();
    if active_slug.as_deref() == Some(slug.as_str()) {
        *state.pool.write().await = None;
        *state.active_club.write().await = None;
    }
    db::delete_club(&slug).map_err(|err| err.to_string())
}

#[tauri::command]
async fn rename_club(
    state: State<'_, AppState>,
    from_slug: String,
    to_slug: String,
) -> Result<(), String> {
    let from_slug = validate_slug(&from_slug)?;
    let to_slug = validate_slug(&to_slug)?;
    if from_slug == to_slug {
        return Ok(());
    }
    let active_slug = state.active_club.read().await.clone();
    let is_active = active_slug.as_deref() == Some(from_slug.as_str());
    if is_active {
        *state.pool.write().await = None;
    }
    db::rename_club(&from_slug, &to_slug).map_err(|err| err.to_string())?;
    if is_active {
        let db_path = db::ensure_club_db_path(&to_slug)?;
        let pool = db::init_pool(&db_path).await.map_err(|err| err.to_string())?;
        *state.pool.write().await = Some(pool);
        *state.active_club.write().await = Some(to_slug);
    }
    Ok(())
}

#[tauri::command]
async fn set_active_club(state: State<'_, AppState>, slug: String) -> Result<(), String> {
    let slug = validate_slug(&slug)?;
    let db_path = db::ensure_club_db_path(&slug)?;
    let pool = db::init_pool(&db_path).await.map_err(|err| err.to_string())?;
    *state.pool.write().await = Some(pool);
    *state.active_club.write().await = Some(slug);
    Ok(())
}

#[tauri::command]
async fn get_active_club(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.active_club.read().await.clone())
}

fn setup_state() -> AppResult<AppState> {
    Ok(AppState {
        pool: RwLock::new(None),
        active_club: RwLock::new(None),
    })
}

fn main() {
    let context = tauri::generate_context!();
    let state = setup_state().expect("database init failed");
    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            list_clubs,
            create_club,
            delete_club,
            rename_club,
            set_active_club,
            get_active_club,
            import_csv,
            list_transactions,
            get_progress,
            get_reconciliation_summary,
            assign_transaction,
            batch_assign,
            list_budget_groups,
            create_budget_group,
            update_budget_group,
            delete_budget_group,
            list_budget_posts,
            create_budget_post,
            update_budget_post,
            delete_budget_post,
            list_rules,
            list_rule_stats,
            create_rule,
            update_rule,
            delete_rule,
            test_rule,
            export_csv,
            export_report_html,
            export_kontering_csv,
            generate_pdf,
            list_years,
            get_active_year,
            set_active_year,
            run_matcher,
            get_settings,
            get_settings_for_year,
            save_settings,
            save_settings_for_year,
            copy_settings_year,
            list_notes,
            save_notes,
            save_note_assignments,
            list_report_categories,
            save_report_categories,
            get_report_preview,
            get_balance_curve,
            save_budget_values,
            reset_data
        ])
        .run(context)
        .expect("error while running tauri application");
}
