import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type TargetKind = "file" | "dir" | "contents" | "glob";
export type Risk = "safe" | "caution";
export type LogLevel = "info" | "ok" | "warn" | "err";

export interface Target {
  id: string;
  group: string;
  path: string;
  pattern: string | null;
  kind: TargetKind;
  risk: Risk;
  admin: boolean;
  defaultOn: boolean;
  exists: boolean;
  size: number;
  files: number;
  scanned: boolean;
  note: string | null;
}

export interface RunningProcess {
  name: string;
  pid: number;
  exe: string | null;
}

export interface Environment {
  user: string;
  isAdmin: boolean;
  savedGames: string | null;
  gameRoots: string[];
  profileRoots: string[];
  gamecenterDir: string | null;
  gamecenterExe: string | null;
  downloadPath: string | null;
  gamesInstallPath: string | null;
  running: RunningProcess[];
  freeSpace: number | null;
  notes: string[];
}

export interface Snapshot {
  environment: Environment;
  targets: Target[];
}

export interface ScanSummary {
  targets: Target[];
  totalBytes: number;
  totalFiles: number;
  durationMs: number;
  errors: CleanError[];
}

export interface CleanError {
  path: string;
  message: string;
}

export interface CleanReport {
  dryRun: boolean;
  bytes: number;
  files: number;
  dirs: number;
  skipped: number;
  killed: RunningProcess[];
  errors: CleanError[];
  targets: Target[];
  durationMs: number;
}

export interface ProgressEvent {
  phase: "scan" | "clean";
  targetId: string;
  path: string;
  bytes: number;
  files: number;
  index: number;
  total: number;
  done: boolean;
  error: string | null;
}

export interface LogEvent {
  level: LogLevel;
  msg: string;
}

export interface Preview {
  planId: number;
  bytes: number;
  files: { path: string; size: number; targetId: string }[];
  errors: CleanError[];
}

export const api = {
  snapshot: (gamePath: string | null = null) => invoke<Snapshot>("snapshot", { gamePath }),
  scan: () => invoke<ScanSummary>("scan_targets"),
  preview: (ids: string[]) => invoke<Preview>("preview_targets", { ids }),
  clean: (planId: number) => invoke<CleanReport>("clean_targets", { planId, confirmed: true }),
  reveal: (targetId: string) => invoke<void>("reveal", { targetId }),
  elevate: () => invoke<void>("elevate"),
};

export const onProgress = (handler: (payload: ProgressEvent) => void): Promise<UnlistenFn> =>
  listen<ProgressEvent>("wf://progress", (event) => handler(event.payload));

export function formatBytes(bytes: number, digits = 1): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const precision = unit === 0 ? 0 : value >= 100 ? 0 : digits;
  return `${value.toFixed(precision)} ${units[unit]}`;
}

export function formatCount(value: number): string {
  return new Intl.NumberFormat("ru-RU").format(value);
}
