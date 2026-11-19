<script lang="ts">
  // 잔여량 추이 선 차트(Swift remainingChart, AnalyticsView.swift:372-396): y 0~100%, x = 기간 시작~지금. 아래에 범례 한 줄.
  import ChartBox from './ChartBox.svelte';
  import { timeTicks, type Series } from './util';

  let { series, since, now }: { series: Series[]; since: number; now: number } = $props();
  const ticks = $derived(timeTicks(since, now));
  const ML = 10, MR = 36;
  const MB = 18, MT = 6;
</script>

<div class="wrap">
  <ChartBox>
    {#snippet children(w: number, h: number)}
      {@const pw = w - ML - MR}
      {@const ph = h - MB - MT}
      {@const X = (t: number) => ML + ((t - since) / (now - since)) * pw}
      {@const Y = (r: number) => MT + ph * (1 - r)}
      <svg width={w} height={h}>
        {#each [0, 25, 50, 75, 100] as v (v)}
          <line x1={ML} x2={w - MR} y1={Y(v / 100)} y2={Y(v / 100)} class="grid" />
          <text x={w - MR + 5} y={Y(v / 100)} text-anchor="start" dominant-baseline="middle">{v}%</text>
        {/each}
        {#each ticks as tk (tk.t)}
          <line x1={X(tk.t)} x2={X(tk.t)} y1={MT} y2={MT + ph} class="grid v" />
          <text x={X(tk.t)} y={h - 4} text-anchor="middle">{tk.label}</text>
        {/each}
        {#each series as s (s.item ?? s.provider)}
          <polyline points={s.remaining.map(([t, r]) => `${X(t).toFixed(1)},${Y(r).toFixed(1)}`).join(' ')}
            fill="none" stroke={s.color} stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round"><title>{s.name}</title></polyline>
        {/each}
      </svg>
    {/snippet}
  </ChartBox>
  <div class="legend">
    {#each series as s (s.item ?? s.provider)}
      <span><i style:background={s.color}></i>{s.name}</span>
    {/each}
  </div>
</div>

<style>
  .wrap { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 6px; }
  .grid { stroke: var(--grid); stroke-width: 1; }
  .grid.v { stroke-dasharray: 2 3; }
  .legend { display: flex; flex-wrap: wrap; gap: 4px 12px; font-size: 10px; color: var(--fg2); }
  .legend span { display: inline-flex; align-items: center; gap: 4px; white-space: nowrap; }
  .legend i { width: 7px; height: 7px; border-radius: 50%; }
</style>
