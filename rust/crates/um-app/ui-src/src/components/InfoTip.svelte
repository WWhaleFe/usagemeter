<script lang="ts">
  // 항목 옆 (i) 아이콘. 마우스를 올리거나 키보드로 초점을 주면 그 항목의 설명을 말풍선으로 보여 준다.
  // 말풍선은 position: fixed 로 그려 스크롤 영역(탭 본문)에 잘리지 않고, 창 가장자리에서는 안쪽으로 밀린다.
  let { text }: { text: string } = $props();
  let icon: HTMLSpanElement;
  let tip = $state<HTMLDivElement>();
  let open = $state(false);
  let pos = $state({ x: 0, y: 0, below: true });
  const tipId = $props.id();

  async function show() {
    open = true;
    await Promise.resolve();
    const r = icon.getBoundingClientRect();
    const w = tip?.offsetWidth ?? 260, h = tip?.offsetHeight ?? 40;
    const x = Math.min(Math.max(8, r.left + r.width / 2 - w / 2), window.innerWidth - w - 8);
    const below = r.bottom + 6 + h <= window.innerHeight - 8;
    pos = { x, y: below ? r.bottom + 6 : r.top - 6 - h, below };
  }
  const hide = () => (open = false);
  // 레이블(체크박스·슬라이더) 안에 있어도 누를 때 그 컨트롤이 바뀌지 않게 한다.
  const swallow = (e: Event) => { e.preventDefault(); e.stopPropagation(); };
</script>

<span class="info" bind:this={icon} role="button" tabindex="0" aria-label={text} aria-describedby={open ? tipId : undefined}
  onmouseenter={show} onmouseleave={hide} onfocus={show} onblur={hide} onclick={swallow} onmousedown={swallow}
  onkeydown={(e) => e.key === 'Escape' && hide()}>
  <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6.6" /><path d="M8 7.2v4" /><circle class="dot" cx="8" cy="4.9" r=".9" /></svg>
</span>
{#if open}
  <div class="tip" id={tipId} role="tooltip" bind:this={tip} style:left="{pos.x}px" style:top="{pos.y}px">{text}</div>
{/if}

<style>
  .info { display: inline-flex; align-items: center; justify-content: center; width: 16px; height: 16px; flex: none; color: var(--fg3); cursor: help; vertical-align: -3px; margin-left: 4px; border-radius: 50%; }
  .info:hover, .info:focus-visible { color: var(--accent); }
  .info:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  svg { width: 14px; height: 14px; fill: none; stroke: currentColor; stroke-width: 1.3; stroke-linecap: round; }
  .dot { fill: currentColor; stroke: none; }
  .tip {
    position: fixed; z-index: 1000; max-width: min(300px, calc(100vw - 16px)); padding: 7px 10px;
    background: var(--bg); color: var(--fg); border: 1px solid var(--control-stroke); border-radius: 6px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.18); font-size: 12px; font-weight: 400; line-height: 1.5;
    white-space: normal; text-align: left; pointer-events: none; word-break: keep-all; overflow-wrap: anywhere;
  }
</style>
