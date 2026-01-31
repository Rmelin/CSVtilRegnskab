import { useEffect, useState } from "react";
import {
  copySettingsYear,
  getSettingsForYear,
  listNotes,
  listBudgetGroups,
  listBudgetPosts,
  resetData,
  saveNotes,
  saveNoteAssignments,
  saveSettingsForYear,
  type Note,
  type NoteAssignmentInput,
  type BudgetGroup,
  type BudgetPost,
  type SettingsPayload
} from "../api/tauri";

const emptySettings: SettingsPayload = {
  chair: "",
  vice_chair: "",
  treasurer: "",
  secretary: "",
  auditor_one: "",
  auditor_two: "",
  board_member_one: "",
  board_member_two: "",
  board_member_three: "",
  board_member_four: "",
  pdf_title_line1: "",
  pdf_title_line2: "",
  signatures_enabled: true
};

type SettingsPageProps = {
  activeYear?: number;
  availableYears: number[];
};

type NoteState = Note & {
  post_ids: number[];
};

export default function SettingsPage({ activeYear, availableYears }: SettingsPageProps) {
  const [settings, setSettings] = useState<SettingsPayload>(emptySettings);
  const [notes, setNotes] = useState<NoteState[]>([]);
  const [budgetPosts, setBudgetPosts] = useState<BudgetPost[]>([]);
  const [budgetGroups, setBudgetGroups] = useState<BudgetGroup[]>([]);
  const [status, setStatus] = useState("");
  const [year, setYear] = useState<number>(activeYear ?? new Date().getFullYear());
  const [copyFrom, setCopyFrom] = useState<number | "">("");

  useEffect(() => {
    if (!year) return;
    Promise.all([getSettingsForYear(year), listNotes(year)]).then(([data, noteItems]) => {
      setSettings({ ...emptySettings, ...data });
      setNotes(buildNoteState(noteItems, budgetPosts));
    });
  }, [year, budgetPosts]);

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
    if (!activeYear) return;
    setYear(activeYear);
  }, [activeYear]);

  const updateField = (key: keyof SettingsPayload, value: string) => {
    setSettings((current) => ({ ...current, [key]: value }));
  };

  const updateBoolField = (key: keyof SettingsPayload, value: boolean) => {
    setSettings((current) => ({ ...current, [key]: value }));
  };

  const onSave = async () => {
    await saveSettingsForYear(year, settings);
    await saveNotes(
      year,
      notes.map((note) => ({ note_number: note.note_number, body: note.body }))
    );
    await saveNoteAssignments(buildAssignments(notes, budgetPosts));
    setStatus(`Opsætning og noter gemt for ${year}.`);
  };

  const onCopy = async () => {
    if (copyFrom === "") return;
    await copySettingsYear(Number(copyFrom), year);
    const [data, noteItems] = await Promise.all([getSettingsForYear(year), listNotes(year)]);
    setSettings({ ...emptySettings, ...data });
    setNotes(buildNoteState(noteItems, budgetPosts));
    setStatus(`Opsætning kopieret fra ${copyFrom} til ${year}.`);
  };

  const onReset = async () => {
    await resetData();
    setStatus("Data er nulstillet (transaktioner og budgettal er slettet).");
  };

  const buildNoteState = (items: Note[], posts: BudgetPost[]) => {
    const byNumber = new Map<number, number[]>();
    for (const post of posts) {
      if (post.note_number) {
        const list = byNumber.get(post.note_number) ?? [];
        list.push(post.id);
        byNumber.set(post.note_number, list);
      }
    }
    return items
      .map((item) => ({
        ...item,
        post_ids: byNumber.get(item.note_number) ?? []
      }))
      .sort((a, b) => a.note_number - b.note_number);
  };

  const buildAssignments = (items: NoteState[], posts: BudgetPost[]) => {
    const assignmentMap = new Map<number, number | null>();
    for (const post of posts) {
      assignmentMap.set(post.id, null);
    }
    for (const note of items) {
      for (const postId of note.post_ids) {
        assignmentMap.set(postId, note.note_number);
      }
    }
    const assignments: NoteAssignmentInput[] = [];
    for (const [postId, noteNumber] of assignmentMap.entries()) {
      assignments.push({ post_id: postId, note_number: noteNumber });
    }
    return assignments;
  };


  const addNote = () => {
    const next = notes.reduce((max, item) => Math.max(max, item.note_number || 0), 0) + 1;
    setNotes(
      [...notes, { note_number: next, body: "", post_ids: [] }].sort(
        (a, b) => a.note_number - b.note_number
      )
    );
  };

  const updateNoteNumber = (index: number, value: string) => {
    const numberValue = Number(value);
    setNotes((current) => {
      const next = [...current];
      next[index] = {
        ...next[index],
        note_number: Number.isFinite(numberValue) ? numberValue : 0
      };
      return next.sort((a, b) => a.note_number - b.note_number);
    });
  };

  const updateNoteBody = (index: number, value: string) => {
    setNotes((current) => {
      const next = [...current];
      next[index] = { ...next[index], body: value };
      return next;
    });
  };

  const removeNote = (index: number) => {
    setNotes((current) => current.filter((_, idx) => idx !== index));
  };

  const togglePost = (noteIndex: number, postId: number) => {
    setNotes((current) => {
      const next = current.map((note, index) => {
        if (index !== noteIndex) {
          return { ...note, post_ids: note.post_ids.filter((id) => id !== postId) };
        }
        const has = note.post_ids.includes(postId);
        return {
          ...note,
          post_ids: has ? note.post_ids.filter((id) => id !== postId) : [...note.post_ids, postId]
        };
      });
      return next;
    });
  };

  const budgetPostLabel = (post: BudgetPost) => {
    const group = budgetGroups.find((item) => item.id === post.group_id);
    const groupName = group?.name ?? "Ingen gruppe";
    const suffix = post.post_type === "income" ? "Indtaegt" : "Udgift";
    return `${groupName} (${suffix}) — ${post.name}`;
  };

  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>Opsætning</h2>
          <p>Angiv bestyrelse og revisorer til PDF-rapporten.</p>
        </div>
      </header>
      <div className="panel-body">
        <div className="form-row">
          <select value={year} onChange={(event) => setYear(Number(event.target.value))}>
            {availableYears.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
          <select
            value={copyFrom}
            onChange={(event) => setCopyFrom(Number(event.target.value))}
          >
            <option value="">Kopier fra år</option>
            {availableYears.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
          <button type="button" className="secondary" onClick={onCopy}>
            Kopier
          </button>
        </div>
        <div className="grid-2">
          <div className="card">
            <h3>PDF overskrift</h3>
            <div className="stack">
              <input
                type="text"
                placeholder="Titel linje 1"
                value={settings.pdf_title_line1 ?? ""}
                onChange={(event) => updateField("pdf_title_line1", event.target.value)}
              />
              <input
                type="text"
                placeholder="Titel linje 2"
                value={settings.pdf_title_line2 ?? ""}
                onChange={(event) => updateField("pdf_title_line2", event.target.value)}
              />
              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={settings.signatures_enabled ?? true}
                  onChange={(event) => updateBoolField("signatures_enabled", event.target.checked)}
                />
                <span>Vis signaturer i rapport og eksport</span>
              </label>
            </div>
          </div>
          <div className="card">
            <h3>Bestyrelse</h3>
            <div className="stack">
              <input
                type="text"
                placeholder="Formand"
                value={settings.chair ?? ""}
                onChange={(event) => updateField("chair", event.target.value)}
              />
              <input
                type="text"
                placeholder="Bestyrelsesmedlem"
                value={settings.board_member_one ?? ""}
                onChange={(event) => updateField("board_member_one", event.target.value)}
              />
              <input
                type="text"
                placeholder="Kasser"
                value={settings.treasurer ?? ""}
                onChange={(event) => updateField("treasurer", event.target.value)}
              />
              <input
                type="text"
                placeholder="Bestyrelsesmedlem"
                value={settings.board_member_two ?? ""}
                onChange={(event) => updateField("board_member_two", event.target.value)}
              />
              <input
                type="text"
                placeholder="Bestyrelsesmedlem"
                value={settings.board_member_three ?? ""}
                onChange={(event) => updateField("board_member_three", event.target.value)}
              />
              <input
                type="text"
                placeholder="Bestyrelsesmedlem"
                value={settings.board_member_four ?? ""}
                onChange={(event) => updateField("board_member_four", event.target.value)}
              />
            </div>
          </div>
          <div className="card">
            <h3>Noter</h3>
            <div className="stack">
              {notes.length === 0 ? <p className="empty">Ingen noter endnu.</p> : null}
              {notes.map((note, index) => (
                <div key={`${note.note_number}-${index}`} className="stack">
                  <div className="form-row">
                    <input
                      type="number"
                      min={1}
                      placeholder="Note #"
                      value={note.note_number || ""}
                      onChange={(event) => updateNoteNumber(index, event.target.value)}
                    />
                    <button type="button" className="secondary" onClick={() => removeNote(index)}>
                      Fjern note
                    </button>
                  </div>
                  <textarea
                    rows={3}
                    placeholder={`Note ${note.note_number || ""}`}
                    value={note.body}
                    onChange={(event) => updateNoteBody(index, event.target.value)}
                  />
                  <div className="stack">
                    {budgetPosts.length === 0 ? (
                      <p className="empty">Ingen budgetposter endnu.</p>
                    ) : (
                      budgetPosts.map((post) => (
                        <label key={post.id} className="checkbox-row">
                          <input
                            type="checkbox"
                            checked={note.post_ids.includes(post.id)}
                            onChange={() => togglePost(index, post.id)}
                          />
                          <span>{budgetPostLabel(post)}</span>
                        </label>
                      ))
                    )}
                  </div>
                </div>
              ))}
              <button type="button" className="secondary" onClick={addNote}>
                Tilføj note
              </button>
            </div>
          </div>
          <div className="card">
            <h3>Revisorer</h3>
            <div className="stack">
              <input
                type="text"
                placeholder="Revisor 1"
                value={settings.auditor_one ?? ""}
                onChange={(event) => updateField("auditor_one", event.target.value)}
              />
              <input
                type="text"
                placeholder="Revisor 2"
                value={settings.auditor_two ?? ""}
                onChange={(event) => updateField("auditor_two", event.target.value)}
              />
            </div>
          </div>
        </div>
        <button type="button" className="primary" onClick={onSave}>
          Gem
        </button>
        <button type="button" className="secondary" onClick={onReset}>
          Nulstil data
        </button>
        {status ? <p className="status">{status}</p> : null}
      </div>
    </section>
  );
}
