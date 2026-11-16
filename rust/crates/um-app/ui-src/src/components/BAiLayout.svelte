<script lang="ts">
  // AI 한 개의 배치 상자(Swift aiLayoutBox/segmentEditor/segmentPresetsAndDiagram, SettingsView.swift:~615-935).
  // 좌측 = 이름 + 가로선 끝 캡 + 방향·시작점, 우측 = 프리셋 2x2 + 다이어그램.
  // 쓰기는 항상 `providerLayouts` 전체 맵에서 이 AI 항목만 바꿔서 보낸다.
  import { ui, t, set } from '../lib/api.svelte';
  import type { LayoutProvider, Layout, SegCap } from './BTypes';
  import Toggle from './Toggle.svelte';
  import BSegDiagram from './BSegDiagram.svelte';

  let { p }: { p: LayoutProvider } = $props();
  const s = $derived(ui.s!);
  const b = $derived(s.border);
  const win = $derived(s.platform === 'win');
  const menuOn = $derived(!!b.menuLineEnabled);
  const dockOn = $derived(!!b.dockLineEnabled);
  const l = $derived(p.layout);
  const segs = $derived(new Set(p.layout.segments ?? []));
  const caps = $derived((l.hCaps ?? {}) as Record<string, SegCap>);

  function mutate(patch: Partial<Layout>) {
    const all = { ...((b.providerLayouts ?? {}) as Record<string, Layout>), [p.id]: { ...l, ...patch } };
    return set('border', { providerLayouts: all });
  }
  const setSegs = (next: string[]) => mutate({ segments: next });

  // 다이어그램 클릭 = 토글(Rust가 유효하다고 한 경우만), 더블클릭 = 그 선만.
  const ORDER = ['hTop', 'hMenu', 'hDock', 'hBottom', 'lMenuBar', 'lMain', 'lDock', 'rMenuBar', 'rMain', 'rDock'];
  function toggle(seg: string) {
    const o = p.segOptions[seg];
    if (!o?.toggleAllowed) return;
    const next = new Set(segs);
    if (!next.delete(seg)) next.add(seg);
    setSegs(ORDER.filter((x) => next.has(x)));
  }
  // '겹치는 선 없음' 상태에서 다른 AI가 쓰는 선이면 경고만 1.6초 띄우고 진행하지 않는다(Swift exclusiveSelect).
  let warning = $state('');
  let warnTimer: ReturnType<typeof setTimeout> | undefined;
  function exclusive(seg: string) {
    const o = p.segOptions[seg];
    if (!o?.available) return;
    if (b.noOverlapLines && o.usedByOther) {
      warning = t('lay.overlapWarn');
      clearTimeout(warnTimer);
      warnTimer = setTimeout(() => (warning = ''), 1600);
      return;
    }
    setSegs([seg]);
  }

  // 가로선 끝 캡: 같은 끝의 반대 방향은 자동 해제(Swift hCapFlag).
  const H_ORDER = ['hTop', 'hMenu', 'hDock', 'hBottom'];
  const hsSel = $derived(H_ORDER.filter((h) => segs.has(h)));
  const capKey = (h: string, right: boolean) => `${h}.${right ? 'R' : 'L'}`;
  const capOf = (h: string, right: boolean): SegCap => caps[capKey(h, right)] ?? 'none';
  function setCap(h: string, right: boolean, dir: 'up' | 'down', on: boolean) {
    mutate({ hCaps: { ...caps, [capKey(h, right)]: on ? dir : 'none' } });
  }

  // 모양 프리셋: 영역 둘레 + 끝 캡 묶음(Swift presetButton). Windows는 메뉴바·Dock 프리셋 없음.
  const PRESETS = $derived<{ key: 'all' | 'menuBar' | 'main' | 'dock'; label: string; off: boolean; caps: Record<string, SegCap> }[]>([
    { key: 'all', label: t('lay.zoneAll'), off: false, caps: {} },
    ...(win ? [] : [{ key: 'menuBar' as const, label: t('zone.menuBar'), off: !menuOn, caps: { 'hMenu.L': 'down', 'hMenu.R': 'down' } as Record<string, SegCap> }]),
    { key: 'main', label: t('zone.main'), off: false, caps: {} },
    ...(win ? [] : [{ key: 'dock' as const, label: t('zone.dock'), off: !dockOn, caps: { 'hDock.L': 'up', 'hDock.R': 'up' } as Record<string, SegCap> }]),
  ]);
  function applyPreset(key: 'all' | 'menuBar' | 'main' | 'dock', hCaps: Record<string, SegCap>) {
    const sp = p.shapePresets[key];
    if (!sp?.allowed) return;
    mutate({ zones: sp.zones, segments: sp.segments, hCaps });
  }

  const CORNERS = ['topLeft', 'topRight', 'bottomRight', 'bottomLeft'];
  // 곡선 모서리 시작 지점: 곡률 있는 일반 모서리 = 위/아래, 스쿱 = 위/모서리/아래(저장은 left/center/right).
  const sideOptions = $derived(
    p.anchorSideKind === 'scoop'
      ? [{ v: 'left', l: t('side.up') }, { v: 'center', l: t('side.corner') }, { v: 'right', l: t('side.down') }]
      : [{ v: 'left', l: t('side.up') }, { v: 'right', l: t('side.down') }],
  );
  const sideValue = $derived(p.anchorSideKind === 'scoop' ? l.anchorSide : l.anchorSide === 'right' ? 'right' : 'left');
