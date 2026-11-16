<script lang="ts">
  // 분석 창 "토큰" 보기(계획서 8-3): 로컬 기록(Claude Code·Codex CLI·Antigravity)으로 본 실제 토큰 사용.
  // 차트 색은 공용 팔레트(LLM/tools/palette.py → src/lib/palette.json): 계열색은 순서대로, 6개를 넘으면 "기타".
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { ui, t } from '../lib/api.svelte';
  import palette from '../lib/palette.json';

  let { range }: { range: string } = $props();
  interface Tot { input: number; output: number; cacheRead: number; cacheWrite: number; reasoning: number; turns: number; total: number }
  interface Grp { key: string; sessions: number; totals: Tot }
  interface Data { totals: Tot; cacheHit: number | null; perTurn: number | null; sessions: number; byModel: Grp[]; byProject: Grp[]; byDayModel: Grp[]; weekdayHour: number[][]; codexLimits: { t: number; primary: number | null; secondary: number | null }[] }
  interface LL { asked: boolean; sources: { id: string; enabled: boolean }[]; scanning: boolean; events: number }

  let source = $state<string | null>(null);
  let d = $state<Data | null>(null);
  let ll = $state<LL | null>(null);
  async function load() {
    ll = (await invoke('ui_locallogs')) as LL;
    d = (await invoke('ui_tokens', { range, source })) as Data;
  }
  $effect(() => { void range; void source; load(); });
  onMount(() => {
    const un = listen('ui-state', () => load());
    const timer = setInterval(load, 60_000);
    return () => { un.then((f) => f()); clearInterval(timer); };
  });

  const nf = $derived(new Intl.NumberFormat(ui.s?.lang ?? 'ko', { notation: 'compact', maximumFractionDigits: 1 }));
  const fmt = (n: number) => nf.format(n);
  const days = $derived(({ day1: 1, day3: 3, week: 7, month: 30 } as Record<string, number>)[range] ?? 1);

  // 모델 → 색(팔레트 순서, 6개 초과는 "기타" 회색).
  const models = $derived((d?.byModel ?? []).slice().sort((a, b) => b.totals.total - a.totals.total).map((g) => g.key || '?'));
  const OTHER = 'other';
  const modelColor = (m: string) => { const i = models.indexOf(m); return i >= 0 && i < palette.order.length ? palette.order[i] : 'var(--fg3)'; };
  const modelKey = (m: string) => (models.indexOf(m) < palette.order.length ? m : OTHER);

  // 날짜별 모델 쌓기: byDayModel key = "YYYY-MM-DD|model".
  const daysData = $derived.by(() => {
    const map = new Map<string, Map<string, number>>();
    for (const g of d?.byDayModel ?? []) {
      const [day, model = '?'] = g.key.split('|');
      const k = modelKey(model || '?');
      const m = map.get(day) ?? new Map();
      m.set(k, (m.get(k) ?? 0) + g.totals.total);
      map.set(day, m);
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  });
  const dayMax = $derived(Math.max(1, ...daysData.map(([, m]) => [...m.values()].reduce((a, b) => a + b, 0))));

  const parts = $derived(d ? [
    { k: 'tokens.input', v: d.totals.input }, { k: 'tokens.output', v: d.totals.output },
    { k: 'tokens.cacheRead', v: d.totals.cacheRead }, { k: 'tokens.cacheWrite', v: d.totals.cacheWrite },
  ] : []);
  const partSum = $derived(Math.max(1, parts.reduce((a, p) => a + p.v, 0)));

  const heatMax = $derived(Math.max(1, ...(d?.weekdayHour ?? []).flat()));
  const ramp = palette.sequential;
  const heatColor = (v: number) => (v <= 0 ? 'var(--surface)' : ramp[Math.min(ramp.length - 1, Math.floor(Math.sqrt(v / heatMax) * (ramp.length - 1) + 0.5))]);
  const weekdayNames = $derived(Array.from({ length: 7 }, (_, i) => new Intl.DateTimeFormat(ui.s?.lang ?? 'ko', { weekday: 'short' }).format(new Date(2026, 10, 16 + i))));
  const projMax = $derived(Math.max(1, ...(d?.byProject ?? []).map((p) => p.totals.total)));
  const anyOn = $derived(ll?.sources.some((s) => s.enabled) ?? false);
</script>

<div class="chips row">
  {#each [null, 'claude', 'codex', 'agy'] as s (s ?? 'all')}
    <button class:on={source === s} onclick={() => (source = s)}>{s ? t('locallogs.source.' + s) : t('tokens.all')}</button>
  {/each}
</div>

{#if !anyOn}
  <p class="empty">{t('tokens.off')}</p>
{:else if !d || d.totals.total === 0}
  <p class="empty">{ll?.scanning ? t('tokens.scanning') : t('tokens.empty')}</p>
{:else}
  <div class="kpis">
    <div class="kpi"><span class="caption">{t('tokens.total')}</span><b>{fmt(d.totals.total)}</b></div>
    <div class="kpi"><span class="caption">{t('tokens.dailyAvg')}</span><b>{fmt(d.totals.total / days)}</b></div>
    <div class="kpi"><span class="caption">{t('tokens.cacheHit')}</span><b>{d.cacheHit == null ? '-' : Math.round(d.cacheHit * 100) + '%'}</b></div>
    <div class="kpi"><span class="caption">{t('tokens.sessions')}</span><b>{d.sessions}</b></div>
    <div class="kpi"><span class="caption">{t('tokens.perTurn')}</span><b>{d.perTurn == null ? '-' : fmt(d.perTurn)}</b></div>
  </div>

  <div class="cards">
    <section class="card wide">
      <h3>{t('tokens.byModel')}</h3>
      <svg viewBox="0 0 600 180" preserveAspectRatio="none" role="img" aria-label={t('tokens.byModel')}>
        {#each daysData as [day, m], i (day)}
          {@const w = 600 / Math.max(1, daysData.length)}
          {@const sorted = [...m.entries()].sort(([a], [b]) => (a === OTHER ? 1 : b === OTHER ? -1 : models.indexOf(a) - models.indexOf(b)))}
          {#each sorted as [k, v], j (k)}
            {@const below = sorted.slice(0, j).reduce((a, [, x]) => a + x, 0)}
            <rect x={i * w + w * 0.15} width={w * 0.7} y={170 - ((below + v) / dayMax) * 160} height={(v / dayMax) * 160} fill={k === OTHER ? 'var(--fg3)' : modelColor(k)}><title>{day} {k} {fmt(v)}</title></rect>
          {/each}
        {/each}
      </svg>
      <div class="legend">
        {#each models.slice(0, palette.order.length) as m (m)}<span><i style:background={modelColor(m)}></i>{m}</span>{/each}
        {#if models.length > palette.order.length}<span><i style:background="var(--fg3)"></i>{t('tokens.all') === 'All' ? 'Other' : '기타'}</span>{/if}
      </div>
    </section>

    <section class="card">
      <h3>{t('tokens.composition')}</h3>
      <div class="bar">
        {#each parts as p, i (p.k)}<span style:width={(p.v / partSum) * 100 + '%'} style:background={palette.order[i]} title={`${t(p.k)} ${fmt(p.v)}`}></span>{/each}
      </div>
      <ul class="parts">
        {#each parts as p, i (p.k)}<li><i style:background={palette.order[i]}></i>{t(p.k)} <b>{fmt(p.v)}</b> <span class="caption">{Math.round((p.v / partSum) * 100)}%</span></li>{/each}
        <li class="caption">{t('tokens.reasoning')} {fmt(d.totals.reasoning)}</li>
      </ul>
    </section>

    <section class="card">
      <h3>{t('tokens.projects')}</h3>
      <ul class="projects">
        {#each d.byProject as p (p.key)}
          <li><span class="pname">{p.key || '-'}</span><span class="pbar"><span style:width={(p.totals.total / projMax) * 100 + '%'}></span></span><b>{fmt(p.totals.total)}</b></li>
        {/each}
      </ul>
    </section>

    <section class="card wide">
      <h3>{t('tokens.heatmap')}</h3>
      <div class="heat">
        {#each d.weekdayHour as row, r (r)}
          <span class="wd caption">{weekdayNames[r]}</span>
          {#each row as v, h (h)}<span class="cell" style:background={heatColor(v)} title={`${weekdayNames[r]} ${h}:00 ${fmt(v)}`}></span>{/each}
        {/each}
      </div>
    </section>

    {#if d.codexLimits.length > 1 && (source === null || source === 'codex')}
      {@const t0 = d.codexLimits[0].t}
      {@const t1 = d.codexLimits[d.codexLimits.length - 1].t}
      <section class="card wide">
        <h3>{t('tokens.codexLimits')}</h3>
        <svg viewBox="0 0 600 120" preserveAspectRatio="none" role="img" aria-label={t('tokens.codexLimits')}>
          <polyline fill="none" stroke={palette.order[3]} stroke-width="2" vector-effect="non-scaling-stroke"
            points={d.codexLimits.filter((p) => p.primary != null).map((p) => `${((p.t - t0) / Math.max(1, t1 - t0)) * 600},${115 - (p.primary! / 100) * 110}`).join(' ')} />
          <polyline fill="none" stroke={palette.order[4]} stroke-width="2" vector-effect="non-scaling-stroke"
            points={d.codexLimits.filter((p) => p.secondary != null).map((p) => `${((p.t - t0) / Math.max(1, t1 - t0)) * 600},${115 - (p.secondary! / 100) * 110}`).join(' ')} />
        </svg>
        <div class="legend"><span><i style:background={palette.order[3]}></i>{t('menu.line5h')}</span><span><i style:background={palette.order[4]}></i>{t('menu.lineWeekly')}</span></div>
      </section>
    {/if}
  </div>
  <p class="caption">{t('tokens.note')}</p>
{/if}

<style>
  .chips { gap: 6px; margin: 4px 0 12px; flex-wrap: wrap; }
  .chips button { border-radius: 999px; padding: 2px 12px; min-height: 24px; }
  .chips button.on { background: var(--accent); color: var(--accent-fg); border-color: transparent; }
  .empty { color: var(--fg2); padding: 24px 0; }
  .kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(130px, 1fr)); gap: 10px; margin-bottom: 12px; }
  .kpi { background: var(--surface); border: 1px solid var(--surface-stroke); border-radius: var(--r-card); padding: 10px 12px; display: flex; flex-direction: column; gap: 4px; }
  .kpi b { font-size: 20px; }
  .cards { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .card { background: var(--surface); border: 1px solid var(--surface-stroke); border-radius: var(--r-card); padding: 12px 14px; min-width: 0; }
  .card.wide { grid-column: 1 / -1; }
  @media (max-width: 860px) { .cards { grid-template-columns: 1fr; } }
  h3 { margin: 0 0 8px; font-size: var(--fs); }
  svg { width: 100%; height: 180px; display: block; }
  .legend { display: flex; flex-wrap: wrap; gap: 10px; font-size: var(--fs-small); color: var(--fg2); margin-top: 6px; }
  .legend i, .parts i { display: inline-block; width: 9px; height: 9px; border-radius: 50%; margin-right: 5px; vertical-align: middle; }
  .bar { display: flex; height: 12px; border-radius: 6px; overflow: hidden; background: var(--control); }
  .parts { list-style: none; padding: 0; margin: 10px 0 0; display: flex; flex-direction: column; gap: 4px; }
  .projects { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 4px; }
  .projects li { display: grid; grid-template-columns: minmax(80px, 1fr) 2fr auto; gap: 8px; align-items: center; }
  .pname { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pbar { height: 8px; background: var(--control); border-radius: 4px; overflow: hidden; }
  .pbar span { display: block; height: 100%; background: var(--accent); }
  .heat { display: grid; grid-template-columns: 28px repeat(24, 1fr); gap: 2px; }
  .cell { height: 16px; border-radius: 2px; }
</style>
