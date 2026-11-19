<script lang="ts">
  // 설정 창 틀: 두 플랫폼이 상단 영역과 가운데 탭 12개를 함께 쓴다.
  import { ui, t } from '../lib/api.svelte';
  import TopArea from './TopArea.svelte';
  import { tabsForStore } from './tabs';

  let { initialTab = '' }: { initialTab?: string } = $props();
  const tabs = $derived(tabsForStore(!!ui.s?.store, ui.s?.platform === 'win'));
  let tab = $state('login');
  // 메뉴에서 특정 탭으로 열면(예: 갱신 주기 "사용자 설정…") 그 탭으로.
  $effect(() => { if (initialTab) tab = initialTab; });
  const current = $derived(tabs.find((x) => x.id === tab) ?? tabs[0]);
</script>

<div class="mac">
  <TopArea />
  <div class="tab-panel">
  <div class="tabs" role="tablist">
    {#each tabs as x (x.id)}
      <button role="tab" aria-selected={x.id === current.id} class:on={x.id === current.id} onclick={() => (tab = x.id)}>{t('tab.' + x.id)}</button>
    {/each}
  </div>
  <main><current.component /></main>
  </div>
</div>

<style>
  .mac { display: flex; flex-direction: column; height: 100vh; padding: 14px 10px 10px; gap: 20px; }
  .tab-panel { flex: 1; min-height: 0; position: relative; border: 1px solid var(--panel-stroke); border-radius: 5px; background: var(--panel-bg); display: flex; flex-direction: column; }
  .mac .tabs { display: flex; justify-content: center; align-self: center; border: 1px solid var(--panel-stroke); border-radius: 7px; padding: 2px; background: var(--segment-bg); position: relative; margin-top: -13px; z-index: 1; flex: none; max-width: calc(100% - 12px); }
  .mac .tabs button { border: none; border-radius: 0; background: transparent; padding: 1px clamp(3px, 1vw, 10px); min-height: 20px; line-height: 16px; font-size: 12px; font-weight: 500; white-space: nowrap; position: relative; }
  .mac .tabs button + button::before { content: ""; position: absolute; left: 0; top: 4px; bottom: 4px; width: 1px; background: var(--surface-stroke); }
  .mac .tabs button.on { background: var(--segment-selected); color: var(--fg); border-radius: 5px; }
  :global(html[lang="en"]) .mac .tabs button { padding-inline: 4px; }
  .mac main { flex: 1; overflow: auto; padding: 20px 16px 14px; min-height: 0; }

</style>
