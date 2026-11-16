<script lang="ts">
  // AI별 소비 비중 도넛(Swift shareChart, AnalyticsView.swift:312-323: 안쪽 반지름 0.62, 조각 사이 틈).
  import { tf } from '../lib/api.svelte';
  import ChartBox from './ChartBox.svelte';
  import { times, type Series } from './util';

  let { series }: { series: Series[] } = $props();
  const data = $derived(series.filter((s) => s.consumed > 0));
  const sum = $derived(data.reduce((a, s) => a + s.consumed, 0));

  /** 조각 하나의 경로. 바깥 반지름 R, 안쪽 r, 시작·끝 각도(12시 방향부터 시계 방향). */
  function arc(cx: number, cy: number, R: number, r: number, a0: number, a1: number): string {
    const pt = (rad: number, ang: number) => `${(cx + rad * Math.sin(ang)).toFixed(2)} ${(cy - rad * Math.cos(ang)).toFixed(2)}`;
    const large = a1 - a0 > Math.PI ? 1 : 0;
    return `M${pt(R, a0)} A${R} ${R} 0 ${large} 1 ${pt(R, a1)} L${pt(r, a1)} A${r} ${r} 0 ${large} 0 ${pt(r, a0)}Z`;
  }
  function slices(R: number) {
    let a = 0;
    const gap = data.length > 1 ? 3 / R : 0;
    return data.map((s) => {
      const span = (s.consumed / sum) * Math.PI * 2;
      const out = { s, a0: a + gap / 2, a1: a + span - gap / 2, span };
      a += span;
      return out;
    });
  }
  const tip = (s: Series) => `${s.name}: ${tf('analytics.timesFmt', times(s.consumed))} (${((s.consumed / sum) * 100).toFixed(0)}%)`;
</script>

<ChartBox>
  {#snippet children(w: number, h: number)}
    {@const R = Math.max(10, Math.min(w, h) / 2 - 4)}
    <svg width={w} height={h}>
      {#if data.length === 1}
        <circle cx={w / 2} cy={h / 2} r={(R + R * 0.62) / 2} fill="none" stroke={data[0].color} stroke-width={R * 0.38}><title>{tip(data[0])}</title></circle>
      {:else}
        {#each slices(R) as sl (sl.s.name)}
          <path d={arc(w / 2, h / 2, R, R * 0.62, sl.a0, sl.a1)} fill={sl.s.color}><title>{tip(sl.s)}</title></path>
        {/each}
      {/if}
    </svg>
  {/snippet}
</ChartBox>
