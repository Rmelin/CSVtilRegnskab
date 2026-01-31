import { Fragment, useEffect, useRef, useState } from "react";
import {
  createBudgetGroup,
  createBudgetPost,
  deleteBudgetGroup,
  deleteBudgetPost,
  getReportPreview,
  listBudgetGroups,
  listBudgetPosts,
  saveBudgetValues,
  updateBudgetGroup,
  updateBudgetPost,
  type BudgetGroup,
  type BudgetPost,
  type BudgetValueRowInput,
  type ReportPostSummary,
  type ReportPreview
} from "../api/tauri";

type BudgetPageProps = {
  activeYear?: number;
};

export default function BudgetPage({ activeYear }: BudgetPageProps) {
  const [groups, setGroups] = useState<BudgetGroup[]>([]);
  const [posts, setPosts] = useState<BudgetPost[]>([]);
  const [groupName, setGroupName] = useState("");
  const [postName, setPostName] = useState("");
  const [postGroupId, setPostGroupId] = useState<number | "">("");
  const [postType, setPostType] = useState("expense");
  const [year, setYear] = useState<number>(activeYear ?? new Date().getFullYear());
  const [preview, setPreview] = useState<ReportPreview | null>(null);
  const [budgetRows, setBudgetRows] = useState<Record<number, BudgetValueRowInput>>({});
  const [status, setStatus] = useState("");
  const statusTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [groupEdits, setGroupEdits] = useState<Record<number, { name: string; sort: number }>>({});
  const [postEdits, setPostEdits] = useState<
    Record<number, { name: string; sort: number; groupId: number; postType: string }>
  >({});

  const load = async () => {
    try {
      const [groupsData, postsData] = await Promise.all([listBudgetGroups(), listBudgetPosts()]);
      setGroups(groupsData);
      setPosts(postsData);
      await refreshPreview(true);
    } catch {
      setStatusWithTimeout("Kunne ikke indlæse budgetdata.");
    }
  };

  const collectBudgetRows = (data: ReportPreview) => {
    const rows: Record<number, BudgetValueRowInput> = {};
    const collect = (items: ReportPostSummary[]) => {
      items.forEach((item) => {
        if (!item.editable) return;
        rows[item.post_id] = {
          post_id: item.post_id,
          budget_current: item.budget_current ?? "",
          budget_next: item.budget_next ?? ""
        };
      });
    };
    data.income_groups.forEach((group) => collect(group.posts));
    data.expense_groups.forEach((group) => collect(group.posts));
    return rows;
  };

  const refreshPreview = async (preserveEdits: boolean) => {
    try {
      const data = await getReportPreview(year);
      setPreview(data);
      const nextRows = collectBudgetRows(data);
      setBudgetRows((current) => (preserveEdits ? { ...nextRows, ...current } : nextRows));
    } catch {
      setPreview(null);
    }
  };

  const sumCategory = (postsInGroup: ReportPostSummary[]) => {
    return postsInGroup.reduce(
      (acc, post) => {
        const current = budgetRows[post.post_id]?.budget_current ?? "";
        const next = budgetRows[post.post_id]?.budget_next ?? "";
        return {
          actual: acc.actual + parseNumber(post.total),
          current: acc.current + parseNumber(current),
          next: acc.next + parseNumber(next)
        };
      },
      { actual: 0, current: 0, next: 0 }
    );
  };

  useEffect(() => {
    load();
  }, []);

  useEffect(() => {
    refreshPreview(false).catch(() => setPreview(null));
  }, [year]);

  useEffect(() => {
    if (!activeYear) return;
    setYear(activeYear);
  }, [activeYear]);

  const addGroup = async () => {
    if (!groupName) return;
    await createBudgetGroup(groupName, groups.length + 1);
    setGroupName("");
    setStatusWithTimeout("Budgetgruppe oprettet.");
    await load();
  };

  const updateGroupField = (id: number, key: "name" | "sort", value: string) => {
    setGroupEdits((current) => ({
      ...current,
      [id]: {
        name: current[id]?.name ?? groups.find((group) => group.id === id)?.name ?? "",
        sort: current[id]?.sort ?? groups.find((group) => group.id === id)?.sort_order ?? 0,
        [key]: key === "sort" ? Number(value) : value
      }
    }));
  };

  const saveGroup = async (group: BudgetGroup) => {
    const draft = groupEdits[group.id];
    if (!draft) return;
    await updateBudgetGroup({
      ...group,
      name: draft.name,
      sort_order: draft.sort
    });
    setStatusWithTimeout("Budgetgruppe opdateret.");
    setGroupEdits((current) => {
      const next = { ...current };
      delete next[group.id];
      return next;
    });
    await load();
  };

  const addPost = async () => {
    if (!postName) return;
    const groupValue = postGroupId === "" ? null : Number(postGroupId);
    await createBudgetPost(groupValue, postName, posts.length + 1, postType);
    setPostName("");
    setStatusWithTimeout("Budgetpost oprettet.");
    await load();
  };

  const updatePostField = (
    post: BudgetPost,
    key: "name" | "sort" | "groupId" | "postType",
    value: string
  ) => {
    setPostEdits((current) => ({
      ...current,
      [post.id]: {
        name: current[post.id]?.name ?? post.name,
        sort: current[post.id]?.sort ?? post.sort_order,
        groupId:
          current[post.id]?.groupId ?? (post.group_id === null ? -1 : post.group_id ?? -1),
        postType: current[post.id]?.postType ?? post.post_type,
        [key]:
          key === "sort"
            ? Number(value)
            : key === "groupId"
              ? Number(value)
              : value
      }
    }));
  };

  const buildPostPayload = (
    post: BudgetPost,
    overrides?: Partial<{ name: string; sort: number; groupId: number; postType: string }>
  ) => {
    const draft = postEdits[post.id];
    const name = overrides?.name ?? draft?.name ?? post.name;
    const sort = overrides?.sort ?? draft?.sort ?? post.sort_order;
    const groupId =
      overrides?.groupId ?? draft?.groupId ?? (post.group_id === null ? -1 : post.group_id ?? -1);
    const postType = overrides?.postType ?? draft?.postType ?? post.post_type;
    return {
      ...post,
      name,
      sort_order: sort,
      group_id: groupId === -1 ? null : groupId,
      post_type: postType,
      note_number: post.note_number ?? null
    };
  };

  const setStatusWithTimeout = (message: string) => {
    setStatus(message);
    if (statusTimer.current) {
      clearTimeout(statusTimer.current);
    }
    statusTimer.current = setTimeout(() => setStatus(""), 2000);
  };

  const savePost = async (
    post: BudgetPost,
    overrides?: Partial<{ name: string; sort: number; groupId: number; postType: string }>
  ) => {
    const payload = buildPostPayload(post, overrides);
    await updateBudgetPost(payload);
    setStatusWithTimeout("Budgetpost opdateret.");
    setPostEdits((current) => {
      const next = { ...current };
      delete next[post.id];
      return next;
    });
    await load();
  };

  const updateBudgetRow = (postId: number, key: keyof BudgetValueRowInput, value: string) => {
    setBudgetRows((current) => ({
      ...current,
      [postId]: {
        ...current[postId],
        [key]: value
      }
    }));
  };

  const onSaveBudgets = async () => {
    const rows = Object.values(budgetRows);
    await saveBudgetValues(year, rows);
    setStatusWithTimeout("Budgettal gemt.");
    await refreshPreview(false);
  };

  const parseNumber = (value: string) => {
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
    const number = parseNumber(value);
    if (!number && number !== 0) return value;
    return `${number.toLocaleString("da-DK", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    })} Kr.`;
  };

  const sumGroup = (postsInGroup: ReportPostSummary[]) => {
    return postsInGroup.reduce(
      (acc, post) => {
        const current = budgetRows[post.post_id]?.budget_current ?? "";
        const next = budgetRows[post.post_id]?.budget_next ?? "";
        return {
          actual: acc.actual + parseNumber(post.total),
          current: acc.current + parseNumber(current),
          next: acc.next + parseNumber(next)
        };
      },
      { actual: 0, current: 0, next: 0 }
    );
  };

  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>Budget</h2>
          <p>Administrer budgetgrupper og budgetposter.</p>
        </div>
      </header>
      {status ? <p className="status">{status}</p> : null}
      <div className="panel-body grid-2">
        <div className="card">
          <h3>Budgetgrupper</h3>
          <div className="form-row">
            <input
              type="text"
              placeholder="Ny gruppe"
              value={groupName}
              onChange={(event) => setGroupName(event.target.value)}
            />
            <button type="button" className="primary" onClick={addGroup}>
              Tilføj
            </button>
          </div>
          <ul className="list">
            {groups.map((group) => (
              <li key={group.id}>
                <span>
                  <input
                    type="text"
                    value={groupEdits[group.id]?.name ?? group.name}
                    onChange={(event) => updateGroupField(group.id, "name", event.target.value)}
                    onBlur={() => saveGroup(group)}
                  />
                  <small>
                    Sortering
                    <input
                      type="number"
                      value={groupEdits[group.id]?.sort ?? group.sort_order}
                      onChange={(event) => updateGroupField(group.id, "sort", event.target.value)}
                      onBlur={() => saveGroup(group)}
                    />
                  </small>
                </span>
                <div className="button-row">
                  <button type="button" className="secondary" onClick={() => saveGroup(group)}>
                    Gem
                  </button>
                  <button
                    type="button"
                    className="ghost"
                    onClick={async () => {
                      try {
                        await deleteBudgetGroup(group.id);
                        setStatusWithTimeout("Budgetgruppe slettet.");
                        await load();
                      } catch {
                        setStatusWithTimeout("Kunne ikke slette budgetgruppe.");
                      }
                    }}
                  >
                    Slet
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </div>
        <div className="card">
          <h3>Budgetposter</h3>
          <div className="form-row">
            <select
              value={postGroupId}
              onChange={(event) => setPostGroupId(event.target.value as unknown as number | "")}
            >
              <option value="">Ingen gruppe</option>
              {groups.map((group) => (
                <option key={group.id} value={group.id}>
                  {group.name}
                </option>
              ))}
            </select>
            <select value={postType} onChange={(event) => setPostType(event.target.value)}>
              <option value="income">Indtaegt</option>
              <option value="expense">Udgift</option>
            </select>
            <input
              type="text"
              placeholder="Ny budgetpost"
              value={postName}
              onChange={(event) => setPostName(event.target.value)}
            />
            <button type="button" className="primary" onClick={addPost}>
              Tilføj
            </button>
          </div>
          <ul className="list">
            {posts.map((post) => (
              <li key={post.id}>
                <span>
                  <input
                    type="text"
                    value={postEdits[post.id]?.name ?? post.name}
                    onChange={(event) => updatePostField(post, "name", event.target.value)}
                    onBlur={() => savePost(post)}
                  />
                  <small>
                    Sortering
                    <input
                      type="number"
                      value={postEdits[post.id]?.sort ?? post.sort_order}
                      onChange={(event) => updatePostField(post, "sort", event.target.value)}
                      onBlur={() => savePost(post)}
                    />
                  </small>
                </span>
                <div className="button-row">
                  <select
                    value={postEdits[post.id]?.groupId ?? (post.group_id ?? -1)}
                    onChange={async (event) => {
                      updatePostField(post, "groupId", event.target.value);
                      await savePost(post, { groupId: Number(event.target.value) });
                    }}
                  >
                    <option value={-1}>Ingen gruppe</option>
                    {groups.map((group) => (
                      <option key={group.id} value={group.id}>
                        {group.name}
                      </option>
                    ))}
                  </select>
                  <select
                    value={postEdits[post.id]?.postType ?? post.post_type}
                    onChange={async (event) => {
                      updatePostField(post, "postType", event.target.value);
                      await savePost(post, { postType: event.target.value });
                    }}
                  >
                    <option value="income">Indtaegt</option>
                    <option value="expense">Udgift</option>
                  </select>
                  <button
                    type="button"
                    className="ghost"
                    onClick={async () => {
                      try {
                        await deleteBudgetPost(post.id);
                        setStatusWithTimeout("Budgetpost slettet.");
                        await load();
                      } catch {
                        setStatusWithTimeout("Kunne ikke slette budgetpost.");
                      }
                    }}
                  >
                    Slet
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </div>
      </div>
      <div className="panel-body">
        <div className="form-row">
          <input
            type="number"
            value={year}
            onChange={(event) => setYear(Number(event.target.value))}
          />
          <button type="button" className="primary" onClick={onSaveBudgets}>
            Gem budget
          </button>
        </div>
        {preview ? (
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
                    const incomePosts = group.posts.filter(
                      (post) => post.editable && post.post_type === "income"
                    );
                    const subtotal = sumCategory(incomePosts);
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
                        {incomePosts.map((post) => (
                          <tr key={post.post_id}>
                            <td>{post.name}</td>
                            <td className="numeric-cell">{formatKr(post.total)}</td>
                            <td>
                              <input
                                type="text"
                                value={budgetRows[post.post_id]?.budget_current ?? ""}
                                onChange={(event) =>
                                  updateBudgetRow(post.post_id, "budget_current", event.target.value)
                                }
                              />
                            </td>
                            <td>
                              <input
                                type="text"
                                value={budgetRows[post.post_id]?.budget_next ?? ""}
                                onChange={(event) =>
                                  updateBudgetRow(post.post_id, "budget_next", event.target.value)
                                }
                              />
                            </td>
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
                    <td className="numeric-cell">{formatKr(preview.budget_current_total_income)}</td>
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
                    const expensePosts = group.posts.filter(
                      (post) => post.editable && post.post_type === "expense"
                    );
                    const subtotal = sumCategory(expensePosts);
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
                        {expensePosts.map((post) => (
                          <tr key={post.post_id}>
                            <td>{post.name}</td>
                            <td className="numeric-cell">{formatKr(post.total)}</td>
                            <td>
                              <input
                                type="text"
                                value={budgetRows[post.post_id]?.budget_current ?? ""}
                                onChange={(event) =>
                                  updateBudgetRow(post.post_id, "budget_current", event.target.value)
                                }
                              />
                            </td>
                            <td>
                              <input
                                type="text"
                                value={budgetRows[post.post_id]?.budget_next ?? ""}
                                onChange={(event) =>
                                  updateBudgetRow(post.post_id, "budget_next", event.target.value)
                                }
                              />
                            </td>
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
                    <td className="numeric-cell">{formatKr(preview.budget_current_total_expense)}</td>
                    <td className="numeric-cell">{formatKr(preview.budget_next_total_expense)}</td>
                  </tr>
                </tfoot>
              </table>
            </div>
            <div className="card">
              <h4>Totals</h4>
              <div className="meta-row">
                <span>Indtægter i alt</span>
                <span>{formatKr(preview.total_income)}</span>
              </div>
              <div className="meta-row">
                <span>Udgifter i alt</span>
                <span>{formatKr(preview.total_expense)}</span>
              </div>
              <div className="meta-row">
                <span>Årets resultat</span>
                <span>{formatKr(preview.result)}</span>
              </div>
              <div className="meta-row">
                <span>Budget {preview.year}</span>
                <span>{formatKr(preview.budget_current_result)}</span>
              </div>
              <div className="meta-row">
                <span>Budget {preview.year + 1}</span>
                <span>{formatKr(preview.budget_next_result)}</span>
              </div>
            </div>
          </div>
        ) : (
          <p className="empty">Ingen budgetdata endnu.</p>
        )}
      </div>
    </section>
  );
}
