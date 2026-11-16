<script lang="ts">
  // 표시 탭(Swift SettingsView.swift:297-398): 언어, 외관, AI 순서, 호버, 메뉴바 아이콘·%, 드롭다운 정보, 차트, 알림. 모두 전역(main).
  import { ui, t, tf, tn, act, set } from '../../lib/api.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import Slider from '../../components/Slider.svelte';
  import Segmented from '../../components/Segmented.svelte';
  const s = $derived(ui.s!);
  const m = $derived(s.main);
  const order = $derived<string[]>(s.providers.map((p) => p.id));
  const hoverOn = $derived(m.hoverInfoEnabled ?? true);
  const edges = $derived<string[]>(m.hoverEdges ?? ['top', 'right', 'bottom', 'left']);
  const corners = $derived<string[]>(m.hoverCorners ?? []);
  const percentIDs = $derived<string[]>(m.menuBarPercentIDs ?? []);
  const thresholds = $derived<number[]>(m.notifyThresholds ?? [75, 90, 95]);
  const chartOn = $derived(m.menuShowChart ?? false);
  const notifyOn = $derived(m.notifyEnabled ?? false);

  // 배열 설정에서 원소 하나를 넣고 빼기(Swift memberBinding).
  function member<T>(list: T[], v: T, on: boolean): T[] {
    return on ? (list.includes(v) ? list : [...list, v]) : list.filter((x) => x !== v);
  }
  // Swift moveProviderUp/Down: 이웃과 자리 바꾸기.
  function move(i: number, d: -1 | 1) {
    const o = [...order];
    [o[i], o[i + d]] = [o[i + d], o[i]];
    set('main', { providerOrder: o });
  }

  const langs = [{ value: 'ko', label: '한국어' }, { value: 'en', label: 'English' }, { value: 'ja', label: '日本語' }];
  const appearances = $derived(['system', 'light', 'dark'].map((v) => ({ value: v, label: t('appearance.' + v) })));
  const edgeKeys = ['top', 'right', 'bottom', 'left'];
  const cornerKeys = ['topLeft', 'topRight', 'bottomLeft', 'bottomRight'];
  const infoRows: [string, string][] = [
    ['info.5h', 'menuShow5h'], ['info.weekly', 'menuShowWeekly'], ['info.opus', 'menuShowOpus'], ['info.reset', 'menuShowReset'],
    ['info.countdown', 'menuShowCountdown'], ['info.pace', 'menuShowPace'], ['info.chart', 'menuShowChart'], ['info.updated', 'menuShowUpdated'],
  ];
  // 기본값(Swift :471-482): 5h·주간·Opus·리셋·카운트다운은 켜짐, 나머지는 꺼짐.
  const infoDefault: Record<string, boolean> = { menuShow5h: true, menuShowWeekly: true, menuShowOpus: true, menuShowReset: true, menuShowCountdown: true };
  import LocalLogsSection from '../LocalLogsSection.svelte';
</script>

<Section title={t('lang.title')}>
  <Segmented options={langs} value={s.lang} onchange={(v) => act('lang', { code: v })} />
</Section>

<Section title={t('appearance.title')}>
  <Segmented options={appearances} value={m.appearance ?? 'system'} onchange={(v) => set('main', { appearance: v })} />
</Section>

