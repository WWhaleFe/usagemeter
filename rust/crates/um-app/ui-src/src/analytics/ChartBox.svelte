<script lang="ts">
  // 차트 영역 크기를 재서 자식에게 (너비, 높이)를 넘긴다. 창 크기가 바뀌면 다시 그린다.
  import type { Snippet } from 'svelte';
  let { children }: { children: Snippet<[number, number]> } = $props();
  let w = $state(0), h = $state(0);
</script>

<div class="box" bind:clientWidth={w} bind:clientHeight={h}>
  {#if w > 0 && h > 0}<div class="in">{@render children(w, h)}</div>{/if}
</div>

<style>
  .box { flex: 1; min-height: 0; min-width: 0; position: relative; }
  .in { position: absolute; inset: 0; }
  .in :global(svg) { display: block; }
  .in :global(text) { fill: var(--fg2); font-size: 10px; }
</style>
