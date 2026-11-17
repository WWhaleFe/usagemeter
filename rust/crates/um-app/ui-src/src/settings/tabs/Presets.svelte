<script lang="ts">
  // 프리셋 탭(Swift SettingsView.swift:258-293): 저장된 프리셋 불러오기·삭제, 새 이름으로 저장.
  import { ui, t, tn, act } from '../../lib/api.svelte';
  import Section from '../../components/Section.svelte';
  const s = $derived(ui.s!);
  let name = $state('');
  const full = $derived(s.presets.length >= s.maxPresets);
  async function save() {
    await act('presetSave', { name });
    name = '';
  }
</script>

<Section title={tn('sec.presets', s.maxPresets)}>
  {#each s.presets as p (p.id)}
    <div class="row">
      <button class="grow name" onclick={() => act('presetLoad', { id: p.id })}>{p.name}</button>
      <button class="danger" aria-label={p.name} title="delete" onclick={() => act('presetDelete', { id: p.id })}>&#128465;</button>
    </div>
  {/each}
  <form class="row" onsubmit={(e) => { e.preventDefault(); if (!full) save(); }}>
    <input class="grow" type="text" placeholder={t('preset.newName')} aria-label={t('preset.newName')} bind:value={name} />
    <button type="submit" disabled={full}>{t('preset.save')}</button>
  </form>
  {#if s.editing}<span class="caption">{t('preset.displayNote')}</span>{/if}
</Section>

<style>
  .name { text-align: left; }
  .danger { color: var(--danger); }
  :global(:root[data-skin="macos"]) input { background: var(--field-bg); }
  :global(:root[data-skin="macos"]) .name { flex: none; }
  :global(:root[data-skin="macos"]) .danger { margin-left: auto; background: transparent; border: 0; box-shadow: none; }
</style>
