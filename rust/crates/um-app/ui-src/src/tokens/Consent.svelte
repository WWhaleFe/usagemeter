<script lang="ts">
  // 처음 분석 창을 열 때 한 번 묻는다(결정 D4). 답은 저장되고, 나중에 설정 > 표시 > 로컬 사용 기록에서 바꾼다.
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '../lib/api.svelte';
  let { onanswer }: { onanswer: () => void } = $props();
  async function answer(enable: boolean) {
    await invoke('ui_locallogs_set', { action: 'answer', arg: { enable } });
    onanswer();
  }
</script>

<div class="consent" role="dialog" aria-labelledby="ll-ask">
  <h2 id="ll-ask">{t('locallogs.askTitle')}</h2>
  <p>{t('locallogs.askBody')}</p>
  <div class="row">
    <span class="grow"></span>
    <button onclick={() => answer(false)}>{t('locallogs.notNow')}</button>
    <button class="primary" onclick={() => answer(true)}>{t('locallogs.enable')}</button>
  </div>
</div>

<style>
  .consent { background: var(--surface); border: 1px solid var(--surface-stroke); border-radius: var(--r-card); padding: 16px 18px; margin-bottom: 14px; }
  h2 { font-size: var(--fs-title); margin: 0 0 6px; }
  p { margin: 0 0 12px; color: var(--fg2); line-height: 1.5; user-select: text; }
</style>
