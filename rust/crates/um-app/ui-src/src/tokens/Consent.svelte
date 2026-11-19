<script lang="ts">
  // 처음 분석 창을 열 때 한 번 묻는다(결정 D4). 답은 저장되고, 나중에 설정 > 표시 > 로컬 사용 기록에서 바꾼다.
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '../lib/api.svelte';
  import Section from '../components/Section.svelte';
  let { onanswer }: { onanswer: () => void } = $props();
  async function answer(enable: boolean) {
    await invoke('ui_locallogs_set', { action: 'answer', arg: { enable } });
    onanswer();
  }
</script>

{#snippet buttons()}
<div class="row">
  <span class="grow"></span>
  <button onclick={() => answer(false)}>{t('locallogs.notNow')}</button>
  <button class="primary" onclick={() => answer(true)}>{t('locallogs.enable')}</button>
</div>
{/snippet}

<div class="mac-consent" role="region" aria-label={t('locallogs.askTitle')}>
  <Section title={t('locallogs.askTitle')}>
    <p>{t('locallogs.askBody')}</p>
    {@render buttons()}
  </Section>
</div>

<style>
  p { margin: 0 0 12px; color: var(--fg2); line-height: 1.5; user-select: text; }
  .mac-consent p { margin: 0; font-size: var(--fs-small); }
</style>
