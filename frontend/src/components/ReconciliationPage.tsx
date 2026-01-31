import { useEffect, useState } from "react";
import { getReconciliationSummary, type ReconciliationSummary } from "../api/tauri";

type ReconciliationPageProps = {
  activeYear?: number;
};

export default function ReconciliationPage({ activeYear }: ReconciliationPageProps) {
  const [year, setYear] = useState<number | "">(activeYear ?? "");
  const [data, setData] = useState<ReconciliationSummary | null>(null);
  const [status, setStatus] = useState("");

  useEffect(() => {
    if (!activeYear) return;
    setYear(activeYear);
  }, [activeYear]);

  useEffect(() => {
    if (year === "") {
      setData(null);
      return;
    }
    setStatus("Indlæser...");
    getReconciliationSummary(Number(year))
      .then((result) => {
        setData(result);
        setStatus("");
      })
      .catch(() => {
        setData(null);
        setStatus("Kunne ikke hente afstemning.");
      });
  }, [year]);

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
    return `${number.toLocaleString("da-DK", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    })} Kr.`;
  };

  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>Afstemning</h2>
          <p>Gennemgang af forskelle mellem kontobevægelser og årets resultat.</p>
        </div>
      </header>
      <div className="panel-body">
        <div className="form-row">
          <input
            type="number"
            value={year}
            onChange={(event) => setYear(Number(event.target.value))}
          />
        </div>
        {status ? <p className="status">{status}</p> : null}
        {data ? (
          <div className="stack">
            <div className="grid-2">
              <div className="card">
                <h4>Kontobevægelser</h4>
                <p>{formatKr(data.bank_movements)}</p>
              </div>
              <div className="card">
                <h4>Årets resultat</h4>
                <p>{formatKr(data.result)}</p>
              </div>
              <div className="card">
                <h4>Afvigelse</h4>
                <p>{formatKr(data.difference)}</p>
              </div>
            </div>
            <div className="card">
              <h4>Ikke-konterede transaktioner</h4>
              {data.unassigned.length === 0 ? (
                <p className="empty">Alle transaktioner er konteret.</p>
              ) : (
                <table>
                  <thead>
                    <tr>
                      <th>Dato</th>
                      <th>Tekst</th>
                      <th className="numeric-cell">Beløb</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.unassigned.map((item) => (
                      <tr key={item.id}>
                        <td>{item.booking_date}</td>
                        <td>{item.text}</td>
                        <td className="numeric-cell">{formatKr(item.amount)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </div>
          </div>
        ) : null}
      </div>
    </section>
  );
}
