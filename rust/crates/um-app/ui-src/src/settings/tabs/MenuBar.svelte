<script lang="ts">
  // 메뉴바 아이콘·호버·클릭 동작과 드롭다운 정보. 모두 전역(main).
  import { ui, settingsProviders, t, tf, tn, set } from '../../lib/api.svelte';
  import PlatformFeature from '../../components/PlatformFeature.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  const s = $derived(ui.s!);
  const m = $derived(s.main);
  const providers = $derived(settingsProviders(s));
  const percentIDs = $derived<string[]>(m.menuBarPercentIDs ?? []);
  const chartOn = $derived(m.menuShowChart ?? false);
  function member<T>(list: T[], v: T, on: boolean): T[] {
    return on ? (list.includes(v) ? list : [...list, v]) : list.filter((x) => x !== v);
  }
  const infoRows: [string, string][] = [
    ['info.5h', 'menuShow5h'], ['info.weekly', 'menuShowWeekly'], ['info.opus', 'menuShowOpus'], ['info.reset', 'menuShowReset'],
    ['info.countdown', 'menuShowCountdown'], ['info.pace', 'menuShowPace'], ['info.chart', 'menuShowChart'], ['info.updated', 'menuShowUpdated'],
  ];
  // 기본값(Swift :471-482): 5h·주간·Opus·리셋·카운트다운은 켜짐, 나머지는 꺼짐.
  const infoDefault: Record<string, boolean> = { menuShow5h: true, menuShowWeekly: true, menuShowOpus: true, menuShowReset: true, menuShowCountdown: true };
</script>

<Section title={t('sec.menuIcon')}>
  <div class="row">
    <label class="grow" for="icon-ai">{t('disp.iconBaseAI')}</label>
    <select id="icon-ai" value={m.menuBarIconID ?? s.providers[0]?.id} onchange={(e) => set('main', { menuBarIconID: e.currentTarget.value })}>
      {#each providers as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
    </select>
  </div>
  <span class="caption">{t('disp.iconDesc')}</span>
  <PlatformFeature>
  <Toggle label={t('menuHover.enable')} checked={m.menuBarHoverInfoEnabled ?? true} onchange={(v) => set('main', { menuBarHoverInfoEnabled: v })} />
  <span class="caption">{t('menuHover.enableDesc')}</span>
  </PlatformFeature>
</Section>

<PlatformFeature>
<Section title={t('tray.clickMode')}>
  <select aria-label={t('tray.clickMode')} value={m.trayClickMode ?? 'split'} onchange={(e) => set('main', { trayClickMode: e.currentTarget.value })}>
    {#each ['split', 'swapped', 'together'] as mode (mode)}<option value={mode}>{t('tray.' + mode)}</option>{/each}
  </select>
</Section>
</PlatformFeature>

{#if s.platform === 'mac'}
<Section title={t('sec.menuPercent')}>
  {#each providers as p (p.id)}
    <Toggle label={tf('disp.showPercent', p.name)} checked={percentIDs.includes(p.id)} onchange={(v) => set('main', { menuBarPercentIDs: member(percentIDs, p.id, v) })} />
  {/each}
  <span class="caption">{t('disp.percentDesc')}</span>
</Section>
{/if}

<Section title={t('sec.dropdownInfo')}>
  <div class="grid">
    {#each infoRows as [key, field] (field)}
      <Toggle label={t(key)} checked={m[field] ?? infoDefault[field] ?? false} onchange={(v) => set('main', { [field]: v })} />
    {/each}
  </div>
  <div class="row" class:off={!chartOn}>
    <label class="grow" for="chart-hours">{t('chart.range')}</label>
    <select id="chart-hours" value={m.chartHours ?? 24} disabled={!chartOn} onchange={(e) => set('main', { chartHours: +e.currentTarget.value })}>
      {#each [6, 12, 24] as h (h)}<option value={h}>{tn('chart.hoursFmt', h)}</option>{/each}
    </select>
  </div>
  <Toggle label={t('chartHover.enable')} checked={m.chartHoverEnabled ?? true} disabled={!chartOn} onchange={(v) => set('main', { chartHoverEnabled: v })} />
</Section>


<style>
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 24px; }
  :global(:root[data-skin="macos"]) .grid { gap: 6px 10px; }
  :global(:root[data-skin="macos"]) .row.off { opacity: .45; }
</style>
