<script lang="ts">
  // 분석 창(Swift AnalyticsView.swift 이식). 계산은 Rust(`ui_analytics`)가 하고 여기서는 받은 숫자만 그린다.
  // 필터 상태(기간·AI·항목)는 여기서 들고, 바뀔 때마다 다시 받는다(Swift buildModel + onChange(of: scope), 159행).
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { ui, t, tf, tn, act } from '../lib/api.svelte';
  import Card from './Card.svelte';
  import DetailTable from './DetailTable.svelte';
  import Donut from './Donut.svelte';
  import HourChart from './HourChart.svelte';
  import Heatmap from './Heatmap.svelte';
  import Icon from './Icon.svelte';
  import ItemBars from './ItemBars.svelte';
  import LineChart from './LineChart.svelte';
  import OverTime from './OverTime.svelte';
  import { RANGES, fmtYMD, type Data, type Range } from './util';

  let range = $state<Range>('day1');
  /** 자세히 볼 AI(null = 전체 AI 비교). */
  let scope = $state<string | null>(null);
  /** 고른 AI에서 보이게 할 세부 항목(비어 있으면 전체). */
  let items = $state<string[]>([]);
  let data = $state<Data | null>(null);
  /** 데이터 없는 AI 칩을 눌렀을 때 잠깐 뜨는 안내(2.2초). */
  let flash = $state<string | null>(null);
  let flashTimer: ReturnType<typeof setTimeout> | undefined;

  let seq = 0;
  async function fetchData() {
    const my = ++seq;
    try {
      const d = (await invoke('ui_analytics', { range, scope, items: $state.snapshot(items) })) as Data;
      if (my === seq) data = d;
    } catch (e) {
      console.error('ui_analytics', e);
    }
  }
  // 기간·AI·항목이 바뀌면 다시 받는다.
  $effect(() => { fetchData(); });
  // 앱 상태가 바뀔 때와 60초마다도 다시 받는다.
  onMount(() => {
    const un = listen('ui-state', () => fetchData());
    const timer = setInterval(fetchData, 60_000);
    return () => { un.then((f) => f()); clearInterval(timer); clearTimeout(flashTimer); };
  });

  function pickScope(id: string | null) {
    scope = id;
    items = [];
  }
  /** AI 칩: 다시 누르면 전체로, 데이터 없는 AI는 안내만(Swift 176행). */
  function chipAI(id: string, name: string) {
    if (!data?.hasData.includes(id)) {
      flash = tf('analytics.noDataFlash', name);
      clearTimeout(flashTimer);
      flashTimer = setTimeout(() => (flash = null), 2200);
      return;
    }
    pickScope(scope === id ? null : id);
  }
  /** 항목 칩: 전체 상태에서 누르면 그 항목만, 그다음부터는 켜고 끄기, 전부 켜면 전체(Swift 197-203행). */
  function chipItem(key: string) {
    let sel = [...items];
    if (sel.length === 0) sel = [key];
    else if (sel.includes(key)) sel = sel.filter((k) => k !== key);
    else sel.push(key);
    items = sel.length === (data?.scopedItems.length ?? 0) ? [] : sel;
  }

  const d = $derived(data);
  const empty = $derived(d != null && d.hasData.length === 0);
  const heatBase = $derived.by(() => {
    if (!d) return 'var(--accent)';
    if (scope) return d.providers.find((p) => p.id === scope)?.color ?? 'var(--accent)';
    return d.series.length === 1 ? d.series[0].color : 'var(--accent)';
  });
</script>

