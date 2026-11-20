<script lang="ts">
  // 마지막 내보내기 결과 한 줄: 성공이면 저장 위치와 "폴더 열기", 실패면 이유.
  import { t, tf } from '../lib/api.svelte';
  import { lastOf, reveal, fmtWhen } from '../lib/export.svelte';
  let { auto = false, button = true }: { auto?: boolean; button?: boolean } = $props();
  // 자동 저장 칸에는 자동 결과, 지금 내보내기 칸(분석 창 포함)에는 직접 저장한 결과.
  const r = $derived(lastOf(auto));
</script>

{#if r}
  <div class="result" class:bad={!r.ok}>
    {#if r.ok && r.path}
      <span class="grow caption">{tf('export.last', fmtWhen(r.t))} · <span class="path" title={r.path}>{r.path}</span></span>
      {#if button}<button onclick={() => reveal(r.path!)}>{t('export.open')}</button>{/if}
    {:else}
      <span class="grow caption warn">{tf('export.failed', r.error ?? '')}</span>
    {/if}
  </div>
{/if}

<style>
  .result { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .path { user-select: text; word-break: break-all; }
</style>
