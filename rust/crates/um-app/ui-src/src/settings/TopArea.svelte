<script lang="ts">
  // 탭과 무관하게 보이는 상단(Swift SettingsView.swift:100-208): 기본 상태 저장·되돌리기, 분석 버튼,
  // 업데이트 줄(Store판 제외), 디스플레이 선택(화면 2대 이상). compact면(Windows 하위 화면) 디스플레이 선택만.
  import { ui, t, tf, act, set } from '../lib/api.svelte';
  import Toggle from '../components/Toggle.svelte';
  let { compact = false }: { compact?: boolean } = $props();
  const s = $derived(ui.s!);
  const u = $derived(s.update);
  const status = $derived(
    u.state === 'checking' ? t('update.checking')
    : u.state === 'upToDate' ? '✓ ' + t('update.upToDate')
    : u.state === 'available' ? '🔵 ' + tf('update.available', u.version ?? '')
    : u.state === 'failed' ? '⚠️ ' + t('update.failed') : '');
  const dl = $derived(({ downloading: 'update.downloading', done: 'update.downloaded', failed: 'update.downloadFail' } as Record<string, string>)[u.download] ?? 'update.download');
  const editing = $derived(s.editing ?? s.displays.find((d) => d.main)?.id ?? '');
  const edDisp = $derived(s.displays.find((d) => d.id === editing));
  function label(d: (typeof s.displays)[number]) {
    const same = s.displays.filter((x) => x.name === d.name);
    const n = same.length > 1 ? ` (${same.indexOf(d) + 1})` : '';
    return d.name + n + (d.primary ? ` - ${t('display.primary')}` : '');
  }
</script>

{#if !compact}
  <div class="row top">
    <span class="caption">{t('sec.baseState')}</span>
    <button onclick={() => act('defaultSave')}>{t('base.save')}</button>
    <button disabled={!s.hasDefault} onclick={() => act('defaultReset')}>{t('base.reset')}</button>
    <span class="grow"></span>
    <button onclick={() => act('openAnalytics')}>{t('menu.analytics')}</button>
  </div>
  {#if !s.store}
    <div class="row">
      <span>UsageMeter {tf('update.version', s.version)}</span>
      <span class="caption">{status}</span>
      <span class="grow"></span>
      <button disabled={u.state === 'checking'} onclick={() => act('checkUpdate')}>{t('update.check')}</button>
      <button class="primary" disabled={u.download === 'downloading'} onclick={() => act('download')}>{t(dl)}</button>
      <button onclick={() => act('openReleases')}>{t('update.openPage')}</button>
    </div>
    <Toggle label={t('update.auto')} checked={!!s.main.autoCheckUpdate} onchange={(v) => set('main', { autoCheckUpdate: v })} />
  {/if}
{/if}
{#if s.displays.length > 1}
  <div class="row displays">
    <span class="caption">{t('display.editing')}</span>
    <select value={editing} onchange={(e) => act('selectDisplay', { id: (e.currentTarget as HTMLSelectElement).value })}>
      {#each s.displays as d (d.id)}<option value={d.id}>{label(d)}</option>{/each}
    </select>
    {#if edDisp}
      <Toggle label={t('display.show')} checked={edDisp.enabled} onchange={(v) => act('displayEnabled', { id: edDisp.id, enabled: v })} />
    {/if}
    <button title={t('display.copyToOthersHelp')} onclick={() => act('copyToOthers')}>{t('display.copyToOthers')}</button>
  </div>
{/if}

<style>
  .row { flex-wrap: wrap; }
  .displays { margin-bottom: 8px; }
  .displays select { max-width: 280px; }
</style>
