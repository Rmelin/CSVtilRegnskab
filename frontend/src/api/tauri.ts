import { invoke } from "@tauri-apps/api/core";

export type TransactionFilters = {
  search?: string;
  missing_assignment?: boolean;
  suggested_only?: boolean;
  kontingent_only?: boolean;
  direction?: "income" | "expense" | "both";
  year?: number;
  matched_rule_ids?: number[];
  budget_post_id?: number;
};

export type PageRequest = {
  page: number;
  page_size: number;
};

export type TransactionView = {
  id: number;
  booking_date: string;
  value_date: string;
  text: string;
  amount: string;
  balance: string;
  own_reference?: string | null;
  suggested_budget_post_id?: number | null;
  assigned_budget_post_id?: number | null;
  matched_rule_id?: number | null;
  confirmed: boolean;
  budget_group_name?: string | null;
  budget_post_name?: string | null;
  suggested_budget_post_name?: string | null;
  matched_rule_name?: string | null;
  kontingent_member_id?: string | null;
  kontingent_member_name?: string | null;
};

export type PagedTransactions = {
  items: TransactionView[];
  page: number;
  page_size: number;
  total: number;
};

export type ProgressSummary = {
  total: number;
  confirmed: number;
  suggested_pending: number;
};

export type BudgetGroup = {
  id: number;
  name: string;
  sort_order: number;
};

export type BudgetPost = {
  id: number;
  group_id?: number | null;
  name: string;
  sort_order: number;
  post_type: string;
  note_number?: number | null;
};

export type MatcherRule = {
  id: number;
  name: string;
  regex_pattern: string;
  default_budget_post_id?: number | null;
  direction: string;
  enabled: number;
  priority: number;
};

export type ImportSummary = {
  imported: number;
  duplicates: number;
  imported_file_id: number;
};

export type RuleTestResult = {
  matched: boolean;
  captures: string[];
};

export type RuleMatchStat = {
  id: number;
  name: string;
  count: number;
  open_count: number;
};

export type ExportResult = {
  path: string;
};

export type SettingsPayload = {
  chair?: string | null;
  vice_chair?: string | null;
  treasurer?: string | null;
  secretary?: string | null;
  auditor_one?: string | null;
  auditor_two?: string | null;
  board_member_one?: string | null;
  board_member_two?: string | null;
  board_member_three?: string | null;
  board_member_four?: string | null;
  pdf_title_line1?: string | null;
  pdf_title_line2?: string | null;
  signatures_enabled?: boolean | null;
};

export type Note = {
  note_number: number;
  body: string;
};

export type NoteAssignmentInput = {
  post_id: number;
  note_number?: number | null;
};


export type ActiveYear = {
  year: number;
};

export type ReportPostSummary = {
  name: string;
  total: string;
  count: number;
  budget_current?: string | null;
  budget_next?: string | null;
  post_id: number;
  editable: boolean;
  post_type: string;
  note_number?: number | null;
};

export type ReportGroupSummary = {
  group_id?: number | null;
  name: string;
  posts: ReportPostSummary[];
};

export type BalanceSummary = {
  start_balance: string;
  movements: string;
  end_balance: string;
};

export type BalancePoint = {
  date: string;
  balance: number;
};

export type ReconciliationItem = {
  id: number;
  booking_date: string;
  text: string;
  amount: string;
};

export type ReconciliationSummary = {
  year: number;
  bank_movements: string;
  result: string;
  difference: string;
  unassigned: ReconciliationItem[];
};

export type ReportPreview = {
  year: number;
  total_income: string;
  total_expense: string;
  result: string;
  budget_current_total_income: string;
  budget_current_total_expense: string;
  budget_current_result: string;
  budget_next_total_income: string;
  budget_next_total_expense: string;
  budget_next_result: string;
  income_groups: ReportGroupSummary[];
  expense_groups: ReportGroupSummary[];
  balance: BalanceSummary;
};

export type BudgetValueRowInput = {
  post_id: number;
  budget_current?: string | null;
  budget_next?: string | null;
};

export const listClubs = () => invoke<string[]>("list_clubs");
export const createClub = (slug: string) => invoke("create_club", { slug });
export const deleteClub = (slug: string) => invoke("delete_club", { slug });
export const renameClub = (fromSlug: string, toSlug: string) =>
  invoke("rename_club", { from_slug: fromSlug, to_slug: toSlug });
export const setActiveClub = (slug: string) => invoke("set_active_club", { slug });
export const getActiveClub = () => invoke<string | null>("get_active_club");

export const importCsv = (path: string) =>
  invoke<ImportSummary>("import_csv", { path });

export const listTransactions = (filters: TransactionFilters, page: PageRequest) =>
  invoke<PagedTransactions>("list_transactions", { filters, page });

