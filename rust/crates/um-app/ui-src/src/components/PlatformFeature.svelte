<script lang="ts">
  // Windows에서 동작하지 않는 기능(OS 차이)은 Windows 화면에서 아예 보이지 않게 한다.
  import type { Snippet } from 'svelte';
  import { ui } from '../lib/api.svelte';
  let { children }: { children: Snippet; hintKey?: string } = $props();
</script>

{#if ui.s?.platform !== 'win'}
  {@render children()}
{/if}
