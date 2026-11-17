<script lang="ts">
  // 설정 창 틀(계획서 5-5, 6-1). macOS = 위쪽 탭 11개(Swift SettingsView.swift:62-72),
  // Windows = Windows 11 설정 앱처럼 왼쪽 탐색 창 + 오른쪽 내용, 구분선·노치 탭은 없다(Windows는 테두리만).
  import { ui, t } from '../lib/api.svelte';
  import TopArea from './TopArea.svelte';
  import { TABS } from './tabs';

  let { initialTab = '' }: { initialTab?: string } = $props();
  const s = $derived(ui.s!);
  const tabs = $derived(TABS.filter((x) => !(s.platform === 'win' && x.macOnly)));
  let tab = $state('login');
  // 메뉴에서 특정 탭으로 열면(예: 갱신 주기 "사용자 설정…") 그 탭으로.
  $effect(() => { if (initialTab) tab = initialTab; });
  const current = $derived(tabs.find((x) => x.id === tab) ?? tabs[0]);
</script>

{#if s.platform === 'win'}
  <div class="win">
    <nav aria-label={t('menu.settings')}>
      <div class="app">UsageMeter</div>
      {#each tabs as x (x.id)}
        <button class:on={x.id === current.id} onclick={() => (tab = x.id)}>
          <span class="icon" aria-hidden="true">{x.glyph}</span>{t('tab.' + x.id)}
        </button>
      {/each}
    </nav>
    <main>
      <h1>{t('tab.' + current.id)}</h1>
      <TopArea compact={current.id !== 'login'} />
      <current.component />
    </main>
  </div>
{:else}
  <div class="mac">
    <TopArea compact={false} />
    <div class="tab-panel">
    <div class="tabs" role="tablist">
      {#each tabs as x (x.id)}
        <button role="tab" aria-selected={x.id === current.id} class:on={x.id === current.id} onclick={() => (tab = x.id)}>{t('tab.' + x.id)}</button>
      {/each}
    </div>
    <main><current.component /></main>
    </div>
  </div>
{/if}

<style>
  .mac { display: flex; flex-direction: column; height: 100vh; padding: 14px 10px 10px; gap: 20px; }
  .tab-panel { flex: 1; min-height: 0; position: relative; border: 1px solid var(--panel-stroke); border-radius: 5px; background: var(--panel-bg); display: flex; flex-direction: column; }
  .mac .tabs { display: flex; justify-content: center; align-self: center; border: 1px solid var(--panel-stroke); border-radius: 7px; padding: 2px; background: var(--segment-bg); position: relative; margin-top: -13px; z-index: 1; flex: none; }
  .mac .tabs button { border: none; border-radius: 0; background: transparent; padding: 1px 12px; min-height: 20px; line-height: 16px; font-size: 12px; font-weight: 500; white-space: nowrap; position: relative; }
  .mac .tabs button + button::before { content: ""; position: absolute; left: 0; top: 4px; bottom: 4px; width: 1px; background: var(--surface-stroke); }
  .mac .tabs button.on { background: var(--segment-selected); color: var(--fg); border-radius: 5px; }
  :global(html[lang="en"]) .mac .tabs button { padding-inline: 8px; }
  .mac main { flex: 1; overflow: auto; padding: 20px 16px 14px; min-height: 0; }

  .win { display: flex; height: 100vh; }
  .win nav { width: 260px; flex: none; padding: 12px 8px; display: flex; flex-direction: column; gap: 2px; overflow: auto; }
  .win .app { font-size: 12px; color: var(--fg2); padding: 8px 12px 14px; }
  .win nav button { display: flex; align-items: center; gap: 12px; text-align: left; border: none; background: transparent; padding: 0 12px; min-height: 36px; border-radius: 4px; position: relative; }
  .win nav button:hover { background: var(--control-hover); }
  .win nav button.on { background: var(--control); }
  .win nav button.on::before { content: ""; position: absolute; left: 0; top: 10px; bottom: 10px; width: 3px; border-radius: 2px; background: var(--accent); }
  .win .icon { font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets"; font-size: 16px; width: 16px; }
  .win main { flex: 1; overflow: auto; padding: 24px 36px 24px 24px; }
  .win h1 { font-size: 28px; font-weight: 600; margin: 4px 0 20px; font-family: "Segoe UI Variable Display", "Segoe UI", sans-serif; }
</style>
