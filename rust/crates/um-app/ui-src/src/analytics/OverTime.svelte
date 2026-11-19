<script lang="ts">
  // 기간별 소비 추이: 버킷마다 쌓인 막대(Swift overTimeChart, AnalyticsView.swift:423-440).
  // 버킷 폭 = bucketSeconds(1일 1시간, 3일 3시간, 주간·한달 하루). 하루 버킷은 날짜 라벨.
  import { tn, tf } from '../lib/api.svelte';
  import ChartBox from './ChartBox.svelte';
  import { bucketStart, fmtMD, fmtTick, niceTicks, num, type Series } from './util';

  let { series, since, now, step }: { series: Series[]; since: number; now: number; step: number } = $props();
  const isDay = $derived(step >= 86400);
  // 가로축 칸: 기간 시작 버킷 ~ 지금 버킷.
  const slots = $derived.by(() => {
    const out: number[] = [];
    const last = bucketStart(now, step);
    let t = bucketStart(since, step);
    const d = new Date(t * 1000);
    while (t <= last && out.length < 400) {
      out.push(t);
      if (isDay) { d.setDate(d.getDate() + 1); t = d.getTime() / 1000; } else t += step;
    }
    return out;
  });
  const idx = (t: number) => {
    let best = 0;
    slots.forEach((s, i) => { if (Math.abs(s - t) < Math.abs(slots[best] - t)) best = i; });
    return best;
  };
  // 계열별 칸 값(%p).
  const vals = $derived(series.map((s) => {
    const v = new Array<number>(slots.length).fill(0);
    for (const [t, a] of s.buckets) v[idx(t)] += a * 100;
    return v;
  }));
  const tot = $derived(slots.map((_, i) => vals.reduce((a, v) => a + v[i], 0)));
  const scale = $derived(niceTicks(Math.max(...tot, 0)));
  // 라벨을 붙일 칸: 하루 버킷은 n칸 간격, 시간 버킷은 3시간(1일)/6시간(3일) 배수 시각.
  const every = $derived(Math.ceil(slots.length / 7));
  const labelH = $derived(step <= 3600 ? 3 : 6);
  function label(i: number): string {
    const d = new Date(slots[i] * 1000);
    if (isDay) return i % every === 0 ? fmtMD(d) : '';
    if (d.getHours() === 0) return fmtMD(d);
    return d.getHours() % labelH === 0 ? tn('analytics.hourFmt', d.getHours()) : '';
  }
  const ML = 6, MR = 30;
  const MB = 18, MT = 6;
</script>

<ChartBox>
  {#snippet children(w: number, h: number)}
    {@const pw = w - ML - MR}
    {@const ph = h - MB - MT}
    {@const slot = pw / Math.max(1, slots.length)}
    {@const bw = Math.max(1, slot * 0.72)}
    <svg width={w} height={h}>
      {#each scale.ticks as v (v)}
        {@const y = MT + ph * (1 - v / scale.top)}
        <line x1={ML} x2={w - MR} y1={y} y2={y} class="grid" />
        <text x={w - MR + 5} {y} text-anchor="start" dominant-baseline="middle">{fmtTick(v)}</text>
      {/each}
      {#each slots as _, i (i)}
        {@const lb = label(i)}
        {#if lb}<text x={ML + slot * (i + 0.5)} y={h - 4} text-anchor="middle">{lb}</text>{/if}
        {#each series as s, k (s.item ?? s.provider)}
          {#if vals[k][i] > 0}
            {@const below = vals.slice(0, k).reduce((a, v) => a + v[i], 0)}
            {@const y1 = MT + ph * (1 - below / scale.top)}
            {@const y2 = MT + ph * (1 - (below + vals[k][i]) / scale.top)}
            <rect x={ML + slot * (i + 0.5) - bw / 2} y={y2} width={bw} height={Math.max(0.5, y1 - y2)} fill={s.color} rx="1.5">
              <title>{fmtMD(new Date(slots[i] * 1000))}{isDay ? '' : ' ' + String(new Date(slots[i] * 1000).getHours()).padStart(2, '0') + ':00'} {s.name}: {tf('analytics.ppFmt', num(vals[k][i]))}</title>
            </rect>
          {/if}
        {/each}
      {/each}
    </svg>
  {/snippet}
</ChartBox>

<style>
  .grid { stroke: var(--grid); stroke-width: 1; }
</style>
