import { useState } from "react";

type Props = {
  clubs: string[];
  activeClub?: string | null;
  error?: string | null;
  onSelect: (slug: string) => void;
  onCreate: (slug: string) => void;
  onDelete: (slug: string) => void;
  onRename: (fromSlug: string, toSlug: string) => void;
};

export default function ClubSelector({
  clubs,
  activeClub,
  error,
  onSelect,
  onCreate,
  onDelete,
  onRename
}: Props) {
  const [newSlug, setNewSlug] = useState("");
  const [renameSlug, setRenameSlug] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");

  const startRename = (slug: string) => {
    setRenameSlug(slug);
    setRenameValue("");
  };

  const cancelRename = () => {
    setRenameSlug(null);
    setRenameValue("");
  };

  const submitRename = () => {
    if (!renameSlug) return;
    onRename(renameSlug, renameValue);
    cancelRename();
  };

  const submitCreate = () => {
    onCreate(newSlug);
    setNewSlug("");
  };

  return (
    <div className="panel">
      <div className="panel-header">
        <div>
          <h2>Vælg klub</h2>
          <p className="app-subtitle">Administrer mapper i Application Support.</p>
        </div>
      </div>
      <div className="panel-body">
        {error ? <p className="warning">{error}</p> : null}
        <div className="grid-2">
          <div className="card">
            <h3>Eksisterende klubber</h3>
            {clubs.length === 0 ? (
              <p className="empty">Ingen klubber endnu.</p>
            ) : (
              <ul className="list">
                {clubs.map((club) => (
                  <li key={club}>
                    <div className="stack">
                      <strong>{club}</strong>
                      <small>{activeClub === club ? "Aktiv klub" : "Klar til brug"}</small>
                      {renameSlug === club ? (
                        <div className="form-row">
                          <input
                            value={renameValue}
                            onChange={(event) => setRenameValue(event.target.value)}
                            placeholder="nyt-klubnavn"
                          />
                        </div>
                      ) : null}
                    </div>
                    <div className="button-row">
                      {renameSlug === club ? (
                        <>
                          <button type="button" className="primary" onClick={submitRename}>
                            Gem
                          </button>
                          <button type="button" className="ghost" onClick={cancelRename}>
                            Annuller
                          </button>
                        </>
                      ) : (
                        <>
                          <button type="button" className="primary" onClick={() => onSelect(club)}>
                            Åbn
                          </button>
                          <button type="button" className="secondary" onClick={() => startRename(club)}>
                            Omdøb
                          </button>
                          <button
                            type="button"
                            className="ghost"
                            onClick={() => {
                              if (window.confirm(`Slet klub ${club}?`)) {
                                onDelete(club);
                              }
                            }}
                          >
                            Slet
                          </button>
                        </>
                      )}
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <div className="card">
            <h3>Opret ny klub</h3>
            <div className="stack">
              <p>Brug kun små bogstaver, tal og bindestreger.</p>
              <div className="form-row">
                <input
                  value={newSlug}
                  onChange={(event) => setNewSlug(event.target.value)}
                  placeholder="ny-klub"
                />
                <button type="button" className="primary" onClick={submitCreate}>
                  Opret
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
