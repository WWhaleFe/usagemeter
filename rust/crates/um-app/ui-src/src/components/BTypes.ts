// 구분선·레이아웃·모서리 탭이 쓰는 `ui_state` 확장 필드 타입(Rust ipc.rs: segOptions·layout·shapePresets·cornerUsed).
import type { Provider, UiState } from '../lib/api.svelte';

export type SegCap = 'none' | 'up' | 'down' | 'include';
export interface SegEnd { right: boolean; level: number }
export interface SegOption { on: boolean; available: boolean; toggleValid: boolean; toggleAllowed: boolean; usedByOther: boolean; ends: [SegEnd, SegEnd] }
export interface Layout {
  anchorCorner: string; anchorSide: string; clockwise: boolean;
  segments?: string[]; zones?: string[]; hCaps?: Record<string, SegCap>;
  [k: string]: unknown;
}
export interface ShapePreset { zones: string[]; segments: string[]; allowed: boolean }
export interface LayoutProvider extends Provider {
  segOptions: Record<string, SegOption>;
  layout: Layout;
  chain: boolean;
  anchorSideKind: 'hidden' | 'round' | 'scoop';
  shapePresets: Record<'all' | 'menuBar' | 'main' | 'dock', ShapePreset>;
}
export type CornerUsed = Record<string, Record<string, boolean>>;
export type StateEx = UiState & { cornerUsed?: CornerUsed };
