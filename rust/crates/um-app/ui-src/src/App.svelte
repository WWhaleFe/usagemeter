<script lang="ts">
  // 해시로 화면을 고른다: "settings", "settings/<탭>", "analytics", "linedrag"(선 위치 끌기).
  import { ui } from './lib/api.svelte';
  import Settings from './settings/Settings.svelte';
  import Analytics from './analytics/Analytics.svelte';
  import LineDrag from './settings/LineDrag.svelte';

  let hash = $state(location.hash.slice(1));
  addEventListener('hashchange', () => (hash = location.hash.slice(1)));
  const page = $derived(hash.split('/')[0] || 'settings');
  const tab = $derived(hash.split('/')[1] ?? '');

  // 레이아웃 스킨은 공통으로 쓰고, OS는 글꼴·네이티브 기능에만 반영한다.
  $effect(() => {
    const s = ui.s;
    if (!s) return;
    document.documentElement.dataset.skin = 'macos';
    document.documentElement.dataset.platform = s.platform;
    const ap = s.main.appearance as string | undefined;
    document.documentElement.dataset.theme = ap === 'light' || ap === 'dark' ? ap : '';
    document.documentElement.lang = s.lang;
  });
</script>

{#if ui.s}
  {#if page === 'analytics'}
    <Analytics />
  {:else if page === 'linedrag'}
    <LineDrag />
  {:else}
    <Settings initialTab={tab} />
  {/if}
{/if}
