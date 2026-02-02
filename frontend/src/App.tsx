import { useEffect, useMemo, useRef, useState } from "react";
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

const navItems = [
  { label: "Kontering", children: ["Import", "Budget", "Regler"] },
  { label: "Rapport", children: ["Opsætning", "Afstemning", "Kontobevægelse"] },
  { label: "Administrer" }
] as const;
type NavItem = (typeof navItems)[number];
type ChildPage = NavItem extends { children: readonly (infer Child)[] } ? Child : never;
type Page = NavItem["label"] | ChildPage;

export default function App() {
  const [page, setPage] = useState<Page>("Import");
  const [openMenu, setOpenMenu] = useState<NavItem["label"] | null>(null);
  const navRef = useRef<HTMLElement | null>(null);
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

  useEffect(() => {
    setOpenMenu(null);
  }, [page]);

  useEffect(() => {
    if (!openMenu) return;

    const handlePointer = (event: MouseEvent) => {
      if (!navRef.current) return;
      if (!navRef.current.contains(event.target as Node)) {
        setOpenMenu(null);
      }
    };
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpenMenu(null);
      }
    };

    document.addEventListener("mousedown", handlePointer);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handlePointer);
      document.removeEventListener("keydown", handleKey);
    };
  }, [openMenu]);

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
      case "Kontobevægelse":
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
        <nav className="app-nav" ref={navRef}>
          {navItems.map((item) => {
            if ("children" in item) {
              const isGroupActive =
                page === item.label || item.children.includes(page as (typeof item.children)[number]);
              const isOpen = openMenu === item.label;
              const submenuItems = [item.label, ...item.children];
              return (
                <div className={isOpen ? "nav-group open" : "nav-group"} key={item.label}>
                  <button
                    type="button"
                    className={isGroupActive ? "nav-button nav-parent active" : "nav-button nav-parent"}
                    onClick={() => setOpenMenu((current) => (current === item.label ? null : item.label))}
                    aria-haspopup="menu"
                    aria-expanded={isOpen}
                  >
                    {item.label}
                    <span className="nav-caret" aria-hidden="true">
                      v
                    </span>
                  </button>
                  <div className="nav-menu" role="menu" aria-label={`${item.label} menu`}>
                    {submenuItems.map((child) => (
                      <button
                        key={child}
                        type="button"
                        className={page === child ? "nav-button nav-child active" : "nav-button nav-child"}
                        onClick={() => {
                          setPage(child as Page);
                          setOpenMenu(null);
                        }}
                        role="menuitem"
                      >
                        {child}
                      </button>
                    ))}
                  </div>
                </div>
              );
            }

            return (
              <button
                key={item.label}
                type="button"
                className={page === item.label ? "nav-button active" : "nav-button"}
                onClick={() => {
                  setPage(item.label);
                  setOpenMenu(null);
                }}
              >
                {item.label}
              </button>
            );
          })}
        </nav>
      </header>
      <main className="app-content">{content}</main>
    </div>
  );
}
