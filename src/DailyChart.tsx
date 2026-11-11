import { useLayoutEffect, useRef, useState, type RefObject } from "react";
import type { DayUsage } from "./api";
import { formatBytes, formatBytesInUnit, formatDay, niceTicks, unitExponent } from "./format";

const HEIGHT = 220;
const MARGIN = { top: 12, right: 8, bottom: 24, left: 72 } as const;
const MAX_BAR_WIDTH = 24;
const SEGMENT_GAP = 2;
const CORNER_RADIUS = 4;

type Props = {
  /** Every day of the month in order (see `fillMonth`). */
  days: DayUsage[];
};

function useElementWidth(): [RefObject<HTMLDivElement | null>, number] {
  const ref = useRef<HTMLDivElement | null>(null);
  const [width, setWidth] = useState<number>(0);
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) {
      return;
    }
    const observer = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  return [ref, width];
}

/** Rectangle with rounded top corners; the bottom stays square on the baseline. */
function roundedTopPath(x: number, top: number, width: number, height: number): string {
  const r: number = Math.min(CORNER_RADIUS, width / 2, height);
  const bottom: number = top + height;
  return `M${x},${bottom} V${top + r} Q${x},${top} ${x + r},${top} H${x + width - r} Q${x + width},${top} ${x + width},${top + r} V${bottom} Z`;
}

function isLabeledDay(dayNumber: number): boolean {
  return dayNumber === 1 || dayNumber % 5 === 0;
}

/** Stacked daily bars: download (series 1) on the baseline, upload (series 2) on top. */
export function DailyChart({ days }: Props) {
  const [containerRef, width] = useElementWidth();
  const [hovered, setHovered] = useState<number | null>(null);

  const totals: number[] = days.map((usage) => usage.bytesIn + usage.bytesOut);
  const ticks: number[] = niceTicks(Math.max(0, ...totals));
  const axisMax: number = Math.max(ticks[ticks.length - 1], 1);
  const axisExponent: number = unitExponent(axisMax);
  const plotWidth: number = Math.max(0, width - MARGIN.left - MARGIN.right);
  const plotHeight: number = HEIGHT - MARGIN.top - MARGIN.bottom;
  const band: number = plotWidth / days.length;
  const barWidth: number = Math.min(MAX_BAR_WIDTH, band * 0.7);
  const baseline: number = MARGIN.top + plotHeight;
  const y = (bytes: number): number => baseline - (bytes / axisMax) * plotHeight;

  const hoveredDay: DayUsage | null = hovered === null ? null : days[hovered];
  const tooltipLeft: number = hovered === null ? 0 : MARGIN.left + band * (hovered + 0.5);

  return (
    <div className="chart">
      <div className="legend">
        <span className="legend-item">
          <span className="swatch swatch-in" /> İndirilen
        </span>
        <span className="legend-item">
          <span className="swatch swatch-out" /> Gönderilen
        </span>
      </div>
      <div className="chart-plot" ref={containerRef}>
        {width > 0 && (
          <svg width={width} height={HEIGHT} onMouseLeave={() => setHovered(null)} role="img" aria-label="Günlük trafik">
            {ticks.map((tick) => (
              <g key={tick}>
                <line
                  className={tick === 0 ? "axis-baseline" : "axis-grid"}
                  x1={MARGIN.left}
                  x2={width - MARGIN.right}
                  y1={y(tick)}
                  y2={y(tick)}
                />
                <text className="axis-label" x={MARGIN.left - 8} y={y(tick)} textAnchor="end" dominantBaseline="middle">
                  {formatBytesInUnit(tick, axisExponent)}
                </text>
              </g>
            ))}
            {days.map((usage, index) => {
              const dayNumber: number = index + 1;
              const x: number = MARGIN.left + band * index + (band - barWidth) / 2;
              const inTop: number = y(usage.bytesIn);
              const totalTop: number = y(usage.bytesIn + usage.bytesOut);
              const outBottom: number = usage.bytesIn > 0 ? inTop - SEGMENT_GAP : baseline;
              const hasIn: boolean = baseline - inTop > 0;
              const hasOut: boolean = outBottom - totalTop > 0;
              return (
                <g key={usage.day}>
                  {hovered === index && (
                    <rect className="hover-wash" x={MARGIN.left + band * index} y={MARGIN.top} width={band} height={plotHeight} />
                  )}
                  {hasIn &&
                    (hasOut ? (
                      <rect className="bar-in" x={x} y={inTop} width={barWidth} height={baseline - inTop} />
                    ) : (
                      <path className="bar-in" d={roundedTopPath(x, inTop, barWidth, baseline - inTop)} />
                    ))}
                  {hasOut && <path className="bar-out" d={roundedTopPath(x, totalTop, barWidth, outBottom - totalTop)} />}
                  {isLabeledDay(dayNumber) && (
                    <text className="axis-label" x={x + barWidth / 2} y={HEIGHT - 6} textAnchor="middle">
                      {dayNumber}
                    </text>
                  )}
                  <rect
                    className="hit-target"
                    x={MARGIN.left + band * index}
                    y={MARGIN.top}
                    width={band}
                    height={plotHeight}
                    onMouseEnter={() => setHovered(index)}
                  />
                </g>
              );
            })}
          </svg>
        )}
        {hoveredDay !== null && hovered !== null && (
          <div
            className={`tooltip ${hovered < days.length / 2 ? "tooltip-right" : "tooltip-left"}`}
            style={{ left: tooltipLeft, top: MARGIN.top }}
          >
            <div className="tooltip-title">{formatDay(hoveredDay.day)}</div>
            <div className="tooltip-row">
              <span className="swatch swatch-in" /> İndirilen <strong>{formatBytes(hoveredDay.bytesIn)}</strong>
            </div>
            <div className="tooltip-row">
              <span className="swatch swatch-out" /> Gönderilen <strong>{formatBytes(hoveredDay.bytesOut)}</strong>
            </div>
            <div className="tooltip-row">
              Toplam <strong>{formatBytes(hoveredDay.bytesIn + hoveredDay.bytesOut)}</strong>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
