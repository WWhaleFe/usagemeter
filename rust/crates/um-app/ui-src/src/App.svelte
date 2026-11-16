<script lang="ts">
  // 해시로 화면을 고른다: "settings", "settings/<탭>", "analytics", "linedrag"(선 위치 끌기).
  import { ui } from './lib/api.svelte';
  import Settings from './settings/Settings.svelte';
  import LineDrag from './settings/LineDrag.svelte';

  let hash = $state(location.hash.slice(1));
  addEventListener('hashchange', () => (hash = location.hash.slice(1)));
  const page = $derived(hash.split('/')[0] || 'settings');
  const tab = $derived(hash.split('/')[1] ?? '');

  // 외관(시스템·밝게·어둡게)과 OS 스킨을 문서에 반영.
  $effect(() => {
    const s = ui.s;
    if (!s) return;
    document.documentElement.dataset.skin = s.platform;
    const ap = s.main.appearance as string | undefined;
    document.documentElement.dataset.theme = ap === 'light' || ap === 'dark' ? ap : '';
    document.documentElement.lang = s.lang;
  });
</script>

{#if ui.s}
  {#if page === 'analytics'}
    <main class="placeholder"><h1>{ui.s.strings['analytics.title']}</h1></main>
  {:else if page === 'linedrag'}
    <LineDrag />
  {:else}
    <Settings initialTab={tab} />
  {/if}
{/if}
