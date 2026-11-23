<script lang="ts">
  // 표시 탭: 언어, 모양, AI 순서, 테두리 호버, 알림, 로컬 기록. 모두 전역(main).
  import { ui, t, tn, act, set } from '../../lib/api.svelte';
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
  const thresholds = $derived<number[]>(m.notifyThresholds ?? [75, 90, 95]);
  const notifyOn = $derived(m.notifyEnabled ?? false);
  // 배치 탭 "호버 영역을 배치 영역과 같게"가 켜지면 아래 변·꼭짓점 설정은 쓰이지 않는다.
  const follow = $derived(m.hoverFollowLayout ?? true);

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
  import LocalLogsSection from '../LocalLogsSection.svelte';
  import Icon from '../../analytics/Icon.svelte';
  import InfoTip from '../../components/InfoTip.svelte';
</script>

<Section title={t('lang.title')} help={t('help.lang')}>
  <Segmented options={langs} value={s.lang} onchange={(v) => act('lang', { code: v })} />
</Section>

<Section title={t('appearance.title')} help={t('help.appearance')}>
  <Segmented options={appearances} value={m.appearance ?? 'system'} onchange={(v) => set('main', { appearance: v })} />
</Section>

<Section title={t('sec.providerOrder')} help={t('help.providerOrder')}>
  {#each order as id, i (id)}
    <div class="row order-row">
      <span class="grow">{i + 1}. {s.providers.find((p) => p.id === id)?.name}</span>
      <button class="icon" aria-label={t('order.up')} title={t('order.up')} disabled={i === 0} onclick={() => move(i, -1)}><Icon name="chevron.up" size={10} /></button>
      <button class="icon" aria-label={t('order.down')} title={t('order.down')} disabled={i === order.length - 1} onclick={() => move(i, 1)}><Icon name="chevron.down" size={10} /></button>
    </div>
  {/each}
  <span class="caption">{t('order.desc')}</span>
</Section>

<Section title={t('sec.hover')} help={t('help.hover')}>
  <Toggle label={t('hover.enable')} checked={hoverOn} onchange={(v) => set('main', { hoverInfoEnabled: v })} />
  <span class="caption">{t('hover.enableDesc')}</span>
  <Toggle label={t('hover.emphasis')} help={t('help.hoverEmphasis')} checked={m.hoverEmphasisEnabled ?? true} disabled={!hoverOn} onchange={(v) => set('main', { hoverEmphasisEnabled: v })} />
  <Toggle label={t('hover.textEmphasis')} help={t('help.hoverTextEmphasis')} checked={m.hoverTextEmphasis ?? true} disabled={!hoverOn} onchange={(v) => set('main', { hoverTextEmphasis: v })} />
  <!-- 배치 탭의 같은 옵션과 한 설정(main.hoverFollowLayout)을 함께 쓴다. -->
  <Toggle label={t('lay.hoverFollowLayout')} help={t('help.hoverFollowLayout')} checked={follow} disabled={!hoverOn} onchange={(v) => set('main', { hoverFollowLayout: v })} />
  <fieldset class="grp" disabled={!hoverOn || follow}>
    {#if follow}<span class="caption note">{t('hoverArea.followingLayout')}</span>{/if}
    <legend>{t('hoverArea.edges')}<InfoTip text={t('help.hoverEdges')} /></legend>
    <div class="checks">
      {#each edgeKeys as e (e)}
        <label class="chk"><input type="checkbox" checked={edges.includes(e)} onchange={(ev) => set('main', { hoverEdges: member(edges, e, ev.currentTarget.checked) })} />{t('edge.' + e)}</label>
      {/each}
      {#if s.platform === 'mac'}
      <label class="chk"><input type="checkbox" checked={m.hoverMenuLine ?? false} onchange={(ev) => set('main', { hoverMenuLine: ev.currentTarget.checked })} />{t('hoverArea.menuLine')}</label>
      {/if}
      <label class="chk"><input type="checkbox" checked={m.hoverDockLine ?? false} onchange={(ev) => set('main', { hoverDockLine: ev.currentTarget.checked })} />{t('hoverArea.dockLine')}</label>
    </div>
    <legend>{t('hoverArea.corners')}<InfoTip text={t('help.hoverCorners')} /></legend>
    <div class="checks">
      {#each cornerKeys as c (c)}
        <label class="chk"><input type="checkbox" checked={corners.includes(c)} onchange={(ev) => set('main', { hoverCorners: member(corners, c, ev.currentTarget.checked) })} />{t('corner.' + c)}</label>
      {/each}
    </div>
    <Slider label={t('hoverArea.cornerSize')} value={m.hoverCornerSize ?? 80} min={20} max={300} step={5} disabled={!hoverOn || follow || corners.length === 0} onchange={(v) => set('main', { hoverCornerSize: v })} />
    <span class="caption">{t('hoverArea.desc')}</span>
  </fieldset>

</Section>

<Section title={t('sec.notify')} help={t('help.notify')}>
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
  .grp:disabled .note { opacity: 1; color: var(--fg); }
  legend { padding: 0; font-weight: 500; margin-bottom: 2px; }
  .checks { display: flex; flex-wrap: wrap; gap: 6px 18px; }
  .checks.off { opacity: 0.45; }
  .chk { display: inline-flex; align-items: center; gap: 6px; min-height: 24px; }
  .chk input { accent-color: var(--accent); margin: 0; }
  :global(:root[data-skin="macos"]) .order-row { margin-block: -2px; }
  :global(:root[data-skin="macos"]) legend { font-size: 12px; font-weight: 500; }
  :global(:root[data-skin="macos"]) .icon { display: flex; align-items: center; justify-content: center; width: 18px; height: 16px; min-height: 16px; background: transparent; border: 0; min-width: 16px; padding: 0 4px; box-shadow: none; color: var(--fg2); }
  :global(:root[data-skin="macos"]) .checks { gap: 6px 14px; }
  :global(:root[data-skin="macos"]) .chk { min-height: 16px; gap: 5px; }
  :global(:root[data-skin="macos"]) .grp { gap: 8px; }
</style>
