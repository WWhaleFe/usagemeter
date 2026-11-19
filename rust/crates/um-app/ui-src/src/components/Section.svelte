<script lang="ts">
  // 설정 묶음 상자(Swift `section(_:_)`).
  import type { Snippet } from 'svelte';
  let { title = '', children }: { title?: string; children: Snippet } = $props();
</script>

<section class="mac">
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
  section.mac { background: var(--surface); border-radius: 8px; padding: 12px; margin-bottom: 12px; }
  section.mac h3 { margin: 0 0 8px; font-size: 13px; line-height: 16px; font-weight: 700; }
  section.mac .body { background: none; border: none; border-radius: 0; padding: 0; gap: 8px; }
</style>
