<script lang="ts">
  // 항목별 소비량 가로 막대(Swift itemUseChart, AnalyticsView.swift:325-340). 막대 끝에 %p 값.
  import { tf } from '../lib/api.svelte';
  import { num, type Series } from './util';

  let { series }: { series: Series[] } = $props();
  const max = $derived(Math.max(0.0001, ...series.map((s) => s.consumed)));
</script>

<div class="bars">
  {#each series as s (s.item)}
    <div class="row" title="{s.name}: {tf('analytics.ppFmt', num(s.consumed * 100))}">
      <span class="name">{s.name}</span>
      <span class="track"><span class="bar" style:width="{(s.consumed / max) * 100}%" style:background={s.color}></span></span>
      <span class="val">{tf('analytics.ppFmt', num(s.consumed * 100))}</span>
    </div>
  {/each}
</div>

<style>
  .bars { flex: 1; min-height: 0; display: flex; flex-direction: column; justify-content: space-around; gap: 6px; overflow: auto; }
  .row { display: grid; grid-template-columns: minmax(48px, 30%) 1fr auto; align-items: center; gap: 8px; min-height: 22px; }
  .name { font-size: var(--fs-small); color: var(--fg2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .track { display: block; height: 16px; min-width: 0; }
  .bar { display: block; height: 100%; min-width: 2px; border-radius: 3px; }
  .val { font-size: 10px; color: var(--fg2); font-variant-numeric: tabular-nums; white-space: nowrap; }
</style>