{#snippet chip(title: string, color: string, on: boolean, enabled: boolean, click: () => void)}
  <button class="chip" class:on class:off={!enabled} style:--c={color} onclick={click}>
    <i class="dot"></i><span>{title}</span>
  </button>
{/snippet}

{#snippet kpi(icon: string, title: string, value: string, sub: string)}
  <div class="tile">
    <div class="tt"><Icon name={icon} size={13} />{title}</div>
    <div class="val">{value}</div>
    <div class="sub">{sub}</div>
  </div>
{/snippet}

<div class="page">
  <div class="inner">
    <header>
      <div class="top">
        <h1>{t('analytics.title')}</h1>
        {#if d?.earliest != null}
          <span class="since"><Icon name="cal" size={13} />{tf('analytics.since', fmtYMD(new Date(d.earliest * 1000)))}</span>
        {/if}
        <button class="gear" onclick={() => act('openSettings', {})}><Icon name="gear" size={14} />{t('menu.settings')}</button>
      </div>
      <div class="seg" role="radiogroup" aria-label={t('analytics.range')}>
        {#each RANGES as r (r.id)}
          <button role="radio" aria-checked={range === r.id} class:on={range === r.id} onclick={() => (range = r.id)}>{t(r.key)}</button>
        {/each}
      </div>
    </header>

    {#if d}
      <div class="chips">
        {@render chip(t('analytics.allAI'), 'var(--accent)', scope === null, true, () => pickScope(null))}
        {#each d.providers as p (p.id)}
          {@render chip(p.name, p.color, scope === p.id, d.hasData.includes(p.id), () => chipAI(p.id, p.name))}
        {/each}
        {#if flash}<span class="flash">{flash}</span>{/if}
      </div>
      {#if scope && d.scopedItems.length > 0}
        <div class="chips">
          <span class="lbl">{t('analytics.items')}</span>
          {@render chip(t('analytics.allAI'), 'var(--fg2)', items.length === 0, true, () => (items = []))}
          {#each d.scopedItems as it (it.id)}
            {@render chip(it.name, it.color, items.length === 0 || items.includes(it.id), true, () => chipItem(it.id))}
          {/each}
        </div>
      {/if}

      {#if empty}
        <div class="empty">
          <Icon name="doc" size={38} />
          <p>{t('analytics.noData')}</p>
        </div>
      {:else}
        <div class="kpis">
          {@render kpi('bars', t('analytics.total'), tf('analytics.timesFmt', d.kpi.total), t('analytics.unitDesc'))}
          {@render kpi('cal', t('analytics.dailyAvg'), tf('analytics.timesFmt', d.kpi.dailyAvg), t('analytics.unitDesc'))}
          {@render kpi('clock', t('analytics.busiest'), d.kpi.busiest == null ? '-' : tn('analytics.hourFmt', d.kpi.busiest), ' ')}
          {@render kpi('octagon', t('analytics.hits'), tn('analytics.countFmt', d.kpi.hits), t('analytics.hitsDesc'))}
        </div>

        <div class="cols a">
          {#if d.itemMode}
            <Card title={t('analytics.itemUse')} desc={t('analytics.itemUseDesc')} icon="bars">
              <ItemBars series={d.series} />
            </Card>
          {:else}
            <Card title={t('analytics.share')} desc={t('analytics.shareDesc')} icon="pie">
              <Donut series={d.series} />
            </Card>
          {/if}
          <Card title={t('analytics.byHour')} desc={t(d.itemMode ? 'analytics.byHourItemDesc' : 'analytics.byHourDesc')} icon="clock">
            <HourChart series={d.series} />
          </Card>
        </div>
        <Card title={t('analytics.remaining')} desc={t('analytics.remainingDesc')} icon="line" height={260}>
          <LineChart series={d.series} since={d.since} now={d.now} />
        </Card>
        <div class="cols b">
          <Card title={t('analytics.overTime')} desc={t('analytics.overTimeDesc')} icon="bars2">
            <OverTime series={d.series} since={d.since} now={d.now} step={d.bucketSeconds} />
          </Card>
          <Card title={t('analytics.heatmap')} desc={t('analytics.heatmapDesc')} icon="grid">
            <Heatmap heat={d.heat} base={heatBase} />
          </Card>
        </div>
        <DetailTable groups={d.detail} />
      {/if}
    {/if}

    <footer class="about">
      <h3><Icon name="info" size={14} />{t('analytics.aboutTitle')}</h3>
      <p>{t('analytics.about')}</p>
      <hr />
      <h3 class="warn"><Icon name="warn" size={14} />{t('analytics.cautionTitle')}</h3>
      <ul>
        {#each [1, 2, 3, 4, 5, 6] as n (n)}<li><span>•</span>{t('analytics.c' + n)}</li>{/each}
      </ul>
    </footer>
  </div>
</div>

<style>
  .page {
    --grid: color-mix(in srgb, var(--fg) 12%, transparent);
    --card-bg: var(--control); --card-border: 1px solid transparent; --card-r: 10px;
    height: 100vh; overflow: auto;
  }
  :global(:root[data-skin="win"]) .page { --card-bg: var(--surface); --card-border: 1px solid var(--surface-stroke); --card-r: var(--r-card); }
  .inner { min-width: 604px; padding: 18px; display: flex; flex-direction: column; gap: 14px; }

  header { display: flex; flex-direction: column; gap: 14px; }
  .top { display: flex; align-items: baseline; gap: 12px; }
  h1 { margin: 0; font-size: 20px; font-weight: 700; flex: 1; }
  :global(:root[data-skin="win"]) h1 { font-size: 28px; font-weight: 600; font-family: "Segoe UI Variable Display", "Segoe UI", sans-serif; }
  .since { display: inline-flex; align-items: center; gap: 4px; font-size: var(--fs-small); color: var(--fg2); }
  .gear { display: inline-flex; align-items: center; gap: 5px; align-self: center; }
  .seg { display: flex; border: 1px solid var(--control-stroke); border-radius: var(--r-control); overflow: hidden; }
  .seg button { flex: 1; border: none; border-radius: 0; background: var(--control); min-height: 24px; padding: 2px 12px; }
  .seg button + button { border-left: 1px solid var(--control-stroke); }
  .seg button.on { background: var(--accent); color: var(--accent-fg); }

  .chips { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .lbl { font-size: var(--fs-small); color: var(--fg2); }
  .flash { font-size: 10px; color: var(--fg2); }
  .chip {
    display: inline-flex; align-items: center; gap: 5px; padding: 3px 10px; min-height: 0; border-radius: 999px;
    border: 1px solid transparent; background: color-mix(in srgb, var(--fg2) 10%, transparent); color: var(--fg2); font-size: var(--fs-small);
  }
  :global(:root[data-skin="win"]) .chip { min-height: 0; padding: 4px 12px; }
  .chip:hover:not(:disabled) { background: color-mix(in srgb, var(--fg2) 16%, transparent); }
  .chip .dot { width: 8px; height: 8px; border-radius: 50%; background: color-mix(in srgb, var(--fg2) 40%, transparent); }
  .chip.on { color: var(--fg); background: color-mix(in srgb, var(--c) 16%, transparent); border-color: color-mix(in srgb, var(--c) 50%, transparent); }
  .chip.on:hover { background: color-mix(in srgb, var(--c) 22%, transparent); }
  .chip.on .dot { background: var(--c); }
  .chip.off { opacity: 0.5; }

  .kpis { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; }
  .tile { background: var(--card-bg); border: var(--card-border); border-radius: var(--card-r); padding: 14px; min-width: 0; }
  .tt { display: flex; align-items: center; gap: 5px; font-size: var(--fs-small); color: var(--fg2); }
  .val { font-size: 22px; font-weight: 700; margin: 6px 0; white-space: nowrap; }
  .sub { font-size: 10px; color: var(--fg2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .cols { display: grid; gap: 14px; }
  .cols.a { grid-template-columns: 330px minmax(0, 1fr); }
  .cols.b { grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }
  @media (max-width: 860px) {
    .cols.a, .cols.b { grid-template-columns: minmax(0, 1fr); }
  }

  .empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 10px; min-height: 220px; color: var(--fg2); background: var(--card-bg); border: var(--card-border); border-radius: var(--card-r); }
  .empty p { margin: 0; text-align: center; white-space: pre-line; }

  .about { padding: 14px; background: color-mix(in srgb, var(--card-bg) 60%, transparent); border: var(--card-border); border-radius: var(--card-r); color: var(--fg2); font-size: var(--fs-small); }
  .about h3 { display: flex; align-items: center; gap: 6px; margin: 0 0 8px; font-size: 12px; font-weight: 700; color: var(--fg); }
  .about h3.warn { color: var(--warn); }
  .about p { margin: 0; }
  .about hr { border: none; border-top: 1px solid var(--grid); margin: 10px 0; }
  .about ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .about li { display: flex; gap: 6px; }
</style>
