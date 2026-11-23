<script lang="ts">
  // 레이아웃 탭(Swift SettingsView.swift:603-1058): 겹침 금지 + AI별 배치 상자(가로선 끝 캡, 방향·시작점, 프리셋, 다이어그램).
  // Windows는 화면 가장자리 테두리뿐이라 세그먼트가 hTop·hBottom·lMain·rMain만 있다(Rust가 available로 알려 준다).
  import { ui, settingsProviders, t, set, act } from '../../lib/api.svelte';
  import type { LayoutProvider } from '../../components/BTypes';
  import Toggle from '../../components/Toggle.svelte';
  import BAiLayout from '../../components/BAiLayout.svelte';
  const s = $derived(ui.s!);
  const providers = $derived(settingsProviders(s));
  const b = $derived(s.border);

  // 겹침 금지를 켜는 순간 기존 겹침을 정리한다(Swift :613-619). 정리 결과는 Rust가 계산해 쓴다.
  async function noOverlap(on: boolean) {
    await set('border', { noOverlapLines: on });
    if (on) await act('enforceNoOverlap');
  }
</script>

<div class="box">
  <Toggle label={t('lay.noOverlap')} help={t('help.noOverlap')} checked={!!b.noOverlapLines} onchange={noOverlap} />
  <!-- 전역 호버 설정: 배치된 선 위에서만 호버 정보(표시 탭의 변·꼭짓점 호버 영역 대신). -->
  <Toggle label={t('lay.hoverFollowLayout')} help={t('help.hoverFollowLayout')} checked={s.main.hoverFollowLayout ?? true} disabled={s.main.hoverInfoEnabled === false} onchange={(v) => set('main', { hoverFollowLayout: v })} />
</div>
{#each providers as p (p.id)}
  <BAiLayout p={p as LayoutProvider} />
{/each}

<style>
  .box { display: flex; flex-direction: column; gap: 10px; margin: 0 0 10px; padding: 8px 12px; background: var(--surface); border: 1px solid var(--surface-stroke); border-radius: var(--r-card); }
  :global(:root[data-skin="macos"]) .box { padding: 12px; margin-bottom: 12px; border: 0; }
</style>
