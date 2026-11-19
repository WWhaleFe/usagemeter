// 세그먼트 ID는 아래 기준 그래프 그대로 두고 실제 화면에서 보이는 좌표·명칭만 바꾼다.
import type { SegOption } from './BTypes';
export type DockSide = 'bottom' | 'top' | 'left' | 'right' | null;
export type Point = [number, number];
export interface DiagramLine { seg: string; points: Point[]; horiz: boolean }
export const H_SEGS = ['hTop', 'hMenu', 'hDock', 'hBottom'];
export const ORDER = [...H_SEGS, 'lMenuBar', 'lMain', 'lDock', 'rMenuBar', 'rMain', 'rDock'];
export const sideDock = (side: DockSide) => side === 'left' || side === 'right';

export function diagramLines(options: Record<string, SegOption>, menu: boolean, dock: boolean, side: DockSide, trimEnds = true): DiagramLine[] {
  const levels = [0, ...(menu ? [1] : []), ...(dock ? [2] : []), 3];
  const lateral = sideDock(side);
  const y = (level: number, extent: number, margin: number) => margin + (extent - 2 * margin) *
    (levels.includes(level) ? levels.indexOf(level) : levels.length - 1) / (levels.length - 1);
  return ORDER.flatMap((seg) => {
    const o = options[seg];
    if (!o?.available || (!dock && ['hDock', 'lDock', 'rDock'].includes(seg))) return [];
    // 실제 경계와 같은 직선을 그린다. 끝의 클릭 여백만 줄인다.
    let points: Point[];
    if (menu && dock && lateral) {
      const m = 48, d = 74, l = 14, r = 195, t = 12, b = 121;
      const paths: Record<string, Point[]> = {
        hTop: [[l, t], [r, t]], hMenu: [[d, m], [r, m]], hDock: [[d, m], [d, b]],
        hBottom: [[l, m], [l, b]], lMenuBar: [[l, t], [l, m]], rMenuBar: [[r, t], [r, m]],
        // 본문 위와 Dock 위는 같은 높이에서 한 번씩만 그린다.
        lMain: [[r, m], [r, b]], lDock: [[l, m], [d, m]],
        rMain: [[r, b], [d, b]], rDock: [[d, b], [l, b]],
      };
      points = paths[seg];
      if (side === 'right' && !['hTop', 'lMenuBar', 'rMenuBar'].includes(seg)) {
        points = points.map(([x, y]) => [209 - x, y]);
      }
    } else {
      points = o.ends.map((end): Point => {
        const x = end.right ? (lateral ? 121 : 195) : (lateral ? 12 : 14);
        let cy = y(end.level, lateral ? 209 : 133, lateral ? 14 : 12);
        if (side === 'top') return [x, 133 - cy];
        if (side === 'left') return [209 - cy, x];
        if (side === 'right') return [cy, x];
        return [x, cy];
      });
    }
    if (points.every((p) => p[0] === points[0][0] && p[1] === points[0][1])) return [];
    // 끝 사이의 여백은 기존 아래 Dock 그림과 같은 크기다.
    const horiz = points.every((p) => p[1] === points[0][1]);
    const trim = (a: Point, b: Point): Point => {
      const n = Math.hypot(b[0] - a[0], b[1] - a[1]);
      const gap = Math.min(n / 3, a[1] === b[1] ? 12 : 9);
      return [a[0] + (b[0] - a[0]) * gap / n, a[1] + (b[1] - a[1]) * gap / n];
    };
    if (!trimEnds) return [{ seg, points, horiz }];
    const last = points.length - 1;
    const start = trim(points[0], points[1]), end = trim(points[last], points[last - 1]);
    return [{ seg, points: [start, ...points.slice(1, -1), end], horiz }];
  });
}

export function segmentKey(seg: string, side: DockSide, menu: boolean): string {
  if (!side || side === 'bottom') return 'seg.' + seg;
  const lateral = sideDock(side);
  if (lateral && menu) {
    const fixed: Record<string, string> = {
      hMenu: 'mainTop', hDock: 'dockVertical', hBottom: 'dockOuter',
      lMain: 'mainOuter', lDock: 'dockTop', rDock: 'dockBottom', rMain: 'mainBottom',
    };
    return 'seg.' + (fixed[seg] ?? seg);
  }
  const names: Record<string, string[]> = {
    top: ['hBottom', 'hMenu', 'hDock', 'hTop', 'lMenuBar', 'lMain', 'lDock', 'rMenuBar', 'rMain', 'rDock'],
    left: ['mainOuter', 'hMenu', 'dockVertical', 'dockOuter', 'lMenuBar', 'mainTop', 'dockTop', 'rMenuBar', 'mainBottom', 'dockBottom'],
    right: ['mainOuter', 'hMenu', 'dockVertical', 'dockOuter', 'lMenuBar', 'mainTop', 'dockTop', 'rMenuBar', 'mainBottom', 'dockBottom'],
  };
  return 'seg.' + names[side][ORDER.indexOf(seg)];
}

// 캡의 up/down은 저장된 기준 구역 방향이다. 화면 회전 뒤의 벡터로 표시한다.
export function capVector(seg: string, up: boolean, side: DockSide, menu: boolean): Point {
  const sign = up ? -1 : 1;
  if (sideDock(side) && (!menu || seg === 'hDock' || seg === 'hBottom')) {
    return [side === 'left' ? -sign : sign, 0];
  }
  return [0, side === 'top' ? -sign : sign];
}
