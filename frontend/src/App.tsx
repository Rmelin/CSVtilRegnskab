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
  "Administrer",
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

  useEffect(() => {
    if (!activeClub) {
      setPage("Administrer");
    }
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

  const adminContent = (
    <>
      <section className="panel">
        <header className="panel-header">
          <div>
            <h2>Administrer</h2>
            <p>Skift tema og administrer klubber.</p>
          </div>
        </header>
        <div className="panel-body">
          <div className="form-row">
            <label className="inline-field">
              Tema
              <select value={theme} onChange={(event) => setTheme(event.target.value as typeof theme)}>
                <option value="light">Lys</option>
                <option value="dark">Mørk (Mocha)</option>
                <option value="system">System</option>
              </select>
            </label>
          </div>
        </div>
      </section>
      <ClubSelector
        clubs={clubs}
        activeClub={activeClub}
        error={clubError}
        onSelect={handleSelectClub}
        onCreate={handleCreateClub}
        onDelete={handleDeleteClub}
        onRename={handleRenameClub}
      />
    </>
  );

  const content = useMemo(() => {
    if (!activeClub && page !== "Administrer") {
      return adminContent;
    }
    switch (page) {
      case "Import":
        return (
          <ImportPage
            activeYear={activeYear ?? undefined}
            onCreateYear={onChangeYear}
          />
        );
      case "Administrer":
        return adminContent;
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
    clubs,
    activeClub,
    clubError,
    handleSelectClub,
    handleCreateClub,
    handleDeleteClub,
    handleRenameClub,
    page,
    activeYear,
    availableYears,
    theme
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
          {activeClub ? (
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
            </>
          ) : null}
        </div>
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
      </header>
      <main className="app-content">{content}</main>
    </div>
  );
}
