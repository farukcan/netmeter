import { invoke } from "@tauri-apps/api/core";

/** Downloaded and uploaded bytes. */
export type Traffic = { bytesIn: number; bytesOut: number };

/** Bytes an app moved during a period. */
export type AppUsage = Traffic & { app: string };

/** Bytes moved on one local day (`YYYY-MM-DD`). */
export type DayUsage = Traffic & { day: string };

/** State of the background collector LaunchAgent. */
export type CollectorStatus = { installed: boolean; running: boolean; pid: number | null };

/** Months with data as `YYYY-MM`, newest first. */
export function listMonths(): Promise<string[]> {
  return invoke<string[]>("list_months");
}

/** Per-app totals of a month, largest first. */
export function monthApps(month: string): Promise<AppUsage[]> {
  return invoke<AppUsage[]>("month_apps", { month });
}

/** Daily totals of all apps in a month; days without data are absent. */
export function monthDays(month: string): Promise<DayUsage[]> {
  return invoke<DayUsage[]>("month_days", { month });
}

/** Daily totals of one app in a month; days without data are absent. */
export function appDays(month: string, app: string): Promise<DayUsage[]> {
  return invoke<DayUsage[]>("app_days", { month, app });
}

export function collectorStatus(): Promise<CollectorStatus> {
  return invoke<CollectorStatus>("collector_status");
}

export function installCollector(): Promise<CollectorStatus> {
  return invoke<CollectorStatus>("install_collector");
}

export function uninstallCollector(): Promise<CollectorStatus> {
  return invoke<CollectorStatus>("uninstall_collector");
}
