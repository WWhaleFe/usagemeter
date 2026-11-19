<script lang="ts">
  // 미지원 기능은 같은 위치에 보여 주되 입력과 값 변경을 막는다.
  import type { Snippet } from 'svelte';
  import { ui, t } from '../lib/api.svelte';
  const featureId = $props.id();
  let { children, hintKey = 'platform.windowsUnsupported' }: { children: Snippet; hintKey?: string } = $props();
</script>

{#if ui.s?.platform === 'win'}
  <fieldset disabled aria-describedby="{featureId}-hint">
    {@render children()}
  </fieldset>
  <p class="caption" id="{featureId}-hint">{t(hintKey)}</p>
{:else}
  {@render children()}
{/if}

<style>
  fieldset { min-width: 0; margin: 0; padding: 0; border: 0; display: flex; flex-direction: column; gap: 8px; opacity: .45; }
  p { margin: 0 0 8px; }
</style>
