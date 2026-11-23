<script lang="ts">
  // 세부 항목 탭(Swift SettingsView.swift:513-579): AI별 분리 토글, 항목별 색·미니 바·바/메뉴 표시.
  import { ui, settingsProviders, t, set, act, fromHex } from '../../lib/api.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import ColorPicker from '../../components/ColorPicker.svelte';
  const s = $derived(ui.s!);
  const providers = $derived(settingsProviders(s));
  const m = $derived(s.main);
  const splitIDs = $derived<string[]>(m.subItemSplitIDs ?? []);
  const colors = $derived((m.subItemColors ?? {}) as Record<string, number[]>);
  const barHidden = $derived<string[]>(m.subItemBarHidden ?? []);
  const menuHidden = $derived<string[]>(m.subItemMenuHidden ?? []);
  const chartShown = $derived<string[]>(m.subItemChartShown ?? []);
  const analyticsHidden = $derived<string[]>(m.subItemAnalyticsHidden ?? []);
  const member = (list: string[], v: string, on: boolean) => (on ? (list.includes(v) ? list : [...list, v]) : list.filter((x) => x !== v));
  function setColor(key: string, hex: string) {
    set('main', { subItemColors: { ...colors, [key]: fromHex(hex) } });
  }
  function resetColor(key: string) {
    const { [key]: _drop, ...rest } = colors;
    set('main', { subItemColors: rest });
  }
</script>

<span class="caption desc">{t('subItems.desc')}</span>
<span class="caption desc">{t('subItems.viewDesc')}</span>

{#each providers as p (p.id)}
  {@const split = splitIDs.includes(p.id)}
  <Section title={p.name} help={t('help.subItemOpts')}>
    <Toggle label={t('subItems.split')} help={t('help.subItemSplit')} checked={split} onchange={(v) => set('main', { subItemSplitIDs: member(splitIDs, p.id, v) })} />
    {#if p.items.length === 0}
      <span class="caption">{t('subItems.noData')}</span>
    {:else}
      {#if !p.itemsLive}<span class="caption warn">{t('subItems.lastKnown')}</span>{/if}
      {#each p.items as it (it.key)}
        {@const pct = Math.round(it.ratio * 100)}
        {@const locked = it.key.endsWith('/5h')}
        {@const custom = !locked && it.key in colors}
        <div class="item" style:opacity={p.itemsLive ? 1 : 0.6}>
          <div class="row">
            {#if s.platform === 'mac'}
              <button class="color-well" disabled={!split || locked} title={locked ? t('subItems.followsAi') : ''} aria-label={it.name} onclick={() => act('pickColor', { id: p.id, item: it.key })}><span style:background={it.color}></span></button>
            {:else}
              <ColorPicker value={it.color} disabled={!split || locked} title={locked ? t('subItems.followsAi') : ''} label={it.name} onpick={(h) => setColor(it.key, h)} />
            {/if}
            <span class="name grow" title={it.name}>{it.name}</span>
            <span class="bar" style:background={it.color + '2e'} aria-hidden="true"><span style:background={it.color} style:width="{pct}%"></span></span>
            <span class="caption pct">{pct}% · {t('subItems.used')} {100 - pct}%</span>
          </div>
          <div class="row opts">
            <label class="chk"><input type="checkbox" checked={!barHidden.includes(it.key)} disabled={!split} onchange={(e) => set('main', { subItemBarHidden: member(barHidden, it.key, !e.currentTarget.checked) })} />{t('subItems.bar')}</label>
            <label class="chk"><input type="checkbox" checked={!menuHidden.includes(it.key)} onchange={(e) => set('main', { subItemMenuHidden: member(menuHidden, it.key, !e.currentTarget.checked) })} />{t('subItems.menu')}</label>
            <label class="chk"><input type="checkbox" checked={chartShown.includes(it.key)} onchange={(e) => set('main', { subItemChartShown: member(chartShown, it.key, e.currentTarget.checked) })} />{t('subItems.chart')}</label>
            <label class="chk"><input type="checkbox" checked={!analyticsHidden.includes(it.key)} onchange={(e) => set('main', { subItemAnalyticsHidden: member(analyticsHidden, it.key, !e.currentTarget.checked) })} />{t('subItems.analytics')}</label>
            <span class="grow"></span>
            <button class="reset" style:visibility={custom ? 'visible' : 'hidden'} disabled={!custom} aria-label={t('subItems.resetColor')} title={t('subItems.resetColor')} onclick={() => resetColor(it.key)}>&#8630;</button>
          </div>
        </div>
      {/each}
    {/if}
  </Section>
{/each}

<style>
  .desc { display: block; margin-bottom: 10px; }
  .item { display: flex; flex-direction: column; gap: 2px; padding: 4px 0; }
  .item + .item { border-top: 1px solid var(--surface-stroke); }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .bar { width: 70px; height: 6px; border-radius: 3px; overflow: hidden; flex: none; }
  .bar span { display: block; height: 100%; border-radius: 3px; }
  .pct { width: 112px; text-align: right; font-variant-numeric: tabular-nums; flex: none; }
  .opts { padding-left: 40px; gap: 16px; min-height: 26px; }
  .chk { display: inline-flex; align-items: center; gap: 6px; }
  .chk input { accent-color: var(--accent); margin: 0; }
  .reset { min-height: 22px; padding: 0 8px; }
  :global(:root[data-skin="macos"]) .item { flex-direction: row; gap: 8px; padding: 0; align-items: center; }
  :global(:root[data-skin="macos"]) .item + .item { border: 0; }
  :global(:root[data-skin="macos"]) .item > .row { display: contents; }
  /* 항목 이름과 막대가 남는 폭을 나눠 쓴다(오른쪽 빈 공간 없앰). */
  :global(:root[data-skin="macos"]) .name { width: auto; flex: 1 1 200px; min-width: 0; font-size: 12px; font-weight: 400; }
  :global(:root[data-skin="macos"]) .bar { width: auto; flex: 1 1 120px; max-width: 260px; }
  :global(:root[data-skin="macos"]) .pct { width: 92px; }
  :global(:root[data-skin="macos"]) .color-well { width: 44px; height: 24px; flex: none; border: 3px solid #777; padding: 0; }
  .color-well span { display: block; width: 100%; height: 100%; border-radius: 1px; }
  :global(:root[data-skin="macos"]) .reset { border: 0; background: transparent; padding: 0; box-shadow: none; }
  :global(:root[data-skin="macos"]) .chk { gap: 4px; white-space: nowrap; }
</style>
