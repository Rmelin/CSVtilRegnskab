import { useEffect, useMemo, useState } from "react";
import ImportPage from "./components/ImportPage";
import KonteringPage from "./components/KonteringPage";
import BudgetPage from "./components/BudgetPage";
import RulesPage from "./components/RulesPage";
import ReportPage from "./components/ReportPage";
import ReconciliationPage from "./components/ReconciliationPage";
import SettingsPage from "./components/SettingsPage";
import BalancePage from "./components/BalancePage";
import ClubSelector from "./components/ClubSelector";
import {
  createClub,
  deleteClub,
  getActiveClub,
  getActiveYear,
  listClubs,
  listYears,
  renameClub,
  setActiveClub,
  setActiveYear
} from "./api/tauri";

const pages = [
  "Import",
  "Kontering",
  "Budget",
  "Regler",
  "Rapport",
  "Afstemning",
  "Saldo-graf",
  "Opsætning"
] as const;
type Page = (typeof pages)[number];

export default function App() {
  const [page, setPage] = useState<Page>("Import");
  const [activeClub, setActiveClubState] = useState<string | null>(null);
  const [clubs, setClubs] = useState<string[]>([]);
  const [clubSelectorOpen, setClubSelectorOpen] = useState(false);
  const [clubError, setClubError] = useState<string | null>(null);
  const [activeYear, setActiveYearState] = useState<number | null>(null);
  const [availableYears, setAvailableYears] = useState<number[]>([]);
  const [theme, setTheme] = useState<"light" | "dark" | "system">(() => {
    const stored = localStorage.getItem("theme");
    if (stored === "light" || stored === "dark" || stored === "system") {
      return stored;
    }
    return "light";
  });

  useEffect(() => {
    Promise.all([listClubs(), getActiveClub()]).then(([list, active]) => {
      setClubs(list);
      setActiveClubState(active ?? null);
      setClubSelectorOpen(active == null);
    });
  }, []);

  useEffect(() => {
    if (!activeClub) return;
    setActiveYearState(null);
    setAvailableYears([]);
    Promise.all([getActiveYear(), listYears()]).then(([active, years]) => {
      setActiveYearState(active.year);
      const merged = new Set([...years.years, active.year]);
      setAvailableYears(Array.from(merged).sort());
    });
  }, [activeClub]);

  const applyTheme = (value: "light" | "dark" | "system") => {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const resolved = value === "system" ? (prefersDark ? "dark" : "light") : value;
    document.documentElement.dataset.theme = resolved;
  };

  useEffect(() => {
    localStorage.setItem("theme", theme);
    applyTheme(theme);
  }, [theme]);

  useEffect(() => {
    if (theme !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => applyTheme("system");
    if (media.addEventListener) {
      media.addEventListener("change", handler);
      return () => media.removeEventListener("change", handler);
    }
    media.addListener(handler);
    return () => media.removeListener(handler);
  }, [theme]);

  const onChangeYear = async (value: number) => {
    await setActiveYear(value);
    setActiveYearState(value);
    const years = await listYears();
    const merged = new Set([...years.years, value]);
    setAvailableYears(Array.from(merged).sort());
  };
  const refreshClubs = async () => {
    const list = await listClubs();
    setClubs(list);
  };

  const activateClub = async (slug: string) => {
    await setActiveClub(slug);
    setActiveClubState(slug);
    setClubSelectorOpen(false);
  };

  const handleSelectClub = async (slug: string) => {
    try {
      setClubError(null);
      await activateClub(slug);
    } catch (error) {
      setClubError(String(error));
    }
  };

  const handleCreateClub = async (slug: string) => {
    try {
      setClubError(null);
      await createClub(slug);
      await refreshClubs();
      await activateClub(slug);
    } catch (error) {
      setClubError(String(error));
    }
  };

  const handleDeleteClub = async (slug: string) => {
    try {
      setClubError(null);
      await deleteClub(slug);
      await refreshClubs();
      if (slug === activeClub) {
        setActiveClubState(null);
        setClubSelectorOpen(true);
      }
    } catch (error) {
      setClubError(String(error));
    }
  };

  const handleRenameClub = async (fromSlug: string, toSlug: string) => {
    try {
      setClubError(null);
      await renameClub(fromSlug, toSlug);
      await refreshClubs();
      if (fromSlug === activeClub) {
        setActiveClubState(toSlug);
      }
    } catch (error) {
      setClubError(String(error));
    }
  };

  const showClubSelector = !activeClub || clubSelectorOpen;

  const content = useMemo(() => {
    if (showClubSelector) {
      return (
        <ClubSelector
          clubs={clubs}
          activeClub={activeClub}
          error={clubError}
          onSelect={handleSelectClub}
          onCreate={handleCreateClub}
          onDelete={handleDeleteClub}
          onRename={handleRenameClub}
        />
      );
    }
    switch (page) {
      case "Import":
        return <ImportPage activeYear={activeYear ?? undefined} />;
      case "Kontering":
        return <KonteringPage activeYear={activeYear ?? undefined} />;
      case "Budget":
        return <BudgetPage activeYear={activeYear ?? undefined} />;
      case "Regler":
        return <RulesPage />;
      case "Rapport":
        return <ReportPage activeYear={activeYear ?? undefined} />;
      case "Afstemning":
        return <ReconciliationPage activeYear={activeYear ?? undefined} />;
      case "Saldo-graf":
        return <BalancePage activeYear={activeYear ?? undefined} />;
      case "Opsætning":
        return (
          <SettingsPage
            activeYear={activeYear ?? undefined}
            availableYears={availableYears}
          />
        );
      default:
        return null;
    }
  }, [
    showClubSelector,
    clubs,
    activeClub,
    clubError,
    handleSelectClub,
    handleCreateClub,
    handleDeleteClub,
    handleRenameClub,
    page,
    activeYear,
    availableYears
  ]);

  return (
    <div className="app-shell">
      <header className="app-header">
        <div>
          <p className="app-title">
            Foreningsregnskab{activeClub ? ` · ${activeClub}` : ""}{activeYear ? ` · ${activeYear}` : ""}
          </p>
          <p className="app-subtitle">Bankimport, kontering og rapportering</p>
        </div>
        <div className="year-select">
          <label>
            Tema
            <select value={theme} onChange={(event) => setTheme(event.target.value as typeof theme)}>
              <option value="light">Lys</option>
              <option value="dark">Mørk (Mocha)</option>
              <option value="system">System</option>
            </select>
          </label>
          {activeClub ? (
            <label>
              Klub
              <div className="form-row">
                <select value={activeClub} onChange={(event) => handleSelectClub(event.target.value)}>
                  {clubs.map((club) => (
                    <option key={club} value={club}>
                      {club}
                    </option>
                  ))}
                </select>
                <button type="button" className="ghost" onClick={() => setClubSelectorOpen(true)}>
                  Administrer
                </button>
              </div>
            </label>
          ) : null}
          {activeClub && !showClubSelector ? (
            <>
              <label>
                Arbejdsaar
                <select
                  value={activeYear ?? ""}
                  onChange={(event) => onChangeYear(Number(event.target.value))}
                >
                  <option value="" disabled>
                    Vælg år
                  </option>
                  {availableYears.map((year) => (
                    <option key={year} value={year}>
                      {year}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Opret år
                <input
                  type="number"
                  value={activeYear ?? ""}
                  onChange={(event) => setActiveYearState(Number(event.target.value))}
                  onBlur={(event) => onChangeYear(Number(event.target.value))}
                />
              </label>
            </>
          ) : null}
        </div>
        {!showClubSelector ? (
          <nav className="app-nav">
            {pages.map((label) => (
              <button
                key={label}
                type="button"
                className={page === label ? "nav-button active" : "nav-button"}
                onClick={() => setPage(label)}
              >
                {label}
              </button>
            ))}
          </nav>
        ) : null}
      </header>
      <main className="app-content">{content}</main>
    </div>
  );
}
