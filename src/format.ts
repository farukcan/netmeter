import type { DayUsage } from "./api";

const LOCALE = "tr-TR";
const BYTE_UNITS: readonly string[] = ["B", "KB", "MB", "GB", "TB"];

/** Power of 1024 that suits `bytes` (0 = B, 1 = KB, 2 = MB, ...). */
export function unitExponent(bytes: number): number {
  if (bytes < 1024) {
    return 0;
  }
  return Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), BYTE_UNITS.length - 1);
}

/** Size in a fixed unit, so values that are compared (axis ticks) share it: exponent 2 → `0,5 MB`. */
export function formatBytesInUnit(bytes: number, exponent: number): string {
  const value: number = bytes / 1024 ** exponent;
  const digits: number = value < 10 ? 2 : 1;
  return `${value.toLocaleString(LOCALE, { maximumFractionDigits: digits })} ${BYTE_UNITS[exponent]}`;
}

/** Human-readable size in powers of 1024, e.g. `1,5 GB`. */
export function formatBytes(bytes: number): string {
  return formatBytesInUnit(bytes, unitExponent(bytes));
}

function parseMonth(month: string): { year: number; monthNumber: number } {
  const [year, monthNumber] = month.split("-").map(Number);
  return { year, monthNumber };
}

/** `2026-11` → `Kasım 2026` */
export function formatMonth(month: string): string {
  const { year, monthNumber } = parseMonth(month);
  return new Date(year, monthNumber - 1, 1).toLocaleDateString(LOCALE, { month: "long", year: "numeric" });
}

/** `2026-11-11` → `11 Kasım Çar` */
export function formatDay(day: string): string {
  const [year, monthNumber, dayNumber] = day.split("-").map(Number);
  return new Date(year, monthNumber - 1, dayNumber).toLocaleDateString(LOCALE, {
    day: "numeric",
    month: "long",
    weekday: "short",
  });
}

/** Every day of a month in order, with zero usage for days that have no data. */
export function fillMonth(month: string, days: DayUsage[]): DayUsage[] {
  const { year, monthNumber } = parseMonth(month);
  const dayCount: number = new Date(year, monthNumber, 0).getDate();
  const byDay: Map<string, DayUsage> = new Map(days.map((usage) => [usage.day, usage]));
  return Array.from({ length: dayCount }, (_, index): DayUsage => {
    const day = `${month}-${String(index + 1).padStart(2, "0")}`;
    return byDay.get(day) ?? { day, bytesIn: 0, bytesOut: 0 };
  });
}

/** Round axis ticks from 0 that cover `max`, stepped as 1/2/5 × 10ⁿ of a 1024-based unit. */
export function niceTicks(max: number): number[] {
  if (max <= 0) {
    return [0];
  }
  const unit: number = 1024 ** unitExponent(max);
  const rawStep: number = max / unit / 4;
  const magnitude: number = 10 ** Math.floor(Math.log10(rawStep));
  const multiplier: number = [1, 2, 5, 10].find((candidate) => candidate * magnitude >= rawStep) ?? 10;
  const step: number = multiplier * magnitude * unit;
  const count: number = Math.ceil(max / step);
  return Array.from({ length: count + 1 }, (_, index) => index * step);
}
