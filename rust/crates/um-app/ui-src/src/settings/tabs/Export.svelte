<script lang="ts">
  // 내보내기 탭: 지금 내보내기(분석 창 버튼과 같은 기능)와 자동 저장 설정. 설정은 Rust `export.json`.
  // 두 묶음이 같은 라벨 열(--label)과 같은 컨트롤 폭을 써서 줄이 맞는다.
  import { t, tf, tn } from '../../lib/api.svelte';
  import { ex, startExport, setExport, pickAutoFolder, exportNow, fmtWhen, lastOf, reveal } from '../../lib/export.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import ExportResult from '../../components/ExportResult.svelte';
  import Icon from '../../analytics/Icon.svelte';
  import NumField from '../../components/NumField.svelte';
  startExport();
  const c = $derived(ex.s?.settings);
  const ranges = ['day1', 'day3', 'week', 'month', 'all'];
  const rangeLabel = (r: string) => t(r === 'all' ? 'export.rangeAll' : 'range.' + r);
  const formats = [['both', 'export.fmtBoth'], ['csv', 'export.fmtCsv'], ['json', 'export.fmtJson']];
  const files = [
    ['summary.csv', 'export.fSummary', 'csv'], ['remaining.csv', 'export.fRemaining', 'csv'],
    ['usage_events.csv', 'export.fEvents', 'csv'], ['daily.csv', 'export.fDaily', 'csv'],
    ['hourly.csv', 'export.fHourly', 'csv'], ['analytics.json', 'export.fJson', 'json'],
  ];
  // 고른 형식에 들어가는 파일만 보여 준다.
  const shown = $derived(files.filter(([, , kind]) => c?.format === 'both' || c?.format === kind));
  // "폴더 열기"는 마지막으로 저장에 성공한 폴더를 연다.
  const okPath = (auto: boolean) => { const r = lastOf(auto); return r?.ok ? r.path : null; };
  const nowPath = $derived(okPath(false));
  const autoPath = $derived(okPath(true));
  const val = (e: Event) => (e.currentTarget as HTMLSelectElement).value;
</script>

