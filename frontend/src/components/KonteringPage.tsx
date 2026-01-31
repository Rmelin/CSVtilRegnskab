import { useEffect, useMemo, useState } from "react";
import {
  batchAssign,
  createBudgetPost,
  createRule,
  exportKonteringCsv,
  getProgress,
  listBudgetGroups,
  listBudgetPosts,
  listRules,
  listRuleStats,
  runMatcher,
  listTransactions,
  type BudgetGroup,
  type BudgetPost,
  type PagedTransactions,
  type RuleMatchStat,
  type TransactionFilters
} from "../api/tauri";

type KonteringPageProps = {
  activeYear?: number;
};

export default function KonteringPage({ activeYear }: KonteringPageProps) {
  const [filters, setFilters] = useState<TransactionFilters>({
    year: activeYear,
    missing_assignment: true
  });
  const [page, setPage] = useState(0);
  const [data, setData] = useState<PagedTransactions | null>(null);
  const [progress, setProgress] = useState({ total: 0, confirmed: 0, suggested_pending: 0 });
  const [selectedIds, setSelectedIds] = useState<number[]>([]);
  const [budgetPosts, setBudgetPosts] = useState<BudgetPost[]>([]);
  const [budgetGroups, setBudgetGroups] = useState<BudgetGroup[]>([]);
  const [budgetPostId, setBudgetPostId] = useState<number | "">("");
  const [filterBudgetPostId, setFilterBudgetPostId] = useState<number | "">("");
  const [ruleStats, setRuleStats] = useState<RuleMatchStat[]>([]);
  const [selectedRuleIds, setSelectedRuleIds] = useState<number[]>([]);
  const [hideResolvedRules, setHideResolvedRules] = useState(true);
  const [status, setStatus] = useState("");
  const [selectedText, setSelectedText] = useState("");
  const [ruleName, setRuleName] = useState("");
  const [rulePattern, setRulePattern] = useState("");
  const [ruleDirection, setRuleDirection] = useState("both");
  const [rulePriority, setRulePriority] = useState(0);
  const [ruleMode, setRuleMode] = useState("contains");
  const [ruleCaseInsensitive, setRuleCaseInsensitive] = useState(true);
  const [createPost, setCreatePost] = useState(false);
  const [newPostName, setNewPostName] = useState("");
  const [newPostGroupId, setNewPostGroupId] = useState<number | "">("");
  const [newPostType, setNewPostType] = useState("expense");

  const pageSize = 50;

  const load = async () => {
    const result = await listTransactions(filters, { page, page_size: pageSize });
    setData(result);
    const summary = await getProgress();
    setProgress(summary);
  };

  const refreshRuleStats = async () => {
    try {
      const stats = await listRuleStats();
      setRuleStats(stats);
    } catch {
      setRuleStats([]);
    }
  };

  useEffect(() => {
    Promise.all([listBudgetPosts(), listBudgetGroups()])
      .then(([posts, groups]) => {
        setBudgetPosts(posts);
        setBudgetGroups(groups);
      })
      .catch(() => {
        setBudgetPosts([]);
        setBudgetGroups([]);
      });
  }, []);

  useEffect(() => {
    runMatcher()
      .catch(() => {})
      .finally(() => {
        refreshRuleStats();
        load();
      });
  }, []);

  useEffect(() => {
    load();
  }, [filters, page]);

  useEffect(() => {
    if (!activeYear) return;
    setFilters((current) => ({ ...current, year: activeYear }));
    setPage(0);
  }, [activeYear]);

  useEffect(() => {
    setFilters((current) => ({
      ...current,
      matched_rule_ids: selectedRuleIds.length > 0 ? selectedRuleIds : undefined,
      kontingent_only: undefined
    }));
  }, [selectedRuleIds]);

  useEffect(() => {
    setFilters((current) => ({
      ...current,
      budget_post_id: filterBudgetPostId === "" ? undefined : Number(filterBudgetPostId)
    }));
    setPage(0);
  }, [filterBudgetPostId]);

  const toggleSelect = (id: number) => {
    setSelectedIds((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id]
    );
  };

  const markAll = () => {
    if (!data) return;
    setSelectedIds(data.items.map((item) => item.id));
  };

  const toggleRule = (id: number) => {
    setSelectedRuleIds((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id]
    );
  };

  const visibleRuleStats = hideResolvedRules
    ? ruleStats.filter((rule) => rule.open_count > 0)
    : ruleStats;

  const acceptSuggested = async () => {
    if (selectedIds.length === 0) return;
    await batchAssign(selectedIds, null, true, true);
    setSelectedIds([]);
    await load();
  };

  const applyBudgetPost = async () => {
    if (selectedIds.length === 0 || budgetPostId === "") return;
    await batchAssign(selectedIds, Number(budgetPostId), false, true);
    setSelectedIds([]);
    await load();
  };

  const exportCsv = async () => {
    const result = await exportKonteringCsv(filters);
    setStatus(`CSV eksporteret: ${result.path}`);
  };

  const sortedBudgetPosts = useMemo(() => {
    const withGroup = budgetPosts.map((post) => {
      const group = budgetGroups.find((item) => item.id === post.group_id);
      return {
        post,
        groupName: group?.name ?? "Ingen gruppe"
      };
    });
    return withGroup
      .sort((a, b) => {
        const groupCompare = a.groupName.localeCompare(b.groupName);
        if (groupCompare !== 0) return groupCompare;
        return a.post.name.localeCompare(b.post.name);
      })
      .map((item) => item.post);
  }, [budgetPosts, budgetGroups]);

  const escapeRegex = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

  const buildRegex = (value: string, mode: string, caseInsensitive: boolean) => {
    const escaped = escapeRegex(value.trim());
    if (!escaped) return "";
    if (mode === "exact") return `^\\s*${escaped}\\s*$`;
    if (mode === "word") return `.*\\b${escaped}\\b.*`;
    return `.*${escaped}.*`;
  };

  const updateRulePattern = (text: string, mode: string, caseInsensitive: boolean) => {
    let pattern = buildRegex(text, mode, caseInsensitive);
    if (caseInsensitive && pattern && !pattern.startsWith("(?i)")) {
      pattern = `(?i)${pattern}`;
    }
    setRulePattern(pattern);
  };

  const onSelectText = () => {
    const selection = window.getSelection();
    const value = selection?.toString().trim() ?? "";
    if (!value) return;
    const trimmed = value.length > 80 ? `${value.slice(0, 80)}...` : value;
    setSelectedText(trimmed);
    setRuleName(`Regex for "${trimmed}"`);
    updateRulePattern(trimmed, ruleMode, ruleCaseInsensitive);
  };

  const clearRuleDraft = () => {
    setSelectedText("");
    setRuleName("");
    setRulePattern("");
    setCreatePost(false);
    setNewPostName("");
    setNewPostGroupId("");
    setNewPostType("expense");
  };

  const createRegexRule = async () => {
    if (!ruleName || !rulePattern) return;
    let defaultPostId: number | null = null;
    if (createPost && newPostName.trim()) {
      const groupValue = newPostGroupId === "" ? null : Number(newPostGroupId);
      defaultPostId = await createBudgetPost(
        groupValue,
        newPostName.trim(),
        budgetPosts.length + 1,
        newPostType
      );
      const updatedPosts = await listBudgetPosts();
      setBudgetPosts(updatedPosts);
    }
    await createRule({
      name: ruleName,
      regex_pattern: rulePattern,
      default_budget_post_id: defaultPostId,
      direction: ruleDirection,
      enabled: true,
      priority: rulePriority
    });
    await runMatcher();
    await refreshRuleStats();
    await load();
    setStatus(`Regel oprettet: ${ruleName}`);
    clearRuleDraft();
  };

  const progressPercent = progress.total
    ? Math.round((progress.confirmed / progress.total) * 100)
    : 0;

  const parseDecimal = (value: string) => {
    if (!value || value.trim() === "") return 0;
    if (value.includes(",")) {
      const normalized = value.replace(/\./g, "").replace(",", ".");
      const number = Number(normalized);
      return Number.isNaN(number) ? 0 : number;
    }
    const number = Number(value);
    return Number.isNaN(number) ? 0 : number;
  };

  const formatKr = (value: string) => {
    if (!value || value.trim() === "") return "";
    const number = parseDecimal(value);
    return `${number.toLocaleString("da-DK", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    })} Kr.`;
  };

  const hasData = data && data.items.length > 0;

  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>Kontering</h2>
          <p>Match transaktioner til budgetposter og bekræft forslag.</p>
        </div>
        <div className="progress">
          <div className="progress-labels">
            <span>Konteret {progress.confirmed} / {progress.total} ({progressPercent}%)</span>
            <span>Foreslået ikke bekræftet: {progress.suggested_pending}</span>
          </div>
          <div className="progress-bar">
            <div style={{ width: `${progressPercent}%` }} />
          </div>
        </div>
      </header>
      <div className="panel-body">
        <div className="filters">
          <input
            type="text"
            placeholder="Søg i tekst..."
            value={filters.search ?? ""}
            onChange={(event) => setFilters({ ...filters, search: event.target.value })}
          />
          <button
            type="button"
            className={filters.missing_assignment ? "chip active" : "chip"}
            onClick={() =>
              setFilters({ ...filters, missing_assignment: !filters.missing_assignment })
            }
          >
            Mangler kontering
          </button>
          <button
            type="button"
            className={hideResolvedRules ? "chip active" : "chip"}
            onClick={() => setHideResolvedRules((current) => !current)}
          >
            Skjul konterede matches
          </button>
          <select
            value={filters.direction ?? ""}
            onChange={(event) =>
              setFilters({
                ...filters,
                direction: event.target.value ? (event.target.value as "income" | "expense") : undefined
              })
            }
          >
            <option value="">Alle retninger</option>
            <option value="income">Indtaegter</option>
            <option value="expense">Udgifter</option>
          </select>
          <select
            value={filterBudgetPostId}
            onChange={(event) => setFilterBudgetPostId(event.target.value as unknown as number | "")}
          >
            <option value="">Alle budgetposter</option>
            {sortedBudgetPosts.map((post) => {
              const group = budgetGroups.find((group) => group.id === post.group_id);
              const groupName = group?.name ?? "Ingen gruppe";
              const suffix = post.post_type === "income" ? "Indtaegt" : "Udgift";
              return (
                <option key={post.id} value={post.id}>
                  {groupName ? `${groupName} (${suffix}) — ${post.name}` : post.name}
                </option>
              );
            })}
          </select>
        </div>
        {visibleRuleStats.length > 0 ? (
          <div className="filters">
            {visibleRuleStats.map((rule) => (
              <button
                key={rule.id}
                type="button"
                className={selectedRuleIds.includes(rule.id) ? "chip active" : "chip"}
                onClick={() => toggleRule(rule.id)}
              >
                {rule.name} ({rule.count})
              </button>
            ))}
          </div>
        ) : null}

        {selectedText ? (
          <div className="card">
            <h4>Ny regex regel</h4>
            <div className="stack">
              <p className="status">Markeret tekst: {selectedText}</p>
              <input
                type="text"
                placeholder="Navn på regel"
                value={ruleName}
                onChange={(event) => setRuleName(event.target.value)}
              />
              <div className="form-row">
                <select
                  value={ruleMode}
                  onChange={(event) => {
                    setRuleMode(event.target.value);
                    updateRulePattern(selectedText, event.target.value, ruleCaseInsensitive);
                  }}
                >
                  <option value="contains">Indeholder</option>
                  <option value="exact">Præcis match</option>
                  <option value="word">Match ord</option>
                </select>
                <label className="checkbox-row">
                  <input
                    type="checkbox"
                    checked={ruleCaseInsensitive}
                    onChange={(event) => {
                      setRuleCaseInsensitive(event.target.checked);
                      updateRulePattern(selectedText, ruleMode, event.target.checked);
                    }}
                  />
                  <span>Ignorer store/små</span>
                </label>
                <select value={ruleDirection} onChange={(event) => setRuleDirection(event.target.value)}>
                  <option value="both">Begge retninger</option>
                  <option value="income">Indtaegter</option>
                  <option value="expense">Udgifter</option>
                </select>
                <input
                  type="number"
                  value={rulePriority}
                  onChange={(event) => setRulePriority(Number(event.target.value))}
                  placeholder="Priority"
                />
              </div>
              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={createPost}
                  onChange={(event) => setCreatePost(event.target.checked)}
                />
                <span>Opret budgetpost sammen med reglen</span>
              </label>
              {createPost ? (
                <div className="stack">
                  <input
                    type="text"
                    placeholder="Navn på budgetpost"
                    value={newPostName}
                    onChange={(event) => setNewPostName(event.target.value)}
                  />
                  <div className="form-row">
                    <select
                      value={newPostGroupId}
                      onChange={(event) =>
                        setNewPostGroupId(event.target.value as unknown as number | "")
                      }
                    >
                      <option value="">Ingen gruppe</option>
                      {budgetGroups.map((group) => (
                        <option key={group.id} value={group.id}>
                          {group.name}
                        </option>
                      ))}
                    </select>
                    <select value={newPostType} onChange={(event) => setNewPostType(event.target.value)}>
                      <option value="income">Indtaegt</option>
                      <option value="expense">Udgift</option>
                    </select>
                  </div>
                </div>
              ) : null}
              <textarea
                rows={2}
                value={rulePattern}
                onChange={(event) => setRulePattern(event.target.value)}
              />
              <div className="button-row">
                <button type="button" className="primary" onClick={createRegexRule}>
                  Opret regel
                </button>
                <button type="button" className="ghost" onClick={clearRuleDraft}>
                  Annuller
                </button>
              </div>
            </div>
          </div>
        ) : null}

        <div className="batch-actions">
          <button type="button" className="secondary" onClick={markAll}>
            Markér alle i filter
          </button>
          <button type="button" className="secondary" onClick={exportCsv}>
            Eksporter CSV
          </button>
          <div className="dropdown-group">
            <select
              value={budgetPostId}
              onChange={(event) => setBudgetPostId(event.target.value as unknown as number | "")}
            >
              <option value="">Vælg budgetpost</option>
              {sortedBudgetPosts.map((post) => {
                const group = budgetGroups.find((group) => group.id === post.group_id);
                const groupName = group?.name ?? "Ingen gruppe";
                const suffix = post.post_type === "income" ? "Indtaegt" : "Udgift";
                return (
                  <option key={post.id} value={post.id}>
                    {groupName
                      ? `${groupName} (${suffix}) — ${post.name}`
                      : post.name}
                  </option>
                );
              })}
            </select>
            <button type="button" className="primary" onClick={applyBudgetPost}>
              Sæt budgetpost på markerede
            </button>
          </div>
          {budgetPosts.length === 0 ? (
            <p className="empty">Ingen budgetposter endnu. Opret dem under Budget.</p>
          ) : null}
        </div>

        {hasData ? (
          <div className="table-wrap">
            <table className="kontering-table">
              <thead>
                <tr>
                  <th></th>
                  <th>Dato</th>
                  <th>Tekst</th>
                  <th>Beløb</th>
                  <th>Budgetpost</th>
                  <th>Match info</th>
                </tr>
              </thead>
              <tbody>
                {data.items.map((item) => (
                  <tr key={item.id}>
                    <td>
                      <div className="cell-clamp">
                        <input
                          type="checkbox"
                          checked={selectedIds.includes(item.id)}
                          onChange={() => toggleSelect(item.id)}
                        />
                      </div>
                    </td>
                    <td>
                      <div className="cell-clamp">{item.booking_date}</div>
                    </td>
                    <td>
                      <div className="text-cell text-tooltip" onMouseUp={onSelectText}>
                        <div className="cell-clamp">
                          <span>{item.text}</span>
                          {item.kontingent_member_name ? (
                            <small>
                              {item.kontingent_member_id} - {item.kontingent_member_name}
                            </small>
                          ) : null}
                        </div>
                        <div className="tooltip-card">{item.text}</div>
                      </div>
                    </td>
                    <td
                      className={
                        Number(item.amount) < 0
                          ? "negative numeric-cell"
                          : "positive numeric-cell"
                      }
                    >
                      <div className="cell-clamp">{formatKr(item.amount)}</div>
                    </td>
                    <td>
                      <div className="cell-clamp">{item.budget_post_name ?? "-"}</div>
                    </td>
                    <td>
                      <div className="cell-clamp">{item.matched_rule_name ?? "-"}</div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="empty">Ingen transaktioner i dette filter.</p>
        )}

        {status ? <p className="status">{status}</p> : null}

        <div className="pagination">
          <button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>
            Forrige
          </button>
          <span>
            Side {page + 1} / {data ? Math.ceil(data.total / pageSize) : 1}
          </span>
          <button
            type="button"
            disabled={!data || (page + 1) * pageSize >= data.total}
            onClick={() => setPage(page + 1)}
          >
            Næste
          </button>
        </div>
      </div>
    </section>
  );
}
