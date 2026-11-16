<script lang="ts">
  // 인터랙티브 화면 다이어그램(Swift segDiagram, SettingsView.swift:~875-935): 화면 모양 위에 세그먼트를 선분으로 그리고
  // 클릭으로 토글, 더블클릭으로 그 선만 선택. 선택 = AI 색, 선택 가능 = 진한 회색, 불가 = 흐리게.
  // 유효성(toggleAllowed)은 Rust가 세그먼트 그래프로 계산해 준 값을 그대로 쓴다.
  import { t } from '../lib/api.svelte';
  import type { SegOption } from './BTypes';

  let { options, color, menuOn, dockOn, warning = '', ontoggle, onexclusive }:
    { options: Record<string, SegOption>; color: string; menuOn: boolean; dockOn: boolean; warning?: string; ontoggle: (seg: string) => void; onexclusive: (seg: string) => void } = $props();

  const W = 209, H = 133, THICK = 16, BAR = 7;
  const H_SEGS = ['hTop', 'hMenu', 'hDock', 'hBottom'];
  const V_SEGS = ['lMenuBar', 'lMain', 'lDock', 'rMenuBar', 'rMain', 'rDock'];

  // 가로 레벨(0 상단, 1 메뉴바선, 2 Dock선, 3 하단)을 보기 좋게 균등 배치(Swift diagramFrames).
  const levels = $derived([0, ...(menuOn ? [1] : []), ...(dockOn ? [2] : []), 3]);
  function y(level: number): number {
    const idx = Math.max(0, levels.indexOf(level));
    return 12 + ((H - 24) * idx) / Math.max(1, levels.length - 1);
  }
  interface Frame { seg: string; x: number; y: number; w: number; h: number; horiz: boolean }
  const frames = $derived.by(() => {
    const out: Frame[] = [];
    for (const seg of H_SEGS) {
      const o = options[seg];
      if (!o?.available) continue;
      const level = H_SEGS.indexOf(seg);
      out.push({ seg, x: 26, y: y(level) - THICK / 2, w: W - 52, h: THICK, horiz: true });
    }
    for (const seg of V_SEGS) {
      const o = options[seg];
      if (!o?.available) continue;
      const [a, b] = o.ends;
      const x = a.right ? W - 14 : 14;
      const yA = y(a.level) + 9, yB = y(b.level) - 9;
      if (yB <= yA) continue;
      out.push({ seg, x: x - THICK / 2, y: yA, w: THICK, h: yB - yA, horiz: false });
    }
    return out;
  });

  // 한 번 클릭과 더블클릭을 구분하려고 한 번 클릭은 잠깐 기다린다(Swift는 같은 제스처 우선순위로 처리).
  let timer: ReturnType<typeof setTimeout> | undefined;
  function click(seg: string) {
    clearTimeout(timer);
    timer = setTimeout(() => ontoggle(seg), 220);
  }
  function dbl(seg: string) {
    clearTimeout(timer);
    onexclusive(seg);
  }
  function key(e: KeyboardEvent, seg: string) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      ontoggle(seg);
    }
  }
</script>

<div class="diagram" style:width="{W}px" style:height="{H}px">
  <svg width={W} height={H} viewBox="0 0 {W} {H}" role="group" aria-label={t('lay.dblHint')}>
    <rect class="bg" x="0.5" y="0.5" width={W - 1} height={H - 1} rx="10" />
    {#each frames as f (f.seg)}
      {@const o = options[f.seg]}
      {@const bw = f.horiz ? f.w : BAR}
      {@const bh = f.horiz ? BAR : f.h}
      <g
        class="seg" class:can={o.toggleAllowed} role="button" tabindex={o.toggleAllowed ? 0 : -1}
        aria-pressed={o.on} aria-disabled={!o.toggleAllowed} aria-label={t('seg.' + f.seg)}
        onclick={() => o.toggleAllowed && click(f.seg)} ondblclick={() => dbl(f.seg)} onkeydown={(e) => o.toggleAllowed && key(e, f.seg)}
      >
        <title>{t('seg.' + f.seg)}</title>
        <rect class="hit" x={f.x} y={f.y} width={f.w} height={f.h} />
        <rect
          class="bar" class:on={o.on} class:can={!o.on && o.toggleAllowed}
          x={f.x + (f.w - bw) / 2} y={f.y + (f.h - bh) / 2} width={bw} height={bh} rx={BAR / 2}
          style:--c={color}
        />
      </g>
    {/each}
  </svg>
  {#if warning}<div class="warn-pop" role="alert">{warning}</div>{/if}
</div>

<style>
  .diagram { position: relative; flex: none; }
  svg { display: block; }
  .bg { fill: var(--surface); stroke: var(--surface-stroke); stroke-width: 1; }
  .seg { cursor: default; outline: none; }
  .hit { fill: transparent; pointer-events: all; }
  /* 불가 = 아주 흐림, 가능 = 진한 회색 + 테두리, 선택 = AI 색 + 같은 색 테두리(Swift :893-899). */
  .bar { fill: var(--fg); opacity: 0.12; }
  .bar.can { fill: var(--fg2); opacity: 0.6; stroke: var(--fg2); stroke-width: 1; }
  .bar.on { fill: var(--c); opacity: 1; stroke: var(--c); stroke-width: 1; }
  .seg:focus-visible .bar { stroke: var(--accent); stroke-width: 2; opacity: 1; }
  .warn-pop {
    position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); max-width: 190px; text-align: center;
    background: var(--danger); color: #fff; font-size: var(--fs-small); padding: 6px 10px; border-radius: 7px; pointer-events: none;
    animation: fade 0.2s ease-out;
  }
  @keyframes fade { from { opacity: 0; } to { opacity: 1; } }
</style>
