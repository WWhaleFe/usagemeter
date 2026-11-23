<script lang="ts">
  // 로컬 사용 기록(토큰 통계) 설정(계획서 8장, 결정 D4: 기본 꺼짐, 언제든 바꿀 수 있음).
  // 소스별 켜기·끄기, 프로젝트 이름 숨기기, 마지막 수집 시각, 수집한 기록 지우기.
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { t, tf } from '../lib/api.svelte';
  import Section from '../components/Section.svelte';
  import Toggle from '../components/Toggle.svelte';

  interface LL { asked: boolean; sources: { id: string; enabled: boolean; found: boolean }[]; hideProjects: boolean; lastScan: number | null; scanning: boolean; events: number }
  let ll = $state<LL | null>(null);
  const load = async () => (ll = (await invoke('ui_locallogs')) as LL);
  const set = async (action: string, arg: Record<string, unknown> = {}) => { await invoke('ui_locallogs_set', { action, arg }); await load(); };
  onMount(() => {
    load();
    const un = listen('ui-state', () => load());
    return () => { un.then((f) => f()); };
  });
  const when = (s: number) => new Date(s * 1000).toLocaleString(undefined, { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
</script>

{#if ll}
  <Section title={t('locallogs.title')} help={t('help.localLogs')}>
    <span class="caption">{t('locallogs.desc')}</span>
    {#each ll.sources as s (s.id)}
      <Toggle label={t('locallogs.source.' + s.id) + (s.found ? '' : ' (-)')} checked={s.enabled} disabled={!s.found && !s.enabled} onchange={(v) => set('source', { id: s.id, enabled: v })} />
    {/each}
    <Toggle label={t('locallogs.hideProjects')} help={t('help.hideProjects')} checked={ll.hideProjects} onchange={(v) => set('hideProjects', { on: v })} />
    <div class="row">
      <span class="caption grow">{ll.scanning ? '…' : ll.lastScan ? tf('locallogs.lastScan', when(ll.lastScan)) : ''}</span>
      <button disabled={ll.events === 0} onclick={() => set('clear')}>{t('locallogs.clear')}</button>
    </div>
  </Section>
{/if}