{#if c}
  <Section title={t('export.nowTitle')}>
    <p class="caption desc">{t('export.nowDesc')}</p>
    <div class="form">
      <span class="label">{t('export.range')}</span>
      <div class="ctl">
        <select class="w" value={c.range} onchange={(e) => setExport({ range: val(e) as typeof c.range })}>
          {#each ranges as r (r)}<option value={r}>{rangeLabel(r)}</option>{/each}
        </select>
        <button class="primary act" disabled={ex.busy} onclick={exportNow}><Icon name="export" />{t('export.button')}</button>
      </div>

      <span class="label">{t('export.format')}</span>
      <div class="ctl">
        <select class="w" value={c.format} onchange={(e) => setExport({ format: val(e) as typeof c.format })}>
          {#each formats as [v, k] (v)}<option value={v}>{t(k)}</option>{/each}
        </select>
        <button class="act" disabled={!nowPath} onclick={() => nowPath && reveal(nowPath)}><Icon name="folder" />{t('export.open')}</button>
      </div>

      <span class="label top">{t('export.filesLabel')}</span>
      <ul class="files">
        {#each shown as [name, key] (name)}<li><code>{name}</code><span class="caption">{t(key)}</span></li>{/each}
      </ul>
    </div>
    <div class="indent"><ExportResult button={false} /></div>
  </Section>

  <Section title={t('export.autoTitle')}>
    <Toggle label={t('export.autoEnable')} checked={c.auto} onchange={(v) => setExport({ auto: v })} />
    <div class="form" class:off={!c.auto}>
      <span class="label">{t('export.folder')}</span>
      <div class="ctl">
        <button class="act" disabled={!c.auto} onclick={pickAutoFolder}><Icon name="folder" />{t('export.choose')}</button>
        <button class="act" disabled={!autoPath} onclick={() => autoPath && reveal(autoPath)}><Icon name="folder" />{t('export.open')}</button>
        <span class="folder caption" title={c.autoFolder ?? ''}>{c.autoFolder ?? t('export.noFolder')}</span>
      </div>

      <span class="label">{t('export.period')}</span>
      <div class="ctl">
        <select class="w" value={c.period} disabled={!c.auto} onchange={(e) => setExport({ period: val(e) as typeof c.period })}>
          {#each ['daily', 'weekly', 'monthly'] as p (p)}<option value={p}>{t('export.' + p)}</option>{/each}
        </select>
        <!-- 요일·날짜 칸은 매일일 때도 자리를 비워 둬서 "시각" 위치가 주기에 따라 움직이지 않는다. -->
        <span class="slot">
          {#if c.period === 'weekly'}
            <select class="w2" aria-label={t('export.weekday')} value={c.weekday} disabled={!c.auto} onchange={(e) => setExport({ weekday: +val(e) })}>
              {#each [1, 2, 3, 4, 5, 6, 7] as d (d)}<option value={d}>{t('export.wd' + d)}</option>{/each}
            </select>
          {:else if c.period === 'monthly'}
            <select class="w2" aria-label={t('export.monthday')} value={c.monthday} disabled={!c.auto} onchange={(e) => setExport({ monthday: +val(e) })}>
              {#each Array.from({ length: 28 }, (_, i) => i + 1) as d (d)}<option value={d}>{tn('export.dayFmt', d)}</option>{/each}
            </select>
          {/if}
        </span>
        <span class="label2">{t('export.time')}</span>
        <NumField label={t('export.hour')} value={c.hour} min={0} max={23} disabled={!c.auto} onchange={(v) => setExport({ hour: v })} />
        <span class="colon">:</span>
        <NumField label={t('export.minute')} value={c.minute} min={0} max={59} disabled={!c.auto} onchange={(v) => setExport({ minute: v })} />
      </div>

      {#if c.auto}
        <span></span>
        <span class="caption">{c.autoFolder ? (ex.s?.nextDue ? tf('export.next', fmtWhen(ex.s.nextDue)) : '') : t('export.needFolder')}</span>
      {/if}
    </div>
    <p class="caption desc">{t('export.autoDesc')}</p>
    <div class="indent"><ExportResult auto button={false} /></div>
  </Section>
{/if}

<style>
  /* 라벨 열 폭: 세 언어의 가장 긴 라벨("ファイル形式", "File format")이 한 줄에 들어가는 값. */
  .form { --label: 88px; display: grid; grid-template-columns: var(--label) minmax(0, 1fr); column-gap: 12px; row-gap: 8px; align-items: center; }
  .label { white-space: nowrap; }
  .label.top { align-self: start; padding-top: 1px; }
  .ctl { display: flex; align-items: center; gap: 8px; min-width: 0; }
  /* 드롭다운은 내용에 맞는 고정 폭(가로로 늘이지 않는다). */
  .ctl select { flex: none; }
  .w { width: 150px; }
  .w2 { width: 110px; }
  .slot { width: 110px; flex: none; display: flex; }
  .label2 { margin-left: 16px; white-space: nowrap; }
  .colon { color: var(--fg2); margin: 0 -2px; }
  .folder { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ctl button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; flex: none; }
  /* 드롭다운 바로 옆 버튼들은 같은 폭이라 위아래 줄이 맞는다. */
  .act { min-width: 110px; }
  /* 파일 목록: 3열 x 2줄. 칸마다 파일명 아래에 짧은 설명. */
  .files { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 6px 16px; }
  .files li { display: flex; flex-direction: column; min-width: 0; }
  .files code { font: 11px/15px ui-monospace, Menlo, Consolas, monospace; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .files .caption { line-height: 15px; }
  .indent { margin-left: calc(88px + 12px); }
  .desc { margin: 0; }
  .off { opacity: .45; }
</style>
