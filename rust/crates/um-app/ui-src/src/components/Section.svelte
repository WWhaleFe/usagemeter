<script lang="ts">
  // 설정 묶음 상자(Swift `section(_:_)`). Windows 스킨에서는 Fluent 설정 카드 모양.
  import type { Snippet } from 'svelte';
  let { title = '', children }: { title?: string; children: Snippet } = $props();
</script>

<section>
  {#if title}<h3>{title}</h3>{/if}
  <div class="body">{@render children()}</div>
</section>

<style>
  section { margin: 0 0 14px; }
  h3 { font-size: var(--fs); font-weight: 600; margin: 0 0 6px; }
  .body {
    background: var(--surface); border: 1px solid var(--surface-stroke); border-radius: var(--r-card);
    padding: 10px 12px; display: flex; flex-direction: column; gap: 6px;
  }
  :global(:root[data-skin="win"]) section { margin-bottom: 20px; }
  :global(:root[data-skin="win"]) h3 { font-size: 14px; font-weight: 600; margin-bottom: 8px; }
  :global(:root[data-skin="win"]) .body { padding: 12px 16px; gap: 8px; }
</style>
