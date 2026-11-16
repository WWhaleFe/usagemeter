<script lang="ts">
  // 세부 항목 상세 표(Swift detailTable·detailRow·etaText, AnalyticsView.swift:501-574).
  import { t, tf, tn } from '../lib/api.svelte';
  import Icon from './Icon.svelte';
  import { fmtHM, fmtMD, num, type Group, type Row } from './util';

  let { groups }: { groups: Group[] } = $props();
  const empty = $derived(groups.every((g) => g.rows.length === 0));
  const cols = ['item', 'now', 'used', 'resets', 'util', 'hits', 'pace', 'eta'];
  const pct = (v: number | null) => (v == null ? '-' : `${Math.round(v * 100)}%`);

  /** 소진 예상 시각: 오늘이면 HH:mm, 아니면 M/d HH:mm. */
  function etaAt(sec: number): string {
    const d = new Date(sec * 1000);
    const today = new Date();
    const same = d.getFullYear() === today.getFullYear() && d.getMonth() === today.getMonth() && d.getDate() === today.getDate();
    return same ? fmtHM(d) : `${fmtMD(d)} ${fmtHM(d)}`;
  }
</script>

{#snippet line(r: Row)}
  <tr>
    <td class="name"><i class="dot" style:background={r.color}></i><span>{r.name}</span></td>
    <td><span class="now"><span class="track" style:background="color-mix(in srgb, {r.color} 18%, transparent)"><span style:width="{50 * (r.current ?? 0)}px" style:background={r.color}></span></span>{pct(r.current)}</span></td>
    <td>{tf('analytics.ppFmt', num(r.consumed * 100))}</td>
    <td>{tn('analytics.countFmt', r.resets)}</td>
    <td>{pct(r.windowUse)}</td>
    <td class:hit={r.limitHits > 0}>{tn('analytics.countFmt', r.limitHits)}</td>
    <td>{r.ratePerHour == null ? '-' : tf('analytics.perHourFmt', num(r.ratePerHour * 100))}</td>
    <td>
      {#if r.eta.kind === 'safe'}<span class="safe">{t('analytics.etaSafe')}</span>
      {:else if r.eta.kind === 'at'}<span class="hit">{tf('analytics.etaAt', etaAt(r.eta.t ?? 0))}</span>
      {:else}<span class="idle">{t('analytics.etaIdle')}</span>{/if}
    </td>
  </tr>
{/snippet}

<section class="card">
  <h2><Icon name="table" size={15} />{t('analytics.detail')}</h2>
  <p class="desc">{t('analytics.detailDesc')}</p>
  {#if empty}
    <p class="none">{t('analytics.itemsNone')}</p>
  {:else}
    <div class="scroll">
      <table>
        <thead><tr>{#each cols as k (k)}<th>{t('analytics.col.' + k)}</th>{/each}</tr></thead>
        <tbody>
          {#each groups as g (g.id)}
            {#if groups.length > 1}
              <tr class="group"><td colspan="8" style:color={g.color}>{g.name}</td></tr>
            {/if}
            {#each g.rows as r (r.name)}{@render line(r)}{/each}
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>

<style>
  .card { display: flex; flex-direction: column; gap: 8px; padding: 16px; background: var(--card-bg); border: var(--card-border); border-radius: var(--card-r); min-width: 0; }
  h2 { display: flex; align-items: center; gap: 6px; margin: 0; font-size: 13px; font-weight: 600; }
  .desc { margin: 0; font-size: var(--fs-small); color: var(--fg2); }
  .none { margin: 0; padding: 8px 0; color: var(--fg2); }
  .scroll { overflow-x: auto; }
  table { border-collapse: separate; border-spacing: 0; width: 100%; font-size: var(--fs-small); font-variant-numeric: tabular-nums; }
  th { text-align: left; font-weight: 600; color: var(--fg2); padding: 0 14px 6px 0; border-bottom: 1px solid var(--grid); white-space: nowrap; }
  td { padding: 4px 14px 4px 0; white-space: nowrap; }
  tbody tr:first-child td { padding-top: 8px; }
  .group td { font-size: 12px; font-weight: 600; padding-top: 10px; }
  .name { min-width: 170px; }
  .name span { vertical-align: middle; }
  .dot { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 6px; vertical-align: middle; }
  .now { display: inline-flex; align-items: center; gap: 6px; }
  .track { display: inline-block; width: 50px; height: 6px; border-radius: 3px; overflow: hidden; }
  .track span { display: block; height: 100%; border-radius: 3px; }
  .hit { color: var(--warn); }
  .safe { color: var(--ok); }
  .idle { color: var(--fg2); }
</style>
