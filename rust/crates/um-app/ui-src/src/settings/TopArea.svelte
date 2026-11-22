<script lang="ts">
  // 탭과 무관하게 보이는 상단(Swift SettingsView.swift:100-208): 기본 상태 저장·되돌리기, 분석 버튼,
  // 업데이트 줄(Store판 제외), 디스플레이 선택(화면 2대 이상).
  import { ui, t, tf, act, set } from '../lib/api.svelte';
  import Toggle from '../components/Toggle.svelte';
  import Section from '../components/Section.svelte';
  import Icon from '../analytics/Icon.svelte';
  const s = $derived(ui.s!);
  const u = $derived(s.update);
  const status = $derived(
    u.state === 'checking' ? t('update.checking')
    : u.state === 'upToDate' ? '✓ ' + t('update.upToDate')
    : u.state === 'available' ? '🔵 ' + tf('update.available', 'v' + (u.version ?? ''))
    : u.state === 'failed' ? '⚠️ ' + t('update.failed') : '');
  // Windows Store판: 현재·최신 버전과 Store 설치 상태.
  const available = $derived(u.state === 'available');
  const installing = $derived(u.download === 'downloading');
  // 최신 버전이 지금 설치된 버전과 같으면 받을 것이 없다(확인 전에는 눌러서 확인·설치할 수 있다).
  const current = $derived(!available && (u.state === 'upToDate' || u.latest === s.version));
  const latest = $derived(
    available ? (u.version ? 'v' + u.version : t('update.newVersion'))
    : u.latest ? 'v' + u.latest : t('update.latestUnknown'));
  const storeStatus = $derived(
    u.state === 'checking' ? t('update.checking')
    : u.state === 'upToDate' ? t('update.upToDate')
    : available ? t('update.availableNoVersion')
    : u.state === 'failed' ? t('update.failed') : '');
  const storeNote = $derived(
    u.blocked ? t('update.installBlocked')
    : u.download === 'failed' ? t('update.installFail')
    : u.download === 'done' ? t('update.installDone') : t('update.storeHint'));
  const dl = $derived(({ downloading: 'update.downloading', done: 'update.downloaded', failed: 'update.downloadFail' } as Record<string, string>)[u.download] ?? 'update.download');
  const editing = $derived(s.editing ?? s.displays.find((d) => d.main)?.id ?? '');
  const edDisp = $derived(s.displays.find((d) => d.id === editing));
  function label(d: (typeof s.displays)[number]) {
    const same = s.displays.filter((x) => x.name === d.name);
    const n = same.length > 1 ? ` (${same.indexOf(d) + 1})` : '';
    return d.name + n + (d.primary ? ` - ${t('display.primary')}` : '');
  }
</script>

<div class="mac-top">
  <Section title={t('sec.baseState')}>
    <div class="row">
      <button onclick={() => act('defaultSave')}>{t('base.save')}</button>
      <button disabled={!s.hasDefault} onclick={() => act('defaultReset')}>{t('base.reset')}</button>
      <span class="grow"></span>
      <button class="analytics" onclick={() => act('openAnalytics')}><Icon name="chart.bar.xaxis" size={14} />{t('menu.analytics')}</button>
    </div>
    {#if !s.store}
      <hr />
      <div class="row update">
        <strong>UsageMeter {tf('update.version', s.version)}</strong>
        {#if status}<span class="caption">{status}</span>{/if}
        <span class="grow"></span>
        <button disabled={u.state === 'checking'} onclick={() => act('checkUpdate')}>{t('update.check')}</button>
        <button class="primary" disabled={u.download === 'downloading'} onclick={() => act('download')}>{t(dl)}</button>
        <button onclick={() => act('openReleases')}>{t('update.openPage')}</button>
      </div>
      <span class="caption">{t('update.installHint')}</span>
      <div class="auto"><Toggle label={t('update.auto')} checked={!!s.main.autoCheckUpdate} onchange={(v) => set('main', { autoCheckUpdate: v })} /></div>
    {:else if s.platform === 'win'}
      <hr />
      <div class="ver">
        <span class="k">{t('update.current')}</span>
        <span class="v num">v{s.version}</span>
        <button disabled={u.state === 'checking' || installing} onclick={() => act('checkUpdate')}>{t('update.check')}</button>
        <span class="k">{t('update.latest')}</span>
        <span class="v"><span class="num" class:new={available}>{latest}</span>{#if storeStatus}<span class="caption">{storeStatus}</span>{/if}</span>
        <button class="primary" disabled={current || u.state === 'checking' || installing} onclick={() => act('installUpdate')}>{installing ? t('update.installing') : t('update.install')}</button>
      </div>
      <div class="opts">
        <Toggle label={t('update.autoInstall')} checked={s.main.autoInstallUpdate ?? true} onchange={(v) => set('main', { autoInstallUpdate: v })} />
        <Toggle label={t('update.notify')} checked={!!s.main.autoCheckUpdate} onchange={(v) => set('main', { autoCheckUpdate: v })} />
        <span class="caption note" class:warn={u.download === 'failed' || u.blocked}>{storeNote}</span>
      </div>
    {/if}
  </Section>
  {#if s.displays.length > 1}
    <Section title={t('sec.displays')}>
      <div class="row">
        <span>{t('display.editing')}</span>
        <select value={editing} onchange={(e) => act('selectDisplay', { id: e.currentTarget.value })}>
          {#each s.displays as d (d.id)}<option value={d.id}>{label(d)}</option>{/each}
        </select>
        <span class="grow"></span>
        {#if edDisp}<Toggle label={t('display.show')} checked={edDisp.enabled} onchange={(v) => act('displayEnabled', { id: edDisp.id, enabled: v })} />{/if}
      </div>
      <div class="row"><span class="caption grow">{t('display.desc')}</span><button title={t('display.copyToOthersHelp')} onclick={() => act('copyToOthers')}>{t('display.copyToOthers')}</button></div>
    </Section>
  {/if}
</div>

<style>
  .row { flex-wrap: wrap; }
  .mac-top { margin: 0 8px; flex: none; }
  .mac-top :global(section.mac) { margin-bottom: 0; }
  .mac-top :global(section.mac + section.mac) { margin-top: 10px; }
  .mac-top .row { flex-wrap: nowrap; }
  .update strong { font-size: 12px; white-space: nowrap; }
  .update button { white-space: nowrap; flex: none; }
  .update .caption { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .auto { font-size: 11px; }
  .auto :global(.toggle.mac) { line-height: 13px; min-height: 14px; }
  .analytics { display: inline-flex; align-items: center; gap: 8px; }
  /* Store판 버전 표: 이름 | 값 | 버튼 세 열, 두 버튼은 같은 폭. */
  .ver { display: grid; grid-template-columns: max-content minmax(0, 1fr) max-content; column-gap: 12px; row-gap: 8px; align-items: center; }
  .ver .k { color: var(--fg2); }
  .ver .v { display: flex; align-items: baseline; gap: 8px; min-width: 0; }
  .ver .v .caption { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ver button { width: 100%; white-space: nowrap; }
  .num { font-variant-numeric: tabular-nums; font-weight: 500; }
  .num.new { color: var(--accent); }
  .opts { display: flex; align-items: center; gap: 24px; margin-top: 12px; }
  .note { margin-left: auto; min-width: 0; text-align: right; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
