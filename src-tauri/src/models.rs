use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportSummary {
    pub imported: u64,
    pub duplicates: u64,
    pub imported_file_id: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionFilters {
    pub search: Option<String>,
    pub missing_assignment: Option<bool>,
    pub suggested_only: Option<bool>,
    pub kontingent_only: Option<bool>,
    pub direction: Option<String>,
    pub year: Option<i32>,
    pub matched_rule_ids: Option<Vec<i64>>,
    pub budget_post_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageRequest {
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactionView {
    pub id: i64,
    pub booking_date: NaiveDate,
    pub value_date: NaiveDate,
    pub text: String,
    pub amount: Decimal,
    pub balance: Decimal,
    pub own_reference: Option<String>,
    pub suggested_budget_post_id: Option<i64>,
    pub assigned_budget_post_id: Option<i64>,
    pub matched_rule_id: Option<i64>,
    pub confirmed: bool,
    pub budget_group_name: Option<String>,
    pub budget_post_name: Option<String>,
    pub suggested_budget_post_name: Option<String>,
    pub matched_rule_name: Option<String>,
    pub kontingent_member_id: Option<String>,
    pub kontingent_member_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PagedTransactions {
    pub items: Vec<TransactionView>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProgressSummary {
    pub total: u64,
    pub confirmed: u64,
    pub suggested_pending: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReconciliationItem {
    pub id: i64,
    pub booking_date: String,
    pub text: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReconciliationSummary {
    pub year: i32,
    pub bank_movements: String,
    pub result: String,
    pub difference: String,
    pub unassigned: Vec<ReconciliationItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssignRequest {
    pub id: i64,
    pub budget_post_id: Option<i64>,
    pub confirm: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchAssignRequest {
    pub ids: Vec<i64>,
    pub budget_post_id: Option<i64>,
    pub accept_suggested: Option<bool>,
    pub confirm: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct BudgetGroup {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetGroupInput {
    pub name: String,
    pub sort_order: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct BudgetPost {
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: String,
    pub sort_order: i64,
    pub post_type: String,
    pub note_number: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetPostInput {
    pub group_id: Option<i64>,
    pub name: String,
    pub sort_order: i64,
    pub post_type: String,
    pub note_number: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct MatcherRule {
    pub id: i64,
    pub name: String,
    pub regex_pattern: String,
    pub default_budget_post_id: Option<i64>,
    pub direction: String,
    pub enabled: i64,
    pub priority: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatcherRuleInput {
    pub name: String,
    pub regex_pattern: String,
    pub default_budget_post_id: Option<i64>,
    pub direction: String,
    pub enabled: bool,
    pub priority: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleTestRequest {
    pub regex_pattern: String,
    pub sample_text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleTestResult {
    pub matched: bool,
    pub captures: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuleMatchStat {
    pub id: i64,
    pub name: String,
    pub count: u64,
    pub open_count: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BalancePoint {
    pub date: String,
    pub balance: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportRequest {
    pub year: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportResult {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListYearsResult {
    pub years: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActiveYear {
    pub year: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsPayload {
    pub chair: Option<String>,
    pub vice_chair: Option<String>,
    pub treasurer: Option<String>,
    pub secretary: Option<String>,
    pub auditor_one: Option<String>,
    pub auditor_two: Option<String>,
    pub board_member_one: Option<String>,
    pub board_member_two: Option<String>,
    pub board_member_three: Option<String>,
    pub board_member_four: Option<String>,
    pub pdf_title_line1: Option<String>,
    pub pdf_title_line2: Option<String>,
    pub signatures_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub note_number: i64,
    pub body: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteAssignmentInput {
    pub post_id: i64,
    pub note_number: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportCategoryAssignment {
    pub group_id: i64,
    pub category_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportPostSummary {
    pub name: String,
    pub total: String,
    pub count: u64,
    pub budget_current: Option<String>,
    pub budget_next: Option<String>,
    pub post_id: i64,
    pub editable: bool,
    pub post_type: String,
    pub note_number: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportGroupSummary {
    pub group_id: Option<i64>,
    pub name: String,
    pub posts: Vec<ReportPostSummary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BalanceSummary {
    pub start_balance: String,
    pub movements: String,
    pub end_balance: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportPreview {
    pub year: i32,
    pub total_income: String,
    pub total_expense: String,
    pub result: String,
    pub budget_current_total_income: String,
    pub budget_current_total_expense: String,
    pub budget_current_result: String,
    pub budget_next_total_income: String,
    pub budget_next_total_expense: String,
    pub budget_next_result: String,
    pub income_groups: Vec<ReportGroupSummary>,
    pub expense_groups: Vec<ReportGroupSummary>,
    pub balance: BalanceSummary,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BudgetValueRowInput {
    pub post_id: i64,
    pub budget_current: Option<String>,
    pub budget_next: Option<String>,
}
