<script lang="ts">
  // 노치 탭(Swift SettingsView.swift:678-707, macOS 전용): 감싸기 켜기, 크기, 노치 모서리 곡률, 자동 감지.
  import { ui, t, set, act } from '../../lib/api.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import Slider from '../../components/Slider.svelte';
  const b = $derived(ui.s!.border);
  const on = $derived(!!b.notchEnabled);
  const radii = $derived((b.notchRadii ?? {}) as Record<string, number>);
  // 저장 순서 = outerLeft, innerLeft, innerRight, outerRight(Swift NotchCorner).
  const CORNERS = ['outerLeft', 'innerLeft', 'innerRight', 'outerRight'];
</script>

<Section title={t('sec.notch')}>
  <Toggle label={t('notch.enable')} checked={on} onchange={(v) => set('border', { notchEnabled: v })} />
  <!-- 꺼지면 노치 세부 전부 비활성(Swift :688-698). -->
  <Slider label={t('notch.width')} value={b.notchWidth ?? 0} min={0} max={500} unit="pt" disabled={!on} onchange={(v) => set('border', { notchWidth: v })} />
  <Slider label={t('notch.height')} value={b.notchHeight ?? 0} min={0} max={100} unit="pt" disabled={!on} onchange={(v) => set('border', { notchHeight: v })} />
  <hr />
  <span class="caption">{t('notch.cornerRadius')}</span>
  {#each CORNERS as c (c)}
    <Slider label={t('notch.' + c)} value={radii[c] ?? 0} min={0} max={40} unit="pt" disabled={!on} onchange={(v) => set('border', { notchRadii: { ...radii, [c]: v } })} />
  {/each}
  <!-- 이 화면의 노치를 읽어 크기를 채운다(없으면 끔). -->
  <div class="row"><button onclick={() => act('detectNotch')}>{t('notch.detect')}</button></div>
</Section>

<style>
  hr { border: none; border-top: 1px solid var(--surface-stroke); margin: 2px 0; width: 100%; }
</style>
