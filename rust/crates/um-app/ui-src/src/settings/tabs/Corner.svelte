<script lang="ts">
  // 모서리 곡률 탭(Swift SettingsView.swift:639-675): 영역(메뉴바·본문·Dock)별 꼭짓점 곡률.
  // 쓰이지 않는 꼭짓점은 비활성(`cornerUsed`, Rust가 세그먼트 그래프로 계산). Windows는 본문뿐(메뉴바선·Dock선 없음).
  import { ui, t, set } from '../../lib/api.svelte';
  import type { StateEx } from '../../components/BTypes';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import Slider from '../../components/Slider.svelte';

  type Zone = 'menuBar' | 'main' | 'dock';
  type Radii = Record<string, number>;
  const s = $derived(ui.s as StateEx);
  const b = $derived(s.border);
  const menuOn = $derived(!!b.menuLineEnabled);
  const dockOn = $derived(!!b.dockLineEnabled);
  // 영역 -> 설정 키(본문은 cornerRadii 재사용).
  const KEY: Record<Zone, string> = { menuBar: 'menuZoneRadii', main: 'cornerRadii', dock: 'dockZoneRadii' };
  const zones = $derived((['menuBar', 'main', 'dock'] as Zone[]).filter((z) => z === 'main' || (z === 'menuBar' ? menuOn : dockOn)));
  // 왼쪽 열 = 좌상·좌하, 오른쪽 열 = 우상·우하(Swift :649).
  const CORNERS = ['topLeft', 'topRight', 'bottomLeft', 'bottomRight'];
  const used = (z: Zone, c: string): boolean => !!s.cornerUsed?.[z]?.[c];
  const radius = (z: Zone, c: string): number => ((b[KEY[z]] ?? {}) as Radii)[c] ?? 0;

  // 곡률 바꾸기 + 경계선을 공유하는 이웃 영역 꼭짓점 연동(Swift setZoneRadius, OverlaySettings.swift:332-352).
  function setRadius(z: Zone, c: string, v: number) {
    const d: Record<Zone, Radii> = { menuBar: { ...(b.menuZoneRadii ?? {}) }, main: { ...(b.cornerRadii ?? {}) }, dock: { ...(b.dockZoneRadii ?? {}) } };
    d[z][c] = v;
    if (b.linkMenuMainRadii ?? true) {
      if (z === 'menuBar' && c === 'bottomLeft') d.main.topLeft = v;
      if (z === 'menuBar' && c === 'bottomRight') d.main.topRight = v;
      if (z === 'main' && c === 'topLeft') d.menuBar.bottomLeft = v;
      if (z === 'main' && c === 'topRight') d.menuBar.bottomRight = v;
    }
    if (b.linkMainDockRadii ?? true) {
      if (z === 'main' && c === 'bottomLeft') d.dock.topLeft = v;
      if (z === 'main' && c === 'bottomRight') d.dock.topRight = v;
      if (z === 'dock' && c === 'topLeft') d.main.bottomLeft = v;
      if (z === 'dock' && c === 'topRight') d.main.bottomRight = v;
    }
    set('border', { menuZoneRadii: d.menuBar, cornerRadii: d.main, dockZoneRadii: d.dock });
  }
</script>

<Section title={t('sec.zoneRadii')} help={t('help.zoneRadii')}>
  {#each zones as z, i (z)}
    <div class="zone">
      {#if menuOn || dockOn}<h4>{t('zone.' + z)}</h4>{/if}
      <div class="grid">
        {#each CORNERS as c (c)}
          <Slider label={t('corner.' + c)} value={radius(z, c)} min={0} max={80} unit="pt" disabled={!used(z, c)} onchange={(v) => setRadius(z, c, v)} />
        {/each}
      </div>
    </div>
    <!-- 곡률 연결: 경계선을 공유하는 이웃 영역과 맞닿은 꼭짓점을 함께 조절(Swift :661-668). -->
    {#if z === 'menuBar'}
      <Toggle label={t('corner.linkMenuMain')} help={t('help.linkRadii')} checked={b.linkMenuMainRadii ?? true} onchange={(v) => set('border', { linkMenuMainRadii: v })} />
    {/if}
    {#if z === 'main' && dockOn}
      <Toggle label={t('corner.linkMainDock')} help={t('help.linkRadii')} checked={b.linkMainDockRadii ?? true} onchange={(v) => set('border', { linkMainDockRadii: v })} />
    {/if}
    {#if i < zones.length - 1}<hr />{/if}
  {/each}
</Section>

<style>
  h4 { margin: 0 0 4px; font-size: var(--fs); font-weight: 600; }
  .grid { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); column-gap: 16px; row-gap: 4px; }
  /* 두 열에 들어가도록 슬라이더 라벨 폭을 줄인다. */
  .grid :global(.label) { width: 72px; }
  .grid :global(input.num) { width: 52px; }
  hr { border: none; border-top: 1px solid var(--surface-stroke); margin: 2px 0; width: 100%; }
</style>
