// 앱(Rust)과 주고받는 층. 상태 원본은 Rust이고, 화면은 `ui.s`를 그리기만 한다(계획서 3장 원칙).
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Rust `ipc::ui_state`가 돌려주는 값(필드는 그쪽 주석 참고). 설정 스냅샷 키는 Swift 저장 형식(camelCase) 그대로.
export type Snapshot = Record<string, any>;
export interface Provider {
  id: string; name: string; loggedIn: boolean; experimental: boolean; color: string;
  items: { id: string; name: string; ratio: number; color: string; key: string }[];
  itemsLive: boolean;
  segOptions: Record<string, { on: boolean; available: boolean; toggleValid: boolean; toggleAllowed: boolean; usedByOther: boolean }>;
}
export interface UiState {
  dockSide: 'top' | 'bottom' | 'left' | 'right' | null;
  platform: 'mac' | 'win'; store: boolean; version: string; lang: 'ko' | 'en' | 'ja';
  strings: Record<string, string>;
  main: Snapshot; border: Snapshot; editing: string | null;
  displays: { id: string; name: string; primary: boolean; builtin: boolean; enabled: boolean; main: boolean; notch: { width: number; height: number } | null }[];
  providers: Provider[];
  presets: { id: string; name: string }[]; maxPresets: number; hasDefault: boolean;
  refreshPresets: number[]; effectiveRefresh: number;
  brandColors: { name: string; hex: string }[];
  update: { state: string; version: string | null; download: string; latest?: string | null; blocked?: boolean };
  autostart: { available: boolean; enabled: boolean };
}

export const ui = $state<{ s: UiState | null }>({ s: null });

export async function load() {
  ui.s = (await invoke('ui_state')) as UiState;
}

export function start() {
  load();
  listen('ui-state', () => load());
}

/** 설정 일부 바꾸기. scope "main" = 전역, "border" = 편집 중인 화면의 테두리. */
export async function set(scope: 'main' | 'border', patch: Snapshot) {
  await invoke('ui_set', { scope, patchV: patch });
}

export async function act(action: string, arg: Record<string, unknown> = {}) {
  return invoke('ui_action', { action, arg });
}

export function t(key: string): string {
  return ui.s?.strings[key] ?? key;
}
export const tf = (key: string, arg: string | number) => t(key).replaceAll('%@', String(arg));
export const tn = (key: string, n: number) => t(key).replaceAll('%d', String(n));

/** sRGB [r,g,b,a](0~1) ↔ "#rrggbb" */
export function toHex(c: number[] | undefined): string {
  if (!c) return '#000000';
  return '#' + c.slice(0, 3).map((v) => Math.round(v * 255).toString(16).padStart(2, '0')).join('');
}
export function fromHex(h: string, a = 1): number[] {
  const n = parseInt(h.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255, a];
}

/** 설정 행은 두 플랫폼 모두 Swift ProviderSpec.all 순서를 따른다. */
export function settingsProviders(s: UiState): Provider[] {
  return ['claude', 'gemini', 'codex'].flatMap((id) => s.providers.filter((p) => p.id === id));
}
