import { useEffect, useMemo, useState } from "react";
import {
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis
} from "recharts";
import { getBalanceCurve, type BalancePoint } from "../api/tauri";

type BalancePageProps = {
  activeYear?: number;
};

export default function BalancePage({ activeYear }: BalancePageProps) {
  const [year, setYear] = useState<number>(activeYear ?? new Date().getFullYear() - 1);
  const [points, setPoints] = useState<BalancePoint[]>([]);
  const [status, setStatus] = useState("");

  useEffect(() => {
    if (!activeYear) return;
    setYear(activeYear);
  }, [activeYear]);

  const load = async () => {
    setStatus("Indlæser...");
    const data = await getBalanceCurve(year);
    setPoints(data);
    setStatus("");
  };

  useEffect(() => {
    if (year) {
      load();
    }
  }, [year]);

  const stats = useMemo(() => {
    if (points.length === 0) {
      return { count: 0, min: 0, max: 0 };
    }
    const balances = points.map((point) => point.balance);
    return {
      count: points.length,
      min: Math.min(...balances),
      max: Math.max(...balances)
    };
  }, [points]);

  const formatKr = (value: number) =>
    `${value.toLocaleString("da-DK", {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    })} Kr.`;

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
    return points
      .filter((point) => point.date.endsWith("-01"))
      .map((point) => point.date);
  }, [points]);

  const formatMonthLabel = (value: string) => {
    const parts = value.split("-");
    if (parts.length !== 3) return value;
    const monthIndex = Number(parts[1]) - 1;
    return monthLabels[monthIndex] ?? value;
  };

  return (
    <section className="panel">
      <header className="panel-header">
        <div>
          <h2>Saldo-graf</h2>
          <p>Saldo pr. dag for hele året (inkl. dage uden transaktioner).</p>
        </div>
        <div className="form-row">
          <input
            type="number"
            value={year}
            onChange={(event) => setYear(Number(event.target.value))}
          />
          <button type="button" className="primary" onClick={load}>
            Indlæs
          </button>
        </div>
      </header>
      <div className="panel-body">
        {status ? <p className="status">{status}</p> : null}
        <div className="meta-row">
          <span>Antal punkter: {stats.count}</span>
          <span>Min: {formatKr(stats.min)}</span>
          <span>Max: {formatKr(stats.max)}</span>
        </div>
        <div style={{ height: 320 }}>
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={points} margin={{ top: 10, right: 20, left: 30, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="date" ticks={monthTicks} tickFormatter={formatMonthLabel} />
              <YAxis width={110} tickFormatter={(value) => formatKr(Number(value))} />
              <Tooltip formatter={(value: number) => formatKr(value)} />
              <Line type="monotone" dataKey="balance" stroke="#fab387" dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>
    </section>
  );
}
