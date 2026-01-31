import { Fragment, useEffect, useMemo, useState } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis
} from "recharts";
import {
  exportCsv,
  exportReportHtml,
  generatePdf,
  getBalanceCurve,
  getReportPreview,
  getSettingsForYear,
  listNotes,
  listYears,
  type BalancePoint,
  type Note,
  type ReportPreview,
  type SettingsPayload
} from "../api/tauri";

type ReportPageProps = {
  activeYear?: number;
};

export default function ReportPage({ activeYear }: ReportPageProps) {
  const [years, setYears] = useState<number[]>([]);
  const [year, setYear] = useState<number | "">(activeYear ?? "");
  const [status, setStatus] = useState("");
  const [preview, setPreview] = useState<ReportPreview | null>(null);
  const [notes, setNotes] = useState<Note[]>([]);
  const [settings, setSettings] = useState<SettingsPayload | null>(null);
  const [points, setPoints] = useState<BalancePoint[]>([]);
  const [balanceStatus, setBalanceStatus] = useState("");

  useEffect(() => {
    listYears().then((result) => {
      setYears(result.years);
      if (activeYear) {
        setYear(activeYear);
      } else if (result.years.length > 0) {
        setYear(result.years[result.years.length - 1]);
      }
    });
  }, [activeYear]);

  useEffect(() => {
    if (year === "") {
      setPreview(null);
      setNotes([]);
      setSettings(null);
      setPoints([]);
      return;
    }
    Promise.all([getReportPreview(Number(year)), listNotes(Number(year)), getSettingsForYear(Number(year))])
      .then(([report, noteItems, settingsPayload]) => {
        setPreview(report);
        setNotes(noteItems.sort((a, b) => a.note_number - b.note_number));
        setSettings(settingsPayload);
      })
      .catch(() => {
        setPreview(null);
        setNotes([]);
        setSettings(null);
      });
  }, [year]);

  useEffect(() => {
    if (year === "") return;
    setBalanceStatus("Indlæser saldo...");
    getBalanceCurve(Number(year))
      .then((data) => {
        setPoints(data);
        setBalanceStatus("");
      })
      .catch(() => {
        setPoints([]);
        setBalanceStatus("Kunne ikke hente saldo");
      });
  }, [year]);

  const sumCategory = (posts: ReportPreview["income_groups"][number]["posts"]) => {
    return posts.reduce(
      (acc, post) => {
        const current = post.budget_current ?? "";
        const next = post.budget_next ?? "";
        return {
          actual: acc.actual + parseNumber(post.total),
          current: acc.current + parseNumber(current),
          next: acc.next + parseNumber(next)
        };
      },
      { actual: 0, current: 0, next: 0 }
    );
  };

  const monthLabels = [
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "Maj",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Okt",
    "Nov",
    "Dec"
  ];

  const monthTicks = useMemo(() => {
    return points.filter((point) => point.date.endsWith("-01")).map((point) => point.date);
  }, [points]);

  const formatMonthLabel = (value: string) => {
    const parts = value.split("-");
    if (parts.length !== 3) return value;
    const monthIndex = Number(parts[1]) - 1;
    return monthLabels[monthIndex] ?? value;
  };

  const runExport = async () => {
    if (year === "") return;
    const result = await exportCsv(Number(year));
    setStatus(`CSV eksporteret: ${result.path}`);
  };

  const runPdf = async () => {
    if (year === "") return;
    try {
      const result = await generatePdf(Number(year));
      setStatus(`PDF genereret: ${result.path}`);
    } catch (error) {
      setStatus(`Kunne ikke generere PDF: ${String(error)}`);
    }
  };

  const runHtml = async () => {
    if (year === "") return;
    try {
      const result = await exportReportHtml(Number(year));
      setStatus(`HTML genereret: ${result.path}`);
    } catch (error) {
      setStatus(`Kunne ikke generere HTML: ${String(error)}`);
    }
  };


  const parseNumber = (value: string | number) => {
    if (typeof value === "number") {
      return value;
    }
    if (!value || value.trim() === "") return 0;
    if (value.includes(",")) {
      const normalized = value.replace(/\./g, "").replace(",", ".");
      const number = Number(normalized);
      return Number.isNaN(number) ? 0 : number;
    }
    const number = Number(value);
    return Number.isNaN(number) ? 0 : number;
  };

  const formatKr = (value: string | number) => {
    if (typeof value === "string" && value.trim() === "") return "";
    const number = parseNumber(value);
    if (!number && number !== 0) return value;
    return `${number.toLocaleString("da-DK", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    })} Kr.`;
  };

  const sumGroup = (posts: ReportPreview["income_groups"][number]["posts"]) => {
    return posts.reduce(
      (acc, post) => {
        const current = post.budget_current ?? "";
        const next = post.budget_next ?? "";
        return {
          actual: acc.actual + parseNumber(post.total),
          current: acc.current + parseNumber(current),
          next: acc.next + parseNumber(next)
        };
      },
      { actual: 0, current: 0, next: 0 }
    );
  };


  const totalCards = useMemo(() => {
    if (!preview) return [];
    return [
      { label: "Indtægter", value: formatKr(preview.total_income) },
      { label: "Udgifter", value: formatKr(preview.total_expense) },
      { label: "Resultat", value: formatKr(preview.result) }
    ];
  }, [preview]);


  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>Rapport og eksport</h2>
          <p>Generér CSV og PDF rapporter pr. år.</p>
        </div>
      </header>
      <div className="panel-body">
        <div className="form-row">
          <select value={year} onChange={(event) => setYear(Number(event.target.value))}>
            <option value="">Vælg år</option>
            {years.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
          <button type="button" className="primary" onClick={runExport}>
            Eksportér CSV
          </button>
          <button type="button" className="secondary" onClick={runHtml}>
            Eksportér HTML
          </button>
          <button type="button" className="secondary" onClick={runPdf}>
            Generér PDF
          </button>
        </div>
        {status ? <p className="status">{status}</p> : null}
        {preview ? (
          <div className="stack">
            <h3>Preview</h3>
            <div className="grid-2">
              {totalCards.map((card) => (
                <div key={card.label} className="card">
                  <h4>{card.label}</h4>
                  <p>{card.value}</p>
                </div>
              ))}
            </div>
            <div className="stack">
              <div className="card">
                <h4>Indtægter</h4>
                <table>
                  <thead>
                    <tr>
                      <th>Post</th>
                      <th className="numeric-cell">Regnskab {preview.year}</th>
                      <th className="numeric-cell">Budget {preview.year}</th>
                      <th className="numeric-cell">Budget {preview.year + 1}</th>
                    </tr>
                  </thead>
                  <tbody>
                  {preview.income_groups.map((group) => {
                    const subtotal = sumCategory(group.posts);
                    return (
                      <Fragment key={`${group.name}-block`}>
                        <tr>
                          <td>
                            <strong>{group.name}</strong>
                          </td>
                          <td className="numeric-cell">
                            <strong>{formatKr(subtotal.actual.toString())}</strong>
                          </td>
                            <td className="numeric-cell">
                              <strong>{formatKr(subtotal.current.toString())}</strong>
                            </td>
                            <td className="numeric-cell">
                              <strong>{formatKr(subtotal.next.toString())}</strong>
                            </td>
                          </tr>
                          {group.posts.map((post) => (
                            <tr key={post.post_id}>
                              <td>{post.name}</td>
                              <td
                                className={
                                  post.total.startsWith("-") && post.post_type === "income"
                                    ? "numeric-cell warning"
                                    : "numeric-cell"
                                }
                              >
                                {formatKr(post.total)}
                                {post.note_number ? `(${post.note_number})` : ""}
                              </td>
                              <td className="numeric-cell">
                                {formatKr(post.budget_current ?? "")}
                              </td>
                              <td className="numeric-cell">{formatKr(post.budget_next ?? "")}</td>
                            </tr>
                          ))}
                        </Fragment>
                      );
                    })}
                  </tbody>
                </table>
                <table className="total-table">
                  <tfoot>
                    <tr>
                      <td>I alt</td>
                      <td className="numeric-cell">{formatKr(preview.total_income)}</td>
                      <td className="numeric-cell">
                        {formatKr(preview.budget_current_total_income)}
                      </td>
                      <td className="numeric-cell">{formatKr(preview.budget_next_total_income)}</td>
                    </tr>
                  </tfoot>
                </table>
              </div>
              <div className="card">
                <h4>Udgifter</h4>
                <table>
                  <thead>
                    <tr>
                      <th>Post</th>
                      <th className="numeric-cell">Regnskab {preview.year}</th>
                      <th className="numeric-cell">Budget {preview.year}</th>
                      <th className="numeric-cell">Budget {preview.year + 1}</th>
                    </tr>
                  </thead>
                  <tbody>
                  {preview.expense_groups.map((group) => {
                    const subtotal = sumCategory(group.posts);
                    return (
                      <Fragment key={`${group.name}-block`}>
                        <tr>
                          <td>
                            <strong>{group.name}</strong>
                          </td>
                          <td className="numeric-cell">
                            <strong>{formatKr(subtotal.actual.toString())}</strong>
                          </td>
                            <td className="numeric-cell">
                              <strong>{formatKr(subtotal.current.toString())}</strong>
                            </td>
                            <td className="numeric-cell">
                              <strong>{formatKr(subtotal.next.toString())}</strong>
                            </td>
                          </tr>
                          {group.posts.map((post) => (
                            <tr key={post.post_id}>
                              <td>{post.name}</td>
                              <td className="numeric-cell">
                                {formatKr(post.total)}
                                {post.note_number ? `(${post.note_number})` : ""}
                              </td>
                              <td className="numeric-cell">
                                {formatKr(post.budget_current ?? "")}
                              </td>
                              <td className="numeric-cell">{formatKr(post.budget_next ?? "")}</td>
                            </tr>
                          ))}
                        </Fragment>
                      );
                    })}
                  </tbody>
                </table>
                <table className="total-table">
                  <tfoot>
                    <tr>
                      <td>I alt</td>
                      <td className="numeric-cell">{formatKr(preview.total_expense)}</td>
                      <td className="numeric-cell">
                        {formatKr(preview.budget_current_total_expense)}
                      </td>
                      <td className="numeric-cell">
                        {formatKr(preview.budget_next_total_expense)}
                      </td>
                    </tr>
                  </tfoot>
                </table>
              </div>
            </div>
            <div className="card">
              <table className="balance-table">
                <thead>
                  <tr>
                    <th colSpan={7}>BALANCE PR. 31.12.{preview.year}</th>
                  </tr>
                  <tr>
                    <th>AKTIVER</th>
                    <th></th>
                    <th className="numeric-cell">Kr.</th>
                    <th></th>
                    <th>PASSIVER</th>
                    <th></th>
                    <th className="numeric-cell">Kr.</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td><strong>Bankbeholdning</strong></td>
                    <td>Primo</td>
                    <td className="numeric-cell">{formatKr(preview.balance.start_balance)}</td>
                    <td></td>
                    <td><strong>Egenkapital</strong></td>
                    <td>Primo</td>
                    <td className="numeric-cell">{formatKr(preview.balance.start_balance)}</td>
                  </tr>
                  <tr>
                    <td></td>
                    <td>Bevægelser</td>
                    <td className="numeric-cell">{formatKr(preview.balance.movements)}</td>
                    <td></td>
                    <td></td>
                    <td>Bevægelser</td>
                    <td className="numeric-cell">{formatKr(preview.balance.movements)}</td>
                  </tr>
                  <tr>
                    <td></td>
                    <td>Ultimo</td>
                    <td className="numeric-cell">{formatKr(preview.balance.end_balance)}</td>
                    <td></td>
                    <td></td>
                    <td>Ultimo</td>
                    <td className="numeric-cell">{formatKr(preview.balance.end_balance)}</td>
                  </tr>
                  <tr>
                    <td><strong>Aktiver i alt</strong></td>
                    <td></td>
                    <td className="numeric-cell"><strong>{formatKr(preview.balance.end_balance)}</strong></td>
                    <td></td>
                    <td><strong>Passiver i alt</strong></td>
                    <td></td>
                    <td className="numeric-cell"><strong>{formatKr(preview.balance.end_balance)}</strong></td>
                  </tr>
                </tbody>
              </table>
            </div>
            <div className="card">
              <h4>Kontobevægelser</h4>
              {balanceStatus ? <p className="status">{balanceStatus}</p> : null}
              <div style={{ height: 260 }}>
                <ResponsiveContainer width="100%" height="100%">
                  <LineChart data={points} margin={{ top: 10, right: 20, left: 30, bottom: 0 }}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="date" ticks={monthTicks} tickFormatter={formatMonthLabel} />
                    <YAxis width={110} tickFormatter={(value) => formatKr(Number(value))} tickCount={5} />
                    <Tooltip formatter={(value: number) => formatKr(value)} />
                    <Line type="monotone" dataKey="balance" stroke="#fab387" dot={false} />
                  </LineChart>
                </ResponsiveContainer>
              </div>
            </div>
            <div className="card">
              <h4>Noter</h4>
              {notes.filter((note) => note.body.trim() !== "").length === 0 ? (
                <p className="empty">Ingen noter endnu.</p>
              ) : (
                <div className="stack">
                  {notes
                    .filter((note) => note.body.trim() !== "")
                    .map((note) => (
                    <div key={note.note_number} className="meta-row">
                      <span>Note {note.note_number}</span>
                      <span>{note.body}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
            {settings?.signatures_enabled ?? true ? (
              <div className="card">
                <h4>Signaturer</h4>
                <p>
                  Regnskabet er gennemgået af revisor. Bankkontoen stemmer med regnskabet, og der er
                  ingen bemærkninger.
                </p>
                <table className="balance-table">
                  <tbody>
                    <tr>
                      <td>Formand</td>
                      <td>
                        <div>{settings?.chair ?? ""}</div>
                        <div>____________________</div>
                      </td>
                      <td>Bestyrelsesmedlem</td>
                      <td>
                        <div>{settings?.board_member_one ?? ""}</div>
                        <div>____________________</div>
                      </td>
                      <td>Kasser</td>
                      <td>
                        <div>{settings?.treasurer ?? ""}</div>
                        <div>____________________</div>
                      </td>
                    </tr>
                    <tr>
                      <td>Bestyrelsesmedlem</td>
                      <td>
                        <div>{settings?.board_member_two ?? ""}</div>
                        <div>____________________</div>
                      </td>
                      <td>Bestyrelsesmedlem</td>
                      <td>
                        <div>{settings?.board_member_three ?? ""}</div>
                        <div>____________________</div>
                      </td>
                      <td>Bestyrelsesmedlem</td>
                      <td>
                        <div>{settings?.board_member_four ?? ""}</div>
                        <div>____________________</div>
                      </td>
                    </tr>
                    <tr>
                      <td>Revisor</td>
                      <td>
                        <div>{settings?.auditor_one ?? ""}</div>
                        <div>____________________</div>
                      </td>
                      <td>Revisor</td>
                      <td>
                        <div>{settings?.auditor_two ?? ""}</div>
                        <div>____________________</div>
                      </td>
                      <td></td>
                      <td></td>
                    </tr>
                  </tbody>
                </table>
              </div>
            ) : null}
          </div>
        ) : (
          <p className="empty">Ingen preview tilgaengelig endnu.</p>
        )}
      </div>
    </section>
  );
}
