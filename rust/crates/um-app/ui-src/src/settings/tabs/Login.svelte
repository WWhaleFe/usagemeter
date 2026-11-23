<script lang="ts">
  // 로그인 탭(Swift SettingsView.swift:213-256): AI별 상태·로그인/로그아웃, 로그인 유지, 로그인 시 자동 실행.
  import { ui, settingsProviders, t, act, set } from '../../lib/api.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  const s = $derived(ui.s!);
  const providers = $derived(settingsProviders(s));
</script>

<Section title={t('sec.accounts')} help={t('help.accounts')}>
  {#each providers as p, i (p.id)}
    <div class="row">
      <span class="dot" class:on={p.loggedIn} aria-hidden="true"></span>
      <span class="name">{p.name}</span>
      <span class="caption">{p.loggedIn ? t('acct.loggedIn') : t('acct.loggedOut')}</span>
      {#if p.experimental}<span class="caption warn">{t('acct.geminiExperimental')}</span>{/if}
      <span class="grow"></span>
      {#if p.loggedIn}
        <button onclick={() => act('logout', { id: p.id })}>{t('acct.logout')}</button>
      {:else}
        <button onclick={() => act('login', { id: p.id })}>{t('acct.login')}</button>
      {/if}
    </div>
    {#if i < providers.length - 1}<hr />{/if}
  {/each}
</Section>

<Section title={t('sec.loginOpts')}>
  <Toggle label={t('login.keep')} help={t('help.keepLogin')} checked={!!s.main.keepLoggedIn} onchange={(v) => set('main', { keepLoggedIn: v })} />
  <Toggle label={t('login.autostart')} help={t('help.autostart')} checked={s.autostart.enabled} disabled={!s.autostart.available} onchange={(v) => act('autostart', { enabled: v })} />
  {#if !s.autostart.available}<span class="caption">{t('login.autostartNote')}</span>{/if}
</Section>

<style>
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--fg3); }
  .dot.on { background: var(--ok); }
  .name { font-weight: 600; min-width: 60px; }
  :global(:root[data-skin="macos"]) .dot { width: 9px; height: 9px; flex: none; }
  :global(:root[data-skin="macos"]) .name { min-width: 0; font-size: 12px; }
  :global(:root[data-skin="macos"]) .row { gap: 10px; }
</style>
