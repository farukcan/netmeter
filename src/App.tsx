import { useEffect, useState } from "react";
import {
  appDays,
  collectorStatus,
  installCollector,
  listMonths,
  monthApps,
  monthDays,
  uninstallCollector,
  type AppUsage,
  type CollectorStatus,
  type DayUsage,
  type Traffic,
} from "./api";
import { AppTable } from "./AppTable";
import { CollectorBar } from "./CollectorBar";
import { DailyChart } from "./DailyChart";
import { fillMonth, formatBytes, formatMonth } from "./format";
import "./App.css";

/** The collector writes every 10 s; the view re-reads the database this often. */
const REFRESH_MS = 30_000;

function sumUsage(items: Traffic[]): Traffic {
  return items.reduce(
    (total, item) => ({ bytesIn: total.bytesIn + item.bytesIn, bytesOut: total.bytesOut + item.bytesOut }),
    { bytesIn: 0, bytesOut: 0 },
  );
}

function StatTiles({ bytesIn, bytesOut }: Traffic) {
  return (
    <div className="tiles">
      <div className="tile">
        <div className="tile-label">Toplam</div>
        <div className="tile-value hero">{formatBytes(bytesIn + bytesOut)}</div>
      </div>
      <div className="tile">
        <div className="tile-label">İndirilen</div>
        <div className="tile-value">{formatBytes(bytesIn)}</div>
      </div>
      <div className="tile">
        <div className="tile-label">Gönderilen</div>
        <div className="tile-value">{formatBytes(bytesOut)}</div>
      </div>
    </div>
  );
}

export default function App() {
  const [refreshTick, setRefreshTick] = useState<number>(0);
  const [months, setMonths] = useState<string[]>([]);
  const [month, setMonth] = useState<string | null>(null);
  const [apps, setApps] = useState<AppUsage[]>([]);
  const [days, setDays] = useState<DayUsage[]>([]);
  const [selectedApp, setSelectedApp] = useState<string | null>(null);
  const [selectedAppDays, setSelectedAppDays] = useState<DayUsage[]>([]);
  const [status, setStatus] = useState<CollectorStatus | null>(null);
  const [busy, setBusy] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const timer = setInterval(() => setRefreshTick((tick) => tick + 1), REFRESH_MS);
    return () => clearInterval(timer);
  }, []);

  useEffect(() => {
    let cancelled = false;
    // Loaded separately so a launchctl failure does not hide the collected data.
    listMonths()
      .then((loadedMonths) => {
        if (cancelled) return;
        setMonths(loadedMonths);
        setMonth((current) => current ?? loadedMonths[0] ?? null);
      })
      .catch((reason: unknown) => setError(String(reason)));
    collectorStatus()
      .then((loadedStatus) => {
        if (!cancelled) setStatus(loadedStatus);
      })
      .catch((reason: unknown) => setError(String(reason)));
    return () => {
      cancelled = true;
    };
  }, [refreshTick]);

  useEffect(() => {
    if (month === null) return;
    let cancelled = false;
    Promise.all([monthApps(month), monthDays(month)])
      .then(([loadedApps, loadedDays]) => {
        if (cancelled) return;
        setApps(loadedApps);
        setDays(loadedDays);
      })
      .catch((reason: unknown) => setError(String(reason)));
    return () => {
      cancelled = true;
    };
  }, [month, refreshTick]);

  useEffect(() => {
    if (month === null || selectedApp === null) return;
    let cancelled = false;
    appDays(month, selectedApp)
      .then((loadedDays) => {
        if (!cancelled) setSelectedAppDays(loadedDays);
      })
      .catch((reason: unknown) => setError(String(reason)));
    return () => {
      cancelled = true;
    };
  }, [month, selectedApp, refreshTick]);

  function changeCollector(action: () => Promise<CollectorStatus>): void {
    setBusy(true);
    setError(null);
    action()
      .then((nextStatus) => {
        setStatus(nextStatus);
        setRefreshTick((tick) => tick + 1);
      })
      .catch((reason: unknown) => setError(String(reason)))
      .finally(() => setBusy(false));
  }

  function selectMonth(nextMonth: string): void {
    setMonth(nextMonth);
    setSelectedApp(null);
  }

  function selectApp(app: string): void {
    setSelectedAppDays([]);
    setSelectedApp(app);
  }

  const selectedUsage: AppUsage | undefined = apps.find((usage) => usage.app === selectedApp);

  return (
    <main className="page">
      <header className="header">
        <h1>netmeter</h1>
        {months.length > 0 && month !== null && (
          <select value={month} onChange={(event) => selectMonth(event.target.value)}>
            {months.map((item) => (
              <option key={item} value={item}>
                {formatMonth(item)}
              </option>
            ))}
          </select>
        )}
      </header>

      <CollectorBar
        status={status}
        busy={busy}
        onInstall={() => changeCollector(installCollector)}
        onUninstall={() => changeCollector(uninstallCollector)}
      />

      {error !== null && (
        <div className="error" role="alert">
          <strong>Hata:</strong> {error}
          <button onClick={() => setError(null)}>Kapat</button>
        </div>
      )}

      {month === null ? (
        <p className="empty">Henüz veri yok. Collector'ı başlat; ilk veriler yaklaşık 30 saniye içinde görünür.</p>
      ) : selectedApp !== null ? (
        <section>
          <button className="link" onClick={() => setSelectedApp(null)}>
            ← Tüm uygulamalar
          </button>
          <h2>
            {selectedApp} · {formatMonth(month)}
          </h2>
          <StatTiles bytesIn={selectedUsage?.bytesIn ?? 0} bytesOut={selectedUsage?.bytesOut ?? 0} />
          <h3>Günlük trafik</h3>
          <DailyChart days={fillMonth(month, selectedAppDays)} />
        </section>
      ) : (
        <section>
          <StatTiles {...sumUsage(apps)} />
          <h3>Günlük trafik</h3>
          <DailyChart days={fillMonth(month, days)} />
          <h3>Uygulamalar</h3>
          <AppTable apps={apps} onSelect={selectApp} />
        </section>
      )}
    </main>
  );
}
