<script lang="ts">
  // 시간대별 사용 패턴: 0~23시 쌓인 막대(Swift hourChart, AnalyticsView.swift:346-366). x 눈금 0·3·...·21시.
  import { tn, tf } from '../lib/api.svelte';
  import ChartBox from './ChartBox.svelte';
  import { fmtTick, niceTicks, num, type Series } from './util';

  let { series }: { series: Series[] } = $props();
  // 시각별 쌓기 합(%p).
  const tot = $derived(Array.from({ length: 24 }, (_, h) => series.reduce((a, s) => a + (s.hours[h] ?? 0) * 100, 0)));
  const scale = $derived(niceTicks(Math.max(...tot)));
  const ML = 30, MB = 18, MT = 6, MR = 6;
</script>

<ChartBox>
  {#snippet children(w: number, h: number)}
    {@const pw = w - ML - MR}
    {@const ph = h - MB - MT}
    {@const slot = pw / 24}
    {@const bw = Math.min(slot * 0.6, 14)}
    <svg width={w} height={h}>
      {#each scale.ticks as v (v)}
        {@const y = MT + ph * (1 - v / scale.top)}
        <line x1={ML} x2={w - MR} y1={y} y2={y} class="grid" />
        <text x={ML - 5} {y} text-anchor="end" dominant-baseline="middle">{fmtTick(v)}</text>
      {/each}
      {#each [0, 3, 6, 9, 12, 15, 18, 21] as hr (hr)}
        <line x1={ML + slot * (hr + 0.5)} x2={ML + slot * (hr + 0.5)} y1={MT} y2={MT + ph} class="grid v" />
        <text x={ML + slot * (hr + 0.5)} y={h - 4} text-anchor="middle">{tn('analytics.hourFmt', hr)}</text>
      {/each}
      {#each Array.from({ length: 24 }, (_, i) => i) as hr (hr)}
        {@const x = ML + slot * (hr + 0.5) - bw / 2}
        {@const stack = series.map((s) => (s.hours[hr] ?? 0) * 100)}
        {#each series as s, i (s.item ?? s.provider)}
          {#if stack[i] > 0}
            {@const below = stack.slice(0, i).reduce((a, b) => a + b, 0)}
            {@const y1 = MT + ph * (1 - below / scale.top)}
            {@const y2 = MT + ph * (1 - (below + stack[i]) / scale.top)}
            <rect {x} y={y2} width={bw} height={Math.max(0.5, y1 - y2)} fill={s.color} rx="1.5"><title>{tn('analytics.hourFmt', hr)} {s.name}: {tf('analytics.ppFmt', num(stack[i]))}</title></rect>
          {/if}
        {/each}
      {/each}
    </svg>
  {/snippet}
</ChartBox>

<style>
  .grid { stroke: var(--grid); stroke-width: 1; }
  .grid.v { stroke-dasharray: 2 3; }
</style>
