<script lang="ts">
  // 화면 분할 탭(Swift SettingsView.swift:582-601, 플랫폼별): 메뉴바선·Dock선 켜기와 위치, 화면에서 끌어 조정.
  import { ui, t, set, act } from '../../lib/api.svelte';
  import PlatformFeature from '../../components/PlatformFeature.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import Slider from '../../components/Slider.svelte';
  const win = $derived(ui.s?.platform === 'win');
  const b = $derived(ui.s!.border);
  const menuOn = $derived(!!b.menuLineEnabled);
  const dockOn = $derived(!!b.dockLineEnabled);
</script>

<Section title={t('sec.partition')}>
  <PlatformFeature hintKey="part.menuWindows">
  <Toggle label={t('part.menuLine')} checked={menuOn} onchange={(v) => set('border', { menuLineEnabled: v })} />
  <Slider label={t('part.menuHeight')} value={b.menuLineHeight ?? 25} min={5} max={200} unit="pt" disabled={!menuOn} onchange={(v) => set('border', { menuLineHeight: v })} />
  </PlatformFeature>
  <hr />
  <Toggle label={t(win ? 'part.taskbarLine' : 'part.dockLine')} checked={dockOn} onchange={(v) => set('border', { dockLineEnabled: v })} />
  {#if win}
    <span class="caption">{t('part.taskbarDesc')}</span>
  {:else}
  <Slider label={t('part.dockHeight')} value={b.dockLineHeight ?? 70} min={5} max={400} unit="pt" disabled={!dockOn} onchange={(v) => set('border', { dockLineHeight: v })} />
  <!-- 화면 위에서 경계선을 직접 끌어 조정(선 위치 끌기 화면). -->
  <div class="row"><button disabled={!menuOn && !dockOn} onclick={() => act('dragLines')}>{t('part.dragAdjust')}</button></div>
  <span class="caption">{t('part.desc')}</span>
  {/if}
</Section>

<style>
  hr { border: none; border-top: 1px solid var(--surface-stroke); margin: 2px 0; width: 100%; }
</style>
