<script lang="ts">
  // 요일 x 시간대 히트맵(Swift heatmap, AnalyticsView.swift:474-497). 첫 요일이 맨 위, 농도 = amount(0~1).
  // 색: AI 하나면 그 AI 색, 아니면 강조색. 빈 칸은 옅은 회색(Swift Gradient 시작 secondary 0.12).
  import { tn, ui } from '../lib/api.svelte';
  import ChartBox from './ChartBox.svelte';

  let { heat, base }: { heat: { weekdays: number[]; cells: [number, number, number][] }; base: string } = $props();
  // 짧은 요일 이름(앱 언어). 2023-01-01 = 일요일이라 index 0 = 일요일.
  const names = $derived.by(() => {
    const f = new Intl.DateTimeFormat(ui.s!.lang, { weekday: 'short' });
    return heat.weekdays.map((wd) => f.format(new Date(2023, 0, 1 + wd)));
  });
  const ML = 26, MB = 18, MT = 4, MR = 4;
</script>

<ChartBox>
  {#snippet children(w: number, h: number)}
    {@const cw = (w - ML - MR) / 24}
    {@const ch = (h - MT - MB) / 7}
    <svg width={w} height={h}>
      {#each names as n, r (r)}
        <text x={ML - 5} y={MT + ch * (r + 0.5)} text-anchor="end" dominant-baseline="middle">{n}</text>
      {/each}
      {#each [0, 6, 12, 18] as hr (hr)}
        <text x={ML + cw * (hr + 0.5)} y={h - 4} text-anchor="middle">{tn('analytics.hourFmt', hr)}</text>
      {/each}
      {#each heat.cells as [row, hr, amt] (row * 24 + hr)}
        {@const x = ML + cw * hr + cw * 0.05}
        {@const y = MT + ch * row + ch * 0.08}
        <rect {x} {y} width={cw * 0.9} height={ch * 0.84} rx="2" class="empty" />
        {#if amt > 0}
          <rect {x} {y} width={cw * 0.9} height={ch * 0.84} rx="2" fill={base} fill-opacity={amt}><title>{names[row]} {tn('analytics.hourFmt', hr)}</title></rect>
        {/if}
      {/each}
    </svg>
  {/snippet}
</ChartBox>

<style>
  .empty { fill: var(--fg2); fill-opacity: 0.12; }
</style>
