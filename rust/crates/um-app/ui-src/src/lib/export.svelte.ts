// 분석 내보내기 상태. 분석 창과 설정 "내보내기" 탭이 같은 명령(Rust `ui_export`)과 상태를 쓴다.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ExportResult { ok: boolean; auto: boolean; t: number; path: string | null; error: string | null }
export interface ExportSettings {
  range: 'day1' | 'day3' | 'week' | 'month' | 'all'; format: 'both' | 'csv' | 'json';
  lastFolder: string | null; auto: boolean; autoFolder: string | null;
  period: 'daily' | 'weekly' | 'monthly'; hour: number; minute: number; weekday: number; monthday: number;
  lastAuto: number | null; lastResult: ExportResult | null; lastAutoResult: ExportResult | null;
}
export interface ExportState { settings: ExportSettings; nextDue: number | null; autoRange: string }

export const ex = $state<{ s: ExportState | null; busy: boolean }>({ s: null, busy: false });

const call = (action: string, arg: Record<string, unknown> = {}) => invoke('ui_export', { action, arg });

export async function loadExport() {
  try {
    ex.s = (await call('state')) as ExportState;
  } catch (e) {
    console.error('ui_export', e);
  }
}

let started = false;
/** 창마다 한 번: 처음 상태를 받고, 다른 창에서 바꾸거나 자동 저장이 끝나면 다시 받는다. */
export function startExport() {
  if (started) return;
  started = true;
  loadExport();
  listen('export-state', () => loadExport());
}

export async function setExport(patch: Partial<ExportSettings>) {
  ex.s = (await call('set', patch)) as ExportState;
}

export async function pickAutoFolder() {
  ex.s = (await call('pickAutoFolder')) as ExportState;
}

/** 폴더를 고르고 지금 설정(기간·형식)대로 저장한다. 취소하면 아무것도 하지 않는다. */
export async function exportNow() {
  if (ex.busy) return;
  ex.busy = true;
  try {
    await call('exportNow');
    await loadExport();
  } catch (e) {
    console.error('exportNow', e);
  } finally {
    ex.busy = false;
  }
}

/** 자동 저장(auto) 또는 직접 저장한 마지막 결과. */
export const lastOf = (auto: boolean) => (auto ? ex.s?.settings.lastAutoResult : ex.s?.settings.lastResult) ?? null;

export const reveal = (path: string) => call('reveal', { path });

/** 로컬 날짜·시각(초 단위 Unix 시각). */
export function fmtWhen(t: number): string {
  const d = new Date(t * 1000);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}
