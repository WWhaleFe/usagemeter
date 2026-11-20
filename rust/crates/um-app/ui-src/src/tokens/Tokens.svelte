<script lang="ts">
  // 분석 창 "토큰 기준" 보기(계획서 8-3): Claude Code·Codex CLI가 이 컴퓨터에 남긴 토큰 기록 그대로(추정치 없음).
  // 형식은 레퍼런스(OpenRouter Activity의 일별 모델 쌓은 막대, Anthropic Console 사용량, ccusage 일별 표)와
  // dataviz 규칙을 따른다: 시간 흐름 = 모델별 쌓은 세로 막대(가는 막대·2px 틈), 모델 비교 = 순위 가로 막대,
  // 구성·프로젝트 = 한 색 가로 막대, 요일×시간 = 단색 히트맵. 모델 색은 전체 기준 순서(modelOrder)로 고정한다.
  // 차트 색은 공용 팔레트(LLM/tools/palette.py → src/lib/palette.json).
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { ui, t, tn } from '../lib/api.svelte';
  import palette from '../lib/palette.json';
  import Card from '../analytics/Card.svelte';
  import ChartBox from '../analytics/ChartBox.svelte';
  import Segmented from '../components/Segmented.svelte';
  import { niceTicks, fmtMD } from '../analytics/util';

  let { range }: { range: string } = $props();
  interface Tot { input: number; output: number; cacheRead: number; cacheWrite: number; reasoning: number; turns: number; total: number }
  interface Grp { key: string; sessions: number; totals: Tot }
  interface Data {
    totals: Tot; cacheHit: number | null; perTurn: number | null; sessions: number; modelOrder: string[];
    byModel: Grp[]; byProject: Grp[]; byDayModel: Grp[]; byHourModel: Grp[];
    weekdayHour: number[][]; weekdayHourNoCache: number[][];
    codexLimits: { t: number; primary: number | null; secondary: number | null }[];
  }
  interface LL { asked: boolean; sources: { id: string; enabled: boolean }[]; scanning: boolean; events: number }

  let source = $state<string | null>(null);
  /** all = 모든 토큰, nocache = 캐시 읽기 제외(새로 처리한 양). */
  let kind = $state<'all' | 'nocache'>('all');
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
  /** 고른 종류의 합계. */
  const val = (x: Tot) => (kind === 'all' ? x.total : x.total - x.cacheRead);
  /** 비율 표기: 99% 이상은 소수 한 자리(반올림해 100%로 보이지 않게). */
  const pct1 = (r: number) => (r >= 0.99 && r < 1 ? (Math.floor(r * 1000) / 10).toFixed(1) + '%' : Math.round(r * 100) + '%');
  const share = (v: number, all: number) => { const p = (v / Math.max(1, all)) * 100; return p > 0 && p < 1 ? '<1%' : Math.round(p) + '%'; };

  // 모델 색: 전체 기준 순서로 팔레트 앞에서부터 고정, 6개를 넘으면 "기타"(회색).
  const OTHER = '__other';
  const order = $derived(d?.modelOrder ?? []);
  const modelKey = (m: string) => { const i = order.indexOf(m); return i >= 0 && i < palette.order.length ? m : OTHER; };
  const modelColor = (k: string) => (k === OTHER ? 'var(--fg3)' : palette.order[order.indexOf(k)] ?? 'var(--fg3)');
  const modelName = (k: string) => (k === OTHER ? t('tokens.other') : k || '?');

  // ---------- 추이(쌓은 세로 막대) ----------
  const pad = (n: number) => String(n).padStart(2, '0');
  const ymd = (x: Date) => `${x.getFullYear()}-${pad(x.getMonth() + 1)}-${pad(x.getDate())}`;
  interface Slot { key: string; at: Date }
  /** 1일 = 1시간 24칸, 3일 = 3시간 24칸, 주간 7칸, 한달 30칸(오늘·지금 칸까지). */
  const slots = $derived.by((): Slot[] => {
    const now = new Date();
    if (range === 'day1' || range === 'day3') {
      const step = range === 'day1' ? 1 : 3;
      const end = new Date(now.getFullYear(), now.getMonth(), now.getDate(), Math.floor(now.getHours() / step) * step);
      return Array.from({ length: 24 }, (_, i) => {
        const at = new Date(end.getTime() - (23 - i) * step * 3600_000);
        return { key: `${ymd(at)} ${pad(at.getHours())}`, at };
      });
    }
    const n = days;
    return Array.from({ length: n }, (_, i) => {
      const at = new Date(now.getFullYear(), now.getMonth(), now.getDate() - (n - 1 - i));
      return { key: ymd(at), at };
    });
  });
  const hourly = $derived(range === 'day1' || range === 'day3');
  /** 칸 키 -> (모델 키 -> 값). 3일은 시간 키를 3시간 칸으로 모은다. */
  const stacks = $derived.by(() => {
    const map = new Map<string, Map<string, number>>();
    for (const g of (hourly ? d?.byHourModel : d?.byDayModel) ?? []) {
      const [when, model = ''] = g.key.split('|');
      let k = when;
      if (range === 'day3') { const [dt, hh] = when.split(' '); k = `${dt} ${pad(Math.floor(+hh / 3) * 3)}`; }
      const mk = modelKey(model);
      const m = map.get(k) ?? new Map<string, number>();
      m.set(mk, (m.get(mk) ?? 0) + val(g.totals));
      map.set(k, m);
    }
    return map;
  });
  /** 쌓는 순서: 전체 기준 순서(많은 모델이 아래), 기타는 맨 위. */
  const stackKeys = $derived([...order.slice(0, palette.order.length), OTHER].filter((k) => [...stacks.values()].some((m) => (m.get(k) ?? 0) > 0)));
  const colTotals = $derived(slots.map((s) => [...(stacks.get(s.key)?.values() ?? [])].reduce((a, b) => a + b, 0)));
  const yScale = $derived(niceTicks(Math.max(0, ...colTotals)));
  const slotLabel = (s: Slot, i: number) => {
    if (range === 'day1') return s.at.getHours() % 6 === 0 ? tn('analytics.hourFmt', s.at.getHours()) : '';
    if (range === 'day3') return s.at.getHours() === 0 ? fmtMD(s.at) : '';
    if (range === 'week') return fmtMD(s.at);
    return i % 5 === 0 || i === slots.length - 1 ? fmtMD(s.at) : '';
  };
  const slotTitle = (s: Slot) => (hourly ? `${fmtMD(s.at)} ${pad(s.at.getHours())}:00` : fmtMD(s.at));
  /** 위쪽 모서리만 둥근 막대(데이터 끝 4px, 바닥은 각지게). */
  // 차트 여백: 왼쪽 4, 오른쪽 y 눈금 자리, 위 6, 아래 x 라벨 18.
  const ML = 4, MT = 6;
  const topRound = (x: number, y: number, w: number, h: number, r = 4) => {
    const rr = Math.min(r, w / 2, h);
    return `M${x} ${y + h}V${y + rr}Q${x} ${y} ${x + rr} ${y}H${x + w - rr}Q${x + w} ${y} ${x + w} ${y + rr}V${y + h}Z`;
  };

  // ---------- 모델 합계·구성·프로젝트 ----------
  const models = $derived((d?.byModel ?? []).map((g) => ({ key: g.key, v: val(g.totals) })).filter((m) => m.v > 0).sort((a, b) => b.v - a.v));
  const modelMax = $derived(Math.max(1, ...models.map((m) => m.v)));
  const sum = $derived(d ? val(d.totals) : 0);
  const parts = $derived(d ? [
    { k: 'tokens.cacheRead', v: d.totals.cacheRead, cache: true }, { k: 'tokens.input', v: d.totals.input, cache: false },
    { k: 'tokens.output', v: d.totals.output, cache: false }, { k: 'tokens.cacheWrite', v: d.totals.cacheWrite, cache: false },
  ].filter((p) => kind === 'all' || !p.cache).sort((a, b) => b.v - a.v) : []);
  const partMax = $derived(Math.max(1, ...parts.map((p) => p.v)));
  const projects = $derived((d?.byProject ?? []).map((p) => ({ key: p.key, v: val(p.totals) })).sort((a, b) => b.v - a.v));
  const projMax = $derived(Math.max(1, ...projects.map((p) => p.v)));

  // ---------- 요일 × 시간 ----------
  const heat = $derived((kind === 'all' ? d?.weekdayHour : d?.weekdayHourNoCache) ?? []);
  const heatMax = $derived(Math.max(1, ...heat.flat()));
  const ramp = palette.sequential;
  const heatColor = (v: number) => (v <= 0 ? 'var(--control)' : ramp[Math.min(ramp.length - 1, Math.floor(Math.sqrt(v / heatMax) * (ramp.length - 1) + 0.5))]);
  const weekdayNames = $derived(Array.from({ length: 7 }, (_, i) => new Intl.DateTimeFormat(ui.s?.lang ?? 'ko', { weekday: 'short' }).format(new Date(2026, 10, 16 + i))));
  const anyOn = $derived(ll?.sources.some((s) => s.enabled) ?? false);
  const kinds = $derived([{ value: 'all', label: t('tokens.kindAll') }, { value: 'nocache', label: t('tokens.kindNoCache') }]);
