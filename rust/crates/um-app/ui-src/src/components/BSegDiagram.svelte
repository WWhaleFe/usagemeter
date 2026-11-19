<script lang="ts">
  // 인터랙티브 화면 다이어그램(Swift segDiagram, SettingsView.swift:~875-935): 화면 모양 위에 세그먼트를 선분으로 그리고
  // 클릭으로 토글, 더블클릭으로 그 선만 선택. 선택 = AI 색, 선택 가능 = 진한 회색, 불가 = 흐리게.
  // 유효성(toggleAllowed)은 Rust가 세그먼트 그래프로 계산해 준 값을 그대로 쓴다.
  import { t } from '../lib/api.svelte';
  import type { SegOption, SegCap } from './BTypes';
  import { diagramLines, segmentKey, capVector, type DockSide, type Point } from './BDiagram';

  let { options, color, menuOn, dockOn, dockSide, caps = {}, warning = '', ontoggle, onexclusive }:
    { options: Record<string, SegOption>; color: string; menuOn: boolean; dockOn: boolean; dockSide: DockSide;
      caps?: Record<string, SegCap>; warning?: string; ontoggle: (seg: string) => void; onexclusive: (seg: string) => void } = $props();

  const W = 209, H = 133, BAR = 7;
  const frames = $derived(diagramLines(options, menuOn, dockOn && dockSide !== null, dockSide));
  const path = (points: Point[]) => points.map(([x, y], i) => `${i ? 'L' : 'M'}${x},${y}`).join(' ');
  function capPath(seg: string, points: Point[], right: boolean): string {
    const cap = caps[`${seg}.${right ? 'R' : 'L'}`];
    if (cap !== 'up' && cap !== 'down') return '';
    const p = points[right ? points.length - 1 : 0];
    const near = points[right ? points.length - 2 : 1];
    const n = Math.hypot(near[0] - p[0], near[1] - p[1]);
    const [dx, dy] = capVector(seg, cap === 'up', dockSide, menuOn);
    return `M${p[0] + (near[0] - p[0]) * 6 / n},${p[1] + (near[1] - p[1]) * 6 / n} Q${p[0]},${p[1]} ${p[0] + dx * 8},${p[1] + dy * 8}`;
  }

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
      {@const label = t(segmentKey(f.seg, dockSide, menuOn))}
      <g
        class="seg" class:can={o.toggleAllowed} role="button" tabindex={o.toggleAllowed ? 0 : -1}
        aria-pressed={o.on} aria-disabled={!o.toggleAllowed} aria-label={label}
        onclick={() => o.toggleAllowed && click(f.seg)} ondblclick={() => dbl(f.seg)} onkeydown={(e) => o.toggleAllowed && key(e, f.seg)}
      >
        <title>{label}</title>
        <path class="hit" d={path(f.points)} />
        {#if f.points.length === 2}
          {@const a = f.points[0]}
          {@const b = f.points[1]}
          <rect class="bar" class:on={o.on} class:can={!o.on && o.toggleAllowed}
            x={Math.min(a[0], b[0]) - (f.horiz ? 0 : BAR / 2)} y={Math.min(a[1], b[1]) - (f.horiz ? BAR / 2 : 0)}
            width={f.horiz ? Math.abs(b[0] - a[0]) : BAR} height={f.horiz ? BAR : Math.abs(b[1] - a[1])} rx={BAR / 2} style:--c={color} />
        {:else}
          <path class="stroke" class:on={o.on} class:can={!o.on && o.toggleAllowed} d={path(f.points)} style:--c={color} />
        {/if}
        {#if o.on}
          {#each [false, true] as right (right)}
            <path class="cap" d={capPath(f.seg, f.points, right)} style:--c={color} />
          {/each}
        {/if}
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
  .hit { fill: none; stroke: transparent; stroke-width: 16; pointer-events: stroke; }
  .stroke, .cap { fill: none; stroke: var(--fg); stroke-width: 7; stroke-linecap: round; stroke-linejoin: round; opacity: 0.12; pointer-events: none; }
  .stroke.can { stroke: var(--fg2); opacity: 0.6; }
  .stroke.on, .cap { stroke: var(--c); opacity: 1; }
  /* 불가 = 아주 흐림, 가능 = 진한 회색 + 테두리, 선택 = AI 색 + 같은 색 테두리(Swift :893-899). */
  .bar { fill: var(--fg); opacity: 0.12; }
  .bar.can { fill: var(--fg2); opacity: 0.6; stroke: var(--fg2); stroke-width: 1; }
  .bar.on { fill: var(--c); opacity: 1; stroke: var(--c); stroke-width: 1; }
  .seg:focus-visible .stroke { stroke: var(--accent); stroke-width: 9; opacity: 1; }
  .seg:focus-visible .bar { stroke: var(--accent); stroke-width: 2; opacity: 1; }
  .warn-pop {
    position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); max-width: 190px; text-align: center;
    background: var(--danger); color: #fff; font-size: var(--fs-small); padding: 6px 10px; border-radius: 7px; pointer-events: none;
    animation: fade 0.2s ease-out;
  }
  @keyframes fade { from { opacity: 0; } to { opacity: 1; } }
</style>