</script>

<section class="box">
  <div class="left">
    <h3>{p.name}</h3>
    {#if hsSel.length}
      <div class="caps">
        <span class="caption">{t('lay.hCapsTitle')}</span>
        {#each hsSel as h (h)}
          <div class="caprow" role="group" aria-label={t('seg.' + h)}>
            <span class="segname caption">{t('seg.' + h)}</span>
            {#each [false, true] as right (right)}
              <span class="end caption">{t(right ? 'cap.rightEnd' : 'cap.leftEnd')}</span>
              <label title={t('cap.upHelp')} class:off={h === 'hTop'}>
                <input type="checkbox" disabled={h === 'hTop'} checked={capOf(h, right) === 'up'} onchange={(e) => setCap(h, right, 'up', e.currentTarget.checked)} />{t('cap.upShort')}
              </label>
              <label title={t('cap.downHelp')} class:off={h === 'hBottom'}>
                <input type="checkbox" disabled={h === 'hBottom'} checked={capOf(h, right) === 'down'} onchange={(e) => setCap(h, right, 'down', e.currentTarget.checked)} />{t('cap.downShort')}
              </label>
            {/each}
          </div>
        {/each}
      </div>
    {/if}
    {#if p.chain}
      <!-- 열린 체인: 방향 반전(= clockwise). -->
      <Toggle label={t('lay.reverse')} checked={l.clockwise} onchange={(v) => mutate({ clockwise: v })} />
    {:else}
      <div class="row">
        <span class="grow">{t('lay.anchor')}</span>
        <select value={l.anchorCorner} onchange={(e) => mutate({ anchorCorner: e.currentTarget.value })}>
          {#each CORNERS as c (c)}<option value={c}>{t('corner.' + c)}</option>{/each}
        </select>
      </div>
      <Toggle label={t('lay.clockwise')} checked={l.clockwise} onchange={(v) => mutate({ clockwise: v })} />
      {#if p.anchorSideKind !== 'hidden'}
        <div class="row">
          <span class="grow">{t('lay.anchorSide')}</span>
          <select value={sideValue} onchange={(e) => mutate({ anchorSide: e.currentTarget.value })}>
            {#each sideOptions as o (o.v)}<option value={o.v}>{o.l}</option>{/each}
          </select>
        </div>
      {/if}
    {/if}
    <span class="caption">{t('lay.dblHint')}</span>
  </div>
  <div class="right">
    <div class="presets">
      {#each PRESETS as pr (pr.key)}
        <button class="small" disabled={pr.off} onclick={() => applyPreset(pr.key, pr.caps)}>{pr.label}</button>
      {/each}
    </div>
    <BSegDiagram options={p.segOptions} color={p.color} {menuOn} {dockOn} {warning} ontoggle={toggle} onexclusive={exclusive} />
  </div>
</section>

<style>
  .box {
    display: flex; flex-wrap: wrap; align-items: flex-start; gap: 10px 14px; margin: 0 0 10px; padding: 10px 12px;
    background: var(--surface); border: 1px solid var(--surface-stroke); border-radius: var(--r-card);
  }
  :global(:root[data-skin="win"]) .box { padding: 12px 16px; margin-bottom: 12px; }
  .left { flex: 1 1 340px; min-width: 0; display: flex; flex-direction: column; gap: 6px; }
  .right { flex: none; width: 209px; display: flex; flex-direction: column; gap: 4px; }
  h3 { margin: 0; font-size: var(--fs-title); font-weight: 600; }
  .caps { display: flex; flex-direction: column; gap: 3px; }
  .caprow { display: flex; align-items: center; gap: 4px 6px; flex-wrap: wrap; }
  .segname { width: 84px; white-space: nowrap; }
  .end { margin-left: 2px; }
  .caprow label { display: inline-flex; align-items: center; gap: 3px; font-size: var(--fs-small); white-space: nowrap; }
  .caprow label.off { opacity: 0.45; }
  input[type="checkbox"] { accent-color: var(--accent); margin: 0; }
  /* 프리셋 2x2: 다이어그램 폭에 맞춘다(Swift :762-780). */
  .presets { display: grid; grid-template-columns: 1fr 1fr; gap: 5px; width: 209px; }
  .small { min-height: 22px; padding: 1px 6px; font-size: var(--fs-small); }
  :global(:root[data-skin="win"]) .small { min-height: 28px; }
</style>
