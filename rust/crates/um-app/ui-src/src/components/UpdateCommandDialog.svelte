<script lang="ts">
  // Homebrew 업데이트 명령을 복사한 뒤 띄우는 안내. 닫기(X)·바깥 클릭·Esc로는 닫히지 않고 확인 버튼으로만 닫는다.
  import { t } from '../lib/api.svelte';
  let { command, copied, onclose }: { command: string; copied: boolean; onclose: () => void } = $props();
  let ok = $state<HTMLButtonElement>();
  $effect(() => { ok?.focus(); });
</script>

<div class="backdrop">
  <div class="dialog" role="alertdialog" aria-modal="true" aria-labelledby="ucd-title" aria-describedby="ucd-body">
    <h2 id="ucd-title">{copied ? t('update.copiedTitle') : t('update.copyFailedTitle')}</h2>
    <p id="ucd-body">{copied ? t('update.copiedBody') : t('update.copyFailedBody')}</p>
    <code>{command}</code>
    <p class="caption">{t('update.copiedFallback')}</p>
    <div class="actions"><button bind:this={ok} class="primary" onclick={onclose}>{t('update.ok')}</button></div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; z-index: 1000; display: grid; place-items: center; background: rgba(0, 0, 0, 0.28); padding: 16px; }
  .dialog {
    width: min(420px, 100%); padding: 20px; border-radius: 12px;
    background: var(--bg); border: 1px solid var(--control-stroke); box-shadow: 0 12px 32px rgba(0, 0, 0, 0.25);
  }
  h2 { margin: 0 0 8px; font-size: 14px; font-weight: 600; }
  p { margin: 0 0 12px; }
  code {
    display: block; margin: 0 0 12px; padding: 8px 12px; border-radius: 8px;
    background: var(--surface); border: 1px solid var(--surface-stroke);
    font: 12px/1.5 ui-monospace, SFMono-Regular, Menlo, monospace; user-select: all; overflow-wrap: anywhere;
  }
  .actions { display: flex; justify-content: flex-end; margin-top: 16px; }
  .actions button { min-width: 72px; }
</style>