</script>

<!-- 거르기는 한 줄, 모든 차트 위(소스·토큰 종류). -->
<div class="filters">
  <div class="chips">
    {#each [null, 'claude', 'codex'] as s (s ?? 'all')}
      <button class:on={source === s} onclick={() => (source = s)}>{s ? t('locallogs.source.' + s) : t('tokens.all')}</button>
    {/each}
  </div>
  <div class="kind"><Segmented options={kinds} value={kind} onchange={(v) => (kind = v)} /></div>
</div>
<p class="notice caption"><span aria-hidden="true">ⓘ</span>{t('tokens.geminiNo')}</p>
{#if kind === 'nocache'}<p class="notice caption"><span aria-hidden="true">ⓘ</span>{t('tokens.kindDesc')}</p>{/if}

{#if !anyOn}
  <p class="empty">{t('tokens.off')}</p>
{:else if !d || d.totals.total === 0}
  <p class="empty">{ll?.scanning ? t('tokens.scanning') : t('tokens.empty')}</p>
{:else}
  <div class="kpis">
    <div class="tile"><span class="tt">{t('tokens.total')}</span><b>{fmt(sum)}</b></div>
    <div class="tile"><span class="tt">{t('tokens.dailyAvg')}</span><b>{fmt(sum / days)}</b></div>
    <div class="tile"><span class="tt">{t('tokens.cacheHit')}</span><b>{d.cacheHit == null ? '-' : pct1(d.cacheHit)}</b></div>
    <div class="tile"><span class="tt">{t('tokens.sessions')}</span><b>{d.sessions}</b></div>
    <div class="tile"><span class="tt">{t('tokens.perTurn')}</span><b>{d.totals.turns > 0 ? fmt(sum / d.totals.turns) : '-'}</b></div>
  </div>

  <Card title={t('tokens.byModel')} desc={t('tokens.trendDesc')} icon="bars2" height={290}>
    <ChartBox>
      {#snippet children(w: number, h: number)}
        {@const pw = w - 4 - 44}
        {@const ph = h - 18 - 6}
        {@const slot = pw / slots.length}
        {@const bw = Math.max(2, Math.min(24, slot * 0.62))}
        <svg width={w} height={h} role="img" aria-label={t('tokens.byModel')}>
          {#each yScale.ticks as v (v)}
            {@const y = MT + ph * (1 - v / yScale.top)}
            <line x1={ML} x2={ML + pw} y1={y} y2={y} class="grid" />
            <text x={ML + pw + 6} {y} dominant-baseline="middle">{fmt(v)}</text>
          {/each}
          {#each slots as s, i (s.key)}
            {@const x = ML + slot * i + (slot - bw) / 2}
            {@const m = stacks.get(s.key)}
            {@const segs = stackKeys.map((k) => ({ k, v: m?.get(k) ?? 0 })).filter((g) => g.v > 0)}
            <g class="col">
              <rect class="hit" x={ML + slot * i} y={MT} width={slot} height={ph}><title>{slotTitle(s)} · {fmt(colTotals[i])}</title></rect>
              {#each segs as g, j (g.k)}
                {@const below = segs.slice(0, j).reduce((a, b) => a + b.v, 0)}
                {@const y1 = MT + ph * (1 - below / yScale.top)}
                {@const y2 = MT + ph * (1 - (below + g.v) / yScale.top)}
                <!-- 쌓은 칸 사이 2px 틈(아래 칸 위쪽을 2px 비운다). 맨 위 칸만 위 모서리를 둥글게. -->
                {@const hh = Math.max(1, y1 - y2 - (j > 0 ? 2 : 0))}
                {#if j === segs.length - 1}
                  <path d={topRound(x, y2, bw, hh)} fill={modelColor(g.k)}><title>{slotTitle(s)} · {modelName(g.k)} {fmt(g.v)}</title></path>
                {:else}
                  <rect {x} y={y2 + (y1 - y2 - hh)} width={bw} height={hh} fill={modelColor(g.k)}><title>{slotTitle(s)} · {modelName(g.k)} {fmt(g.v)}</title></rect>
                {/if}
              {/each}
            </g>
            {@const lb = slotLabel(s, i)}
            {#if lb}<text x={ML + slot * (i + 0.5)} y={h - 4} text-anchor="middle">{lb}</text>{/if}
          {/each}
          <line x1={ML} x2={ML + pw} y1={MT + ph} y2={MT + ph} class="axis" />
        </svg>
      {/snippet}
    </ChartBox>
    <div class="legend">
      {#each stackKeys as k (k)}<span><i class="sw" style:background={modelColor(k)}></i>{modelName(k)}</span>{/each}
    </div>
  </Card>

  <div class="cols">
    <Card title={t('tokens.modelTotal')} desc={t('tokens.modelTotalDesc')} icon="bars" height={null}>
      <ul class="ranked">
        {#each models as m (m.key)}
          <li>
            <span class="name" title={m.key || '?'}><i class="sw" style:background={modelColor(modelKey(m.key))}></i>{m.key || '?'}</span>
            <span class="track"><span class="fill" style:width={(m.v / modelMax) * 100 + '%'} style:background={modelColor(modelKey(m.key))}></span></span>
            <span class="num">{fmt(m.v)}</span>
            <span class="num pct">{share(m.v, sum)}</span>
          </li>
        {/each}
      </ul>
      <p class="caption foot">{t('tokens.noLimit')}</p>
    </Card>

    <Card title={t('tokens.composition')} desc={t('tokens.compositionDesc')} icon="pie" height={null}>
      <ul class="ranked">
        {#each parts as p (p.k)}
          <li>
            <span class="name">{t(p.k)}</span>
            <span class="track"><span class="fill neutral" style:width={(p.v / partMax) * 100 + '%'}></span></span>
            <span class="num">{fmt(p.v)}</span>
            <span class="num pct">{share(p.v, sum)}</span>
          </li>
        {/each}
      </ul>
      <p class="caption foot">{t('tokens.reasoning')} {fmt(d.totals.reasoning)}</p>
    </Card>
  </div>

  <div class="cols">
    <Card title={t('tokens.heatmap')} desc={t('tokens.heatmapDesc')} icon="grid" height={null}>
      <div class="heat">
        {#each heat as row, r (r)}
          <span class="wd">{weekdayNames[r]}</span>
          {#each row as v, hr (hr)}<span class="cell" style:background={heatColor(v)} title={`${weekdayNames[r]} ${tn('analytics.hourFmt', hr)} · ${fmt(v)}`}></span>{/each}
        {/each}
        <span></span>
        {#each Array.from({ length: 24 }, (_, i) => i) as hr (hr)}<span class="hx">{hr % 6 === 0 ? tn('analytics.hourFmt', hr) : ''}</span>{/each}
      </div>
      <div class="scale"><span>{t('tokens.less')}</span>{#each ramp as c (c)}<i style:background={c}></i>{/each}<span>{t('tokens.more')}</span></div>
    </Card>

    <Card title={t('tokens.projects')} desc={t('tokens.projectsDesc')} icon="table" height={null}>
      <ul class="ranked">
        {#each projects as p (p.key)}
          <li>
            <span class="name" title={p.key || '-'}>{p.key || '-'}</span>
            <span class="track"><span class="fill neutral" style:width={(p.v / projMax) * 100 + '%'}></span></span>
            <span class="num">{fmt(p.v)}</span>
            <span class="num pct">{share(p.v, sum)}</span>
          </li>
        {/each}
      </ul>
    </Card>
  </div>

  {#if d.codexLimits.length > 1 && (source === null || source === 'codex')}
    {@const t0 = d.codexLimits[0].t}
    {@const t1 = d.codexLimits[d.codexLimits.length - 1].t}
    {@const lim = d.codexLimits}
    <Card title={t('tokens.codexLimits')} desc={t('tokens.codexLimitsDesc')} icon="line" height={220}>
      <ChartBox>
        {#snippet children(w: number, h: number)}
          {@const pw = w - 4 - 40}
          {@const ph = h - 18 - 6}
          {@const tx = (tt: number) => ML + ((tt - t0) / Math.max(1, t1 - t0)) * pw}
          {@const ty = (v: number) => MT + ph * (1 - v / 100)}
          <svg width={w} height={h} role="img" aria-label={t('tokens.codexLimits')}>
            {#each [0, 50, 100] as v (v)}
              <line x1={ML} x2={ML + pw} y1={ty(v)} y2={ty(v)} class="grid" />
              <text x={ML + pw + 6} y={ty(v)} dominant-baseline="middle">{v}%</text>
            {/each}
            {#each [0, 0.5, 1] as f (f)}
              {@const tt = t0 + (t1 - t0) * f}
              <text x={tx(tt)} y={h - 4} text-anchor={f === 0 ? 'start' : f === 1 ? 'end' : 'middle'}>{fmtMD(new Date(tt * 1000))} {pad(new Date(tt * 1000).getHours())}:00</text>
            {/each}
            <polyline class="ln" stroke={palette.order[3]} points={lim.filter((p) => p.primary != null).map((p) => `${tx(p.t)},${ty(p.primary!)}`).join(' ')} />
            <polyline class="ln" stroke={palette.order[4]} points={lim.filter((p) => p.secondary != null).map((p) => `${tx(p.t)},${ty(p.secondary!)}`).join(' ')} />
          </svg>
        {/snippet}
      </ChartBox>
      <div class="legend"><span><i class="ln-key" style:background={palette.order[3]}></i>{t('menu.line5h')}</span><span><i class="ln-key" style:background={palette.order[4]}></i>{t('menu.lineWeekly')}</span></div>
    </Card>
  {/if}
  <p class="caption">{t('tokens.note')}</p>
{/if}

<style>
  .filters { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .chips { display: flex; gap: 8px; flex-wrap: wrap; }
  .chips button { display: inline-flex; align-items: center; gap: 5px; min-height: 0; padding: 3px 10px; border-radius: 999px; font-size: var(--fs-small); background: color-mix(in srgb, var(--fg2) 10%, transparent); border: 1px solid transparent; color: var(--fg2); box-shadow: none; }
  .chips button::before { content: ''; width: 8px; height: 8px; border-radius: 50%; background: color-mix(in srgb, var(--fg2) 40%, transparent); }
  .chips button:hover { background: color-mix(in srgb, var(--fg2) 16%, transparent); }
  .chips button.on { color: var(--fg); background: color-mix(in srgb, var(--accent) 16%, transparent); border-color: color-mix(in srgb, var(--accent) 50%, transparent); }
  .chips button.on::before { background: var(--accent); }
  .kind { width: 220px; }
  .notice { display: flex; gap: 6px; align-items: baseline; margin: 0; }
  .empty { min-height: 220px; display: grid; place-items: center; text-align: center; white-space: pre-line; margin: 0; padding: 24px; color: var(--fg2); background: var(--card-bg); border: var(--card-border); border-radius: var(--card-r); }

  .kpis { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 12px; }
  .tile { background: var(--card-bg); border: var(--card-border); border-radius: var(--card-r); padding: 14px; display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .tt { font-size: var(--fs-small); color: var(--fg2); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .tile b { font-size: 22px; line-height: 26px; font-weight: 700; }

  .cols { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 14px; align-items: start; }
  @media (max-width: 860px) { .cols { grid-template-columns: minmax(0, 1fr); } .kpis { grid-template-columns: repeat(3, minmax(0, 1fr)); } }

  svg :global(.grid) { stroke: var(--grid); stroke-width: 1; }
  svg :global(.axis) { stroke: color-mix(in srgb, var(--fg) 25%, transparent); stroke-width: 1; }
  svg :global(.hit) { fill: transparent; }
  .col:hover :global(.hit) { fill: color-mix(in srgb, var(--fg) 6%, transparent); }
  .ln { fill: none; stroke-width: 2; stroke-linejoin: round; stroke-linecap: round; }

  .legend { display: flex; flex-wrap: wrap; gap: 12px; font-size: var(--fs-small); color: var(--fg2); }
  .legend span { display: inline-flex; align-items: center; gap: 5px; }
  .sw { display: inline-block; width: 9px; height: 9px; border-radius: 2px; flex: none; }
  .ln-key { display: inline-block; width: 12px; height: 2px; border-radius: 1px; }

  /* 순위 가로 막대: 이름 · 막대(가는 8px) · 값 · 비율. 값은 열로 맞춘다. */
  .ranked { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 8px; }
  .ranked li { display: grid; grid-template-columns: minmax(90px, 38%) minmax(40px, 1fr) 56px 36px; gap: 10px; align-items: center; font-size: var(--fs-small); }
  .name { display: flex; align-items: center; gap: 6px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track { height: 8px; border-radius: 4px; background: color-mix(in srgb, var(--fg) 8%, transparent); overflow: hidden; }
  .fill { display: block; height: 100%; min-width: 2px; border-radius: 0 4px 4px 0; }
  .fill.neutral { background: #94a3b8; }
  .num { text-align: right; font-variant-numeric: tabular-nums; color: var(--fg); }
  .pct { color: var(--fg2); }
  .foot { margin: 6px 0 0; }

  .heat { display: grid; grid-template-columns: 26px repeat(24, minmax(0, 1fr)); gap: 2px; align-items: center; }
  .wd, .hx { font-size: 10px; color: var(--fg2); }
  .hx { white-space: nowrap; overflow: visible; }
  .cell { height: 14px; border-radius: 2px; }
  .scale { display: flex; align-items: center; gap: 3px; justify-content: flex-end; font-size: 10px; color: var(--fg2); }
  .scale i { width: 14px; height: 8px; border-radius: 2px; }
  .scale span { margin: 0 3px; }
</style>
