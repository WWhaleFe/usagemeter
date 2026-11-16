<script lang="ts">
  // 선 위치 끌기 화면(Swift LineDragOverlay.swift, macOS 전용). 편집 중인 화면 전체를 덮는 투명 창 위에
  // 0.22 검정 막 + 메뉴바선(주황)·Dock선(청록) 손잡이. 끌면 바로 설정에 반영되어 테두리가 따라 움직인다.
  // 한계 5pt ~ 화면 높이 60%, 완료 버튼 또는 Esc로 닫는다(Esc는 Swift에 없던 작은 개선).
  import { onMount } from 'svelte';
  import { ui, t, set, act } from '../lib/api.svelte';

  const b = $derived(ui.s!.border);
  let H = $state(window.innerHeight);
  // 끄는 동안 화면 값을 먼저 보여 준다(설정 반영은 응답을 기다리며 합쳐 보낸다).
  let drag = $state<{ key: 'menu' | 'dock'; v: number } | null>(null);
  const menuH = $derived(drag?.key === 'menu' ? drag.v : (b.menuLineHeight ?? 25));
  const dockH = $derived(drag?.key === 'dock' ? drag.v : (b.dockLineHeight ?? 70));

  onMount(() => {
    // 이 페이지만 배경을 투명하게(전역 body 배경을 덮어쓴다).
    document.documentElement.dataset.page = 'linedrag';
    const resize = () => (H = window.innerHeight);
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && done();
    addEventListener('resize', resize);
    addEventListener('keydown', esc);
    return () => {
      delete document.documentElement.dataset.page;
      removeEventListener('resize', resize);
      removeEventListener('keydown', esc);
    };
  });

  const done = () => act('closeLineDrag');
  const clamp = (v: number) => Math.round(Math.min(Math.max(5, v), H * 0.6));

  let sending = false, pending: { key: 'menu' | 'dock'; v: number } | null = null;
  async function push(key: 'menu' | 'dock', v: number) {
    pending = { key, v };
    if (sending) return;
    sending = true;
    while (pending) {
      const p = pending;
      pending = null;
      await set('border', p.key === 'menu' ? { menuLineHeight: p.v } : { dockLineHeight: p.v });
    }
    sending = false;
  }
  function move(e: PointerEvent, key: 'menu' | 'dock') {
    if (!(e.currentTarget as HTMLElement).hasPointerCapture(e.pointerId)) return;
    const v = clamp(key === 'menu' ? e.clientY : H - e.clientY);
    drag = { key, v };
    push(key, v);
  }
  function key(e: KeyboardEvent, k: 'menu' | 'dock') {
    const d = e.key === 'ArrowDown' ? 1 : e.key === 'ArrowUp' ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const cur = k === 'menu' ? menuH : dockH;
    // 메뉴선은 아래로 갈수록 커지고, Dock선은 위로 갈수록 커진다.
    push(k, clamp(cur + (k === 'menu' ? d : -d) * (e.shiftKey ? 10 : 1)));
  }
</script>

<div class="scrim">
  {#each [{ k: 'menu' as const, on: b.menuLineEnabled, y: menuH, name: t('part.menuLine'), cls: 'menu', min: 5, max: 200 }, { k: 'dock' as const, on: b.dockLineEnabled, y: H - dockH, name: t('part.dockLine'), cls: 'dock', min: 5, max: 400 }] as h (h.k)}
    {#if h.on}
      <div
        class="handle {h.cls}" style:top="{h.y}px" role="slider" tabindex="0" aria-label={h.name}
        aria-orientation="vertical" aria-valuemin={h.min} aria-valuemax={h.max} aria-valuenow={h.k === 'menu' ? menuH : dockH}
        onpointerdown={(e) => (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)}
        onpointermove={(e) => move(e, h.k)} onpointerup={() => (drag = null)} onpointercancel={() => (drag = null)}
        onkeydown={(e) => key(e, h.k)}
      >
        <span class="line"></span>
        <span class="chip">{h.name}  {Math.round(h.k === 'menu' ? menuH : dockH)}pt</span>
      </div>
    {/if}
  {/each}
  <button class="primary done" onclick={done}>{t('part.dragDone')}</button>
</div>

<style>
  :global(html[data-page="linedrag"]), :global(html[data-page="linedrag"] body) { background: transparent; }
  .scrim { position: fixed; inset: 0; background: rgba(0, 0, 0, 0.22); }
  /* 선 2pt, 위아래로 넓은 30pt 히트 영역(Swift :119-132). 색은 데이터 색(주황·청록). */
  .handle { position: absolute; left: 0; right: 0; height: 30px; margin-top: -15px; cursor: ns-resize; touch-action: none; outline: none; }
  .line { position: absolute; left: 0; right: 0; top: 14px; height: 2px; background: var(--hc); }
  .chip {
    position: absolute; left: 50%; top: -12px; transform: translateX(-50%); white-space: nowrap; pointer-events: none;
    background: var(--hc); color: #000; font-size: var(--fs-small); font-weight: 700; padding: 3px 8px; border-radius: 999px;
  }
  .handle:focus-visible .line { height: 4px; top: 13px; }
  .menu { --hc: #ff9500; }
  .dock { --hc: #32ade6; }
  .done { position: absolute; top: 64px; left: 50%; transform: translateX(-50%); font-size: var(--fs-title); font-weight: 600; padding: 8px 22px; }
</style>
