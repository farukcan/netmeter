import type { AppUsage } from "./api";
import { formatBytes } from "./format";

type Props = {
  /** Sorted largest total first. */
  apps: AppUsage[];
  onSelect: (app: string) => void;
};

/** Per-app monthly totals; the share bar is relative to the largest app. */
export function AppTable({ apps, onSelect }: Props) {
  const largest: number = Math.max(1, ...apps.map((usage) => usage.bytesIn + usage.bytesOut));
  return (
    <table className="app-table">
      <thead>
        <tr>
          <th>Uygulama</th>
          <th className="numeric">İndirilen</th>
          <th className="numeric">Gönderilen</th>
          <th className="numeric">Toplam</th>
          <th className="share-cell" aria-label="Oran" />
        </tr>
      </thead>
      <tbody>
        {apps.map((usage) => {
          const total: number = usage.bytesIn + usage.bytesOut;
          return (
            <tr key={usage.app} onClick={() => onSelect(usage.app)}>
              <td className="app-name">{usage.app}</td>
              <td className="numeric">{formatBytes(usage.bytesIn)}</td>
              <td className="numeric">{formatBytes(usage.bytesOut)}</td>
              <td className="numeric strong">{formatBytes(total)}</td>
              <td className="share-cell">
                <div className="share-bar" style={{ width: `${(total / largest) * 100}%` }} />
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