<Section title={t('sec.providerOrder')}>
  {#each order as id, i (id)}
    <div class="row">
      <span class="grow">{i + 1}. {s.providers.find((p) => p.id === id)?.name}</span>
      <button class="icon" aria-label={t('order.up')} title={t('order.up')} disabled={i === 0} onclick={() => move(i, -1)}>&#9650;</button>
      <button class="icon" aria-label={t('order.down')} title={t('order.down')} disabled={i === order.length - 1} onclick={() => move(i, 1)}>&#9660;</button>
    </div>
  {/each}
  <span class="caption">{t('order.desc')}</span>
</Section>

<Section title={t('sec.hover')}>
  <Toggle label={t('hover.enable')} checked={hoverOn} onchange={(v) => set('main', { hoverInfoEnabled: v })} />
  <span class="caption">{t('hover.enableDesc')}</span>
  <fieldset class="grp" disabled={!hoverOn}>
    <legend>{t('hoverArea.edges')}</legend>
    <div class="checks">
      {#each edgeKeys as e (e)}
        <label class="chk"><input type="checkbox" checked={edges.includes(e)} onchange={(ev) => set('main', { hoverEdges: member(edges, e, ev.currentTarget.checked) })} />{t('edge.' + e)}</label>
      {/each}
    </div>
    <legend>{t('hoverArea.corners')}</legend>
    <div class="checks">
      {#each cornerKeys as c (c)}
        <label class="chk"><input type="checkbox" checked={corners.includes(c)} onchange={(ev) => set('main', { hoverCorners: member(corners, c, ev.currentTarget.checked) })} />{t('corner.' + c)}</label>
      {/each}
    </div>
    <Slider label={t('hoverArea.cornerSize')} value={m.hoverCornerSize ?? 80} min={20} max={300} step={5} disabled={!hoverOn || corners.length === 0} onchange={(v) => set('main', { hoverCornerSize: v })} />
    <span class="caption">{t('hoverArea.desc')}</span>
  </fieldset>
  <Toggle label={t('menuHover.enable')} checked={m.menuBarHoverInfoEnabled ?? true} onchange={(v) => set('main', { menuBarHoverInfoEnabled: v })} />
  <span class="caption">{t('menuHover.enableDesc')}</span>
</Section>

<Section title={t('sec.menuIcon')}>
  <div class="row">
    <label class="grow" for="icon-ai">{t('disp.iconBaseAI')}</label>
    <select id="icon-ai" value={m.menuBarIconID ?? s.providers[0]?.id} onchange={(e) => set('main', { menuBarIconID: e.currentTarget.value })}>
      {#each s.providers as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
    </select>
  </div>
  <span class="caption">{t('disp.iconDesc')}</span>
</Section>

<Section title={t('sec.menuPercent')}>
  {#each s.providers as p (p.id)}
    <Toggle label={tf('disp.showPercent', p.name)} checked={percentIDs.includes(p.id)} onchange={(v) => set('main', { menuBarPercentIDs: member(percentIDs, p.id, v) })} />
  {/each}
  <span class="caption">{t('disp.percentDesc')}</span>
</Section>

<Section title={t('sec.dropdownInfo')}>
  <div class="grid">
    {#each infoRows as [key, field] (field)}
      <Toggle label={t(key)} checked={m[field] ?? infoDefault[field] ?? false} onchange={(v) => set('main', { [field]: v })} />
    {/each}
  </div>
  <div class="row">
    <label class="grow" for="chart-hours">{t('chart.range')}</label>
    <select id="chart-hours" value={m.chartHours ?? 24} disabled={!chartOn} onchange={(e) => set('main', { chartHours: +e.currentTarget.value })}>
      {#each [6, 12, 24] as h (h)}<option value={h}>{tn('chart.hoursFmt', h)}</option>{/each}
    </select>
  </div>
  <Toggle label={t('chartHover.enable')} checked={m.chartHoverEnabled ?? true} disabled={!chartOn} onchange={(v) => set('main', { chartHoverEnabled: v })} />
</Section>

<Section title={t('sec.notify')}>
  <Toggle label={t('notify.enable')} checked={notifyOn} onchange={(v) => set('main', { notifyEnabled: v })} />
  <div class="checks" class:off={!notifyOn}>
    {#each [75, 90, 95] as th (th)}
      <label class="chk"><input type="checkbox" checked={thresholds.includes(th)} disabled={!notifyOn} onchange={(ev) => set('main', { notifyThresholds: member(thresholds, th, ev.currentTarget.checked).sort((a, b) => a - b) })} />{tn('notify.thFmt', th)}</label>
    {/each}
  </div>
  <span class="caption">{t('notify.desc')}</span>
</Section>

<LocalLogsSection />

<style>
  .icon { min-width: 28px; padding: 2px 8px; font-size: 10px; }
  .grp { border: none; margin: 0; padding: 0; min-width: 0; display: flex; flex-direction: column; gap: 6px; }
  .grp:disabled { opacity: 0.45; }
  legend { padding: 0; font-weight: 500; margin-bottom: 2px; }
  .checks { display: flex; flex-wrap: wrap; gap: 6px 18px; }
  .checks.off { opacity: 0.45; }
  .chk { display: inline-flex; align-items: center; gap: 6px; min-height: 24px; }
  .chk input { accent-color: var(--accent); margin: 0; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 24px; }
</style>
