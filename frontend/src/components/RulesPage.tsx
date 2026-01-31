import { useEffect, useState } from "react";
import {
  createRule,
  deleteRule,
  listRules,
  listRuleStats,
  runMatcher,
  testRule,
  updateRule,
  type MatcherRule,
  type RuleMatchStat
} from "../api/tauri";

export default function RulesPage() {
  const [rules, setRules] = useState<MatcherRule[]>([]);
  const [ruleStats, setRuleStats] = useState<RuleMatchStat[]>([]);
  const [name, setName] = useState("");
  const [regexPattern, setRegexPattern] = useState("");
  const [direction, setDirection] = useState("both");
  const [priority, setPriority] = useState(0);
  const [sampleText, setSampleText] = useState("");
  const [testResult, setTestResult] = useState<string>("");
  const [status, setStatus] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editDrafts, setEditDrafts] = useState<
    Record<number, { name: string; regex: string; direction: string; priority: number; enabled: boolean }>
  >({});

  const load = async () => {
    const [data, stats] = await Promise.all([listRules(), listRuleStats()]);
    setRules(data);
    setRuleStats(stats);
  };

  const startEdit = (rule: MatcherRule) => {
    setEditingId(rule.id);
    setEditDrafts((current) => ({
      ...current,
      [rule.id]: {
        name: rule.name,
        regex: rule.regex_pattern,
        direction: rule.direction,
        priority: rule.priority,
        enabled: rule.enabled
      }
    }));
  };

  const updateDraft = (id: number, key: string, value: string | number | boolean) => {
    setEditDrafts((current) => ({
      ...current,
      [id]: {
        ...current[id],
        [key]: value
      }
    }));
  };

  const cancelEdit = () => {
    setEditingId(null);
  };

  const saveEdit = async (rule: MatcherRule) => {
    const draft = editDrafts[rule.id];
    if (!draft) return;
    await updateRule({
      ...rule,
      name: draft.name,
      regex_pattern: draft.regex,
      direction: draft.direction,
      priority: draft.priority,
      enabled: draft.enabled
    });
    setEditingId(null);
    await runMatcher();
    await load();
    setStatus("Regel opdateret.");
  };

  useEffect(() => {
    load();
  }, []);

  const addRule = async () => {
    if (!name || !regexPattern) return;
    await createRule({
      name,
      regex_pattern: regexPattern,
      default_budget_post_id: null,
      direction,
      enabled: true,
      priority
    });
    setName("");
    setRegexPattern("");
    await runMatcher();
    await load();
    setStatus("Regel gemt.");
  };

  const runTest = async () => {
    const result = await testRule(regexPattern, sampleText);
    if (result.matched) {
      setTestResult(`Match: ${result.captures.join(" | ")}`);
    } else {
      setTestResult("Ingen match");
    }
  };

  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>Regler</h2>
          <p>Regex-baserede match-regler til automatisk kontering.</p>
        </div>
      </header>
      <div className="panel-body grid-2">
        <div className="card">
          <h3>Ny regel</h3>
          <div className="stack">
            <input
              type="text"
              placeholder="Navn"
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
            <input
              type="text"
              placeholder="Regex"
              value={regexPattern}
              onChange={(event) => setRegexPattern(event.target.value)}
            />
            <div className="form-row">
              <select value={direction} onChange={(event) => setDirection(event.target.value)}>
                <option value="both">Begge retninger</option>
                <option value="income">Indtaegter</option>
                <option value="expense">Udgifter</option>
              </select>
              <input
                type="number"
                value={priority}
                onChange={(event) => setPriority(Number(event.target.value))}
                placeholder="Priority"
              />
            </div>
            <button type="button" className="primary" onClick={addRule}>
              Gem regel
            </button>
          </div>
          <div className="stack">
            <h4>Test regex</h4>
            <textarea
              rows={3}
              placeholder="Indsæt eksempeltekst"
              value={sampleText}
              onChange={(event) => setSampleText(event.target.value)}
            />
            <button type="button" className="secondary" onClick={runTest}>
              Test
            </button>
            {testResult ? <p className="status">{testResult}</p> : null}
          </div>
        </div>
        <div className="card">
          <h3>Aktive regler</h3>
          <button type="button" className="secondary" onClick={runMatcher}>
            Koer matcher
          </button>
          {status ? <p className="status">{status}</p> : null}
          <ul className="list">
            {rules.map((rule) => (
              <li key={rule.id}>
                {editingId === rule.id ? (
                  <div className="stack">
                    <input
                      type="text"
                      value={editDrafts[rule.id]?.name ?? ""}
                      onChange={(event) => updateDraft(rule.id, "name", event.target.value)}
                    />
                    <input
                      type="text"
                      value={editDrafts[rule.id]?.regex ?? ""}
                      onChange={(event) => updateDraft(rule.id, "regex", event.target.value)}
                    />
                    <div className="form-row">
                      <select
                        value={editDrafts[rule.id]?.direction ?? "both"}
                        onChange={(event) => updateDraft(rule.id, "direction", event.target.value)}
                      >
                        <option value="both">Begge retninger</option>
                        <option value="income">Indtaegter</option>
                        <option value="expense">Udgifter</option>
                      </select>
                      <input
                        type="number"
                        value={editDrafts[rule.id]?.priority ?? 0}
                        onChange={(event) => updateDraft(rule.id, "priority", Number(event.target.value))}
                      />
                      <label className="checkbox-row">
                        <input
                          type="checkbox"
                          checked={editDrafts[rule.id]?.enabled ?? true}
                          onChange={(event) => updateDraft(rule.id, "enabled", event.target.checked)}
                        />
                        <span>Aktiv</span>
                      </label>
                    </div>
                    <div className="button-row">
                      <button type="button" className="primary" onClick={() => saveEdit(rule)}>
                        Gem
                      </button>
                      <button type="button" className="ghost" onClick={cancelEdit}>
                        Annuller
                      </button>
                    </div>
                  </div>
                ) : (
                  <>
                    <span>
                      {rule.name}
                      <small>{rule.regex_pattern}</small>
                      <small>
                        {ruleStats.find((item) => item.id === rule.id)?.count ?? 0} matches
                      </small>
                    </span>
                    <div className="button-row">
                      <button type="button" className="secondary" onClick={() => startEdit(rule)}>
                        Rediger
                      </button>
                      <button
                        type="button"
                        className="ghost"
                        onClick={async () => {
                          try {
                            await deleteRule(rule.id);
                            await runMatcher();
                            await load();
                            setStatus("Regel slettet.");
                          } catch (error) {
                            setStatus(`Kunne ikke slette regel: ${String(error)}`);
                          }
                        }}
                      >
                        Slet
                      </button>
                    </div>
                  </>
                )}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </section>
  );
}
