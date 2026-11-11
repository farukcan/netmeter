import type { CollectorStatus } from "./api";

type Props = {
  status: CollectorStatus | null;
  busy: boolean;
  onInstall: () => void;
  onUninstall: () => void;
};

function statusLabel(status: CollectorStatus): string {
  if (status.running) {
    return `Collector çalışıyor (pid ${status.pid})`;
  }
  if (status.installed) {
    return "Collector kurulu ama çalışmıyor";
  }
  return "Collector kurulu değil: veri toplanmıyor";
}

/** Shows whether the background collector LaunchAgent runs, with start/stop controls. */
export function CollectorBar({ status, busy, onInstall, onUninstall }: Props) {
  if (status === null) {
    return <div className="collector-bar">Collector durumu okunuyor…</div>;
  }
  return (
    <div className="collector-bar">
      <span className={`status ${status.running ? "status-good" : "status-warning"}`}>
        <span className="status-icon" aria-hidden="true">
          {status.running ? "●" : "▲"}
        </span>
        {statusLabel(status)}
      </span>
      {status.running ? (
        <button disabled={busy} onClick={onUninstall}>
          Durdur
        </button>
      ) : (
        <button className="primary" disabled={busy} onClick={onInstall}>
          {status.installed ? "Yeniden başlat" : "Başlat"}
        </button>
      )}
    </div>
  );
}
