<script lang="ts">
  // 갱신 주기 탭(Swift SettingsView.swift:445-460).
  import { ui, t, tn, set } from '../../lib/api.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import Slider from '../../components/Slider.svelte';
  const s = $derived(ui.s!);
  const m = $derived(s.main);
</script>

<Section title={t('sec.interval')} help={t('help.interval')}>
  <div class="row" class:off={!!m.refreshUseCustom}>
    <span class="grow">{t('interval.preset')}</span>
    <select value={m.refreshPresetMinutes} disabled={m.refreshUseCustom} onchange={(e) => set('main', { refreshPresetMinutes: +(e.currentTarget as HTMLSelectElement).value })}>
      {#each s.refreshPresets as n (n)}<option value={n}>{tn('interval.minutesFmt', n)}</option>{/each}
    </select>
  </div>
  <hr />
  <Toggle label={t('interval.useCustom')} help={t('help.customInterval')} checked={!!m.refreshUseCustom} onchange={(v) => set('main', { refreshUseCustom: v })} />
  <Slider label={t('interval.customMin')} value={m.refreshCustomMinutes ?? 5} min={1} max={180} step={1} disabled={!m.refreshUseCustom} onchange={(v) => set('main', { refreshCustomMinutes: Math.round(v) })} />
  <span class="caption">{tn('interval.currentFmt', s.effectiveRefresh)}</span>
</Section>

<style>
  .off { opacity: .45; }
</style>