export const getProgress = () => invoke<ProgressSummary>("get_progress");

export const assignTransaction = (id: number, budgetPostId: number | null, confirm: boolean) =>
  invoke("assign_transaction", {
    request: { id, budget_post_id: budgetPostId, confirm }
  });

export const batchAssign = (
  ids: number[],
  budgetPostId?: number | null,
  acceptSuggested?: boolean,
  confirm?: boolean
) =>
  invoke("batch_assign", {
    request: {
      ids,
      budget_post_id: budgetPostId ?? null,
      accept_suggested: acceptSuggested ?? false,
      confirm: confirm ?? false
    }
  });

export const listBudgetGroups = () => invoke<BudgetGroup[]>("list_budget_groups");
export const createBudgetGroup = (name: string, sortOrder: number) =>
  invoke<number>("create_budget_group", {
    input: { name, sort_order: sortOrder }
  });
export const updateBudgetGroup = (group: BudgetGroup) =>
  invoke("update_budget_group", { input: group });
export const deleteBudgetGroup = (id: number) =>
  invoke("delete_budget_group", { id });

export const listBudgetPosts = () => invoke<BudgetPost[]>("list_budget_posts");
export const createBudgetPost = (
  groupId: number | null,
  name: string,
  sortOrder: number,
  postType: string
) =>
  invoke<number>("create_budget_post", {
    input: { group_id: groupId, name, sort_order: sortOrder, post_type: postType }
  });
export const updateBudgetPost = (post: BudgetPost) =>
  invoke("update_budget_post", { input: post });
export const deleteBudgetPost = (id: number) =>
  invoke("delete_budget_post", { id });

export const listRules = () => invoke<MatcherRule[]>("list_rules");
export const listRuleStats = () => invoke<RuleMatchStat[]>("list_rule_stats");
export const createRule = (rule: Omit<MatcherRule, "id" | "enabled"> & { enabled: boolean }) =>
  invoke<number>("create_rule", {
    input: {
      name: rule.name,
      regex_pattern: rule.regex_pattern,
      default_budget_post_id: rule.default_budget_post_id ?? null,
      direction: rule.direction,
      enabled: rule.enabled,
      priority: rule.priority
    }
  });
export const updateRule = (rule: MatcherRule) =>
  invoke("update_rule", { input: rule });
export const deleteRule = (id: number) =>
  invoke("delete_rule", { id });
export const testRule = (regexPattern: string, sampleText: string) =>
  invoke<RuleTestResult>("test_rule", { request: { regex_pattern: regexPattern, sample_text: sampleText } });

export const exportCsv = (year: number) =>
  invoke<ExportResult>("export_csv", { request: { year } });

export const exportReportHtml = (year: number) =>
  invoke<ExportResult>("export_report_html", { request: { year } });

export const exportKonteringCsv = (filters: TransactionFilters) =>
  invoke<ExportResult>("export_kontering_csv", { filters });

export const generatePdf = (year: number) =>
  invoke<ExportResult>("generate_pdf", { request: { year } });

export const listYears = () => invoke<{ years: number[] }>("list_years");
export const runMatcher = () => invoke("run_matcher");
export const getSettings = () => invoke<SettingsPayload>("get_settings");
export const saveSettings = (payload: SettingsPayload) =>
  invoke("save_settings", { payload });
export const getActiveYear = () => invoke<ActiveYear>("get_active_year");
export const setActiveYear = (year: number) => invoke("set_active_year", { payload: { year } });
export const getSettingsForYear = (year: number) =>
  invoke<SettingsPayload>("get_settings_for_year", { payload: { year } });
export const saveSettingsForYear = (year: number, payload: SettingsPayload) =>
  invoke("save_settings_for_year", { year: { year }, payload });
export const copySettingsYear = (fromYear: number, toYear: number) =>
  invoke("copy_settings_year", { from_year: { year: fromYear }, to_year: { year: toYear } });
export const listNotes = (year: number) => invoke<Note[]>("list_notes", { year });
export const saveNotes = (year: number, notes: Note[]) =>
  invoke("save_notes", { year, notes });
export const saveNoteAssignments = (assignments: NoteAssignmentInput[]) =>
  invoke("save_note_assignments", { assignments });
export const getReportPreview = (year: number) =>
  invoke<ReportPreview>("get_report_preview", { request: { year } });
export const saveBudgetValues = (year: number, rows: BudgetValueRowInput[]) =>
  invoke("save_budget_values", { request: { year }, rows });
export const resetData = () => invoke("reset_data");
export const getBalanceCurve = (year: number) =>
  invoke<BalancePoint[]>("get_balance_curve", { year });
export const getReconciliationSummary = (year: number) =>
  invoke<ReconciliationSummary>("get_reconciliation_summary", { year });
