<script lang="ts">
  // Windows 색 선택(오피스식): 테마 색 10열 x 밝게·어둡게 단계, 표준 색, 최근 색, HEX 입력, 사용자 지정.
  // macOS는 시스템 색상 패널(act('pickColor'))을 그대로 쓴다.
  import { ui } from '../lib/api.svelte';

  let { value, disabled = false, label = '', title = '', onpick }: {
    value: string; disabled?: boolean; label?: string; title?: string; onpick: (hex: string) => void;
  } = $props();

  const L = {
    ko: { theme: '테마 색', standard: '표준 색', recent: '최근 사용한 색', more: '다른 색...', hex: 'HEX' },
    en: { theme: 'Theme colors', standard: 'Standard colors', recent: 'Recent colors', more: 'More colors...', hex: 'HEX' },
    ja: { theme: 'テーマの色', standard: '標準の色', recent: '最近使用した色', more: 'その他の色...', hex: 'HEX' },
  };
  const tx = $derived(L[(ui.s?.lang ?? 'en') as keyof typeof L] ?? L.en);

  const BASE = ['#FFFFFF', '#000000', '#E7E6E6', '#44546A', '#4472C4', '#ED7D31', '#A5A5A5', '#FFC000', '#5B9BD5', '#70AD47'];
  const STANDARD = ['#C00000', '#FF0000', '#FFC000', '#FFFF00', '#92D050', '#00B050', '#00B0F0', '#0070C0', '#002060', '#7030A0'];

  const rgb = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
  const hex = (c: number[]) => '#' + c.map((v) => Math.round(Math.max(0, Math.min(255, v))).toString(16).padStart(2, '0')).join('').toUpperCase();
  const mix = (h: string, k: number) => hex(rgb(h).map((v) => (k > 0 ? v + (255 - v) * k : v * (1 + k))));
  // 오피스와 같은 단계: 흰색은 어둡게만, 검정은 밝게만, 나머지는 밝게 80·60·40%와 어둡게 25·50%.
  function steps(h: string): number[] {
    const lum = rgb(h).reduce((a, v) => a + v, 0) / 765;
    if (lum > 0.9) return [-0.05, -0.15, -0.25, -0.35, -0.5];
    if (lum < 0.1) return [0.5, 0.35, 0.25, 0.15, 0.05];
    return [0.8, 0.6, 0.4, -0.25, -0.5];
  }
  const shades = BASE.map((b) => steps(b).map((k) => mix(b, k)));

  const KEY = 'um.recentColors';
  function loadRecent(): string[] {
    try { const v = JSON.parse(localStorage.getItem(KEY) ?? '[]'); return Array.isArray(v) ? v.slice(0, 10) : []; } catch { return []; }
  }
  let recent = $state<string[]>(loadRecent());

  let open = $state(false);
  let well: HTMLButtonElement;
  let pop = $state<HTMLDivElement>();
  let native = $state<HTMLInputElement>();
  let pos = $state({ left: 0, top: 0 });
  let draft = $state('');

  const norm = (h: string) => h.toUpperCase();
  const same = (a: string, b: string) => norm(a) === norm(b);

  function choose(h: string) {
    const v = norm(h);
    recent = [v, ...recent.filter((r) => !same(r, v))].slice(0, 10);
    try { localStorage.setItem(KEY, JSON.stringify(recent)); } catch { /* 저장 못 해도 선택은 된다 */ }
    onpick(v.toLowerCase());
    open = false;
  }

  const W = 252, H = 330;
  function toggle() {
    if (disabled) return;
    if (open) { open = false; return; }
    const r = well.getBoundingClientRect();
    const left = Math.max(8, Math.min(r.left, window.innerWidth - W - 8));
    const below = r.bottom + 4;
    const top = below + H > window.innerHeight - 8 ? Math.max(8, r.top - H - 4) : below;
    pos = { left, top };
    draft = norm(value).slice(1);
    open = true;
  }

  function onHex(e: KeyboardEvent) {
    if (e.key !== 'Enter') return;
    const v = draft.trim().replace(/^#/, '');
    if (/^[0-9a-fA-F]{6}$/.test(v)) choose('#' + v);
    else if (/^[0-9a-fA-F]{3}$/.test(v)) choose('#' + [...v].map((c) => c + c).join(''));
  }

  $effect(() => {
    if (!open) return;
    const down = (e: PointerEvent) => {
      const t = e.target as Node;
      if (!pop?.contains(t) && !well.contains(t)) open = false;
    };
    const key = (e: KeyboardEvent) => { if (e.key === 'Escape') { open = false; well.focus(); } };
    const close = () => (open = false);
    window.addEventListener('pointerdown', down, true);
    window.addEventListener('keydown', key);
    window.addEventListener('resize', close);
    window.addEventListener('scroll', close, true);
    return () => {
      window.removeEventListener('pointerdown', down, true);
      window.removeEventListener('keydown', key);
      window.removeEventListener('resize', close);
      window.removeEventListener('scroll', close, true);
    };
  });
</script>

<button bind:this={well} class="well" {disabled} {title} aria-label={label} aria-haspopup="dialog" aria-expanded={open} onclick={toggle}>
  <span style:background={value}></span>
</button>

{#if open}
  <div bind:this={pop} class="pop" role="dialog" aria-label={label} style:left="{pos.left}px" style:top="{pos.top}px">
    <div class="head">{tx.theme}</div>
    <div class="grid">
      {#each BASE as c (c)}
        <button class="sw" class:on={same(c, value)} style:background={c} title={c} aria-label={c} onclick={() => choose(c)}></button>
      {/each}
    </div>
    <div class="grid cols">
      {#each [0, 1, 2, 3, 4] as row (row)}
        {#each shades as col, i (i)}
          {@const c = col[row]}
          <button class="sw" class:on={same(c, value)} style:background={c} title={c} aria-label={c} onclick={() => choose(c)}></button>
        {/each}
      {/each}
    </div>
    <div class="head">{tx.standard}</div>
    <div class="grid">
      {#each STANDARD as c (c)}
        <button class="sw" class:on={same(c, value)} style:background={c} title={c} aria-label={c} onclick={() => choose(c)}></button>
      {/each}
    </div>
    {#if recent.length}
      <div class="head">{tx.recent}</div>
      <div class="grid">
        {#each recent as c (c)}
          <button class="sw" class:on={same(c, value)} style:background={c} title={c} aria-label={c} onclick={() => choose(c)}></button>
        {/each}
      </div>
    {/if}
    <hr />
    <div class="foot">
      <span class="cur" style:background={'#' + (/^[0-9a-fA-F]{6}$/.test(draft) ? draft : norm(value).slice(1))}></span>
      <label class="hex"><span class="caption">{tx.hex} #</span><input type="text" maxlength="7" spellcheck="false" bind:value={draft} onkeydown={onHex} /></label>
      <button class="more" onclick={() => native?.click()}>{tx.more}</button>
      <input bind:this={native} class="native" type="color" tabindex="-1" aria-hidden="true" value={value} onchange={(e) => choose(e.currentTarget.value)} />
    </div>
  </div>
{/if}

<style>
  .well { width: 36px; height: 24px; min-height: 0; padding: 3px; flex: none; border: 1px solid var(--control-stroke); border-radius: var(--r-control); background: var(--control); }
  .well span { display: block; width: 100%; height: 100%; border-radius: 2px; box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.15); }
  .pop {
    position: fixed; z-index: 1000; width: 252px; padding: 10px;
    background: var(--bg); border: 1px solid var(--control-stroke); border-radius: var(--r-card);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.22);
  }
  .head { font-size: var(--fs-small); color: var(--fg2); margin: 6px 0 4px; }
  .head:first-child { margin-top: 0; }
  .grid { display: grid; grid-template-columns: repeat(10, 20px); gap: 3px; }
  .grid.cols { margin-top: 5px; row-gap: 0; }
  .sw { width: 20px; height: 20px; min-height: 0; padding: 0; border: 1px solid rgba(0, 0, 0, 0.16); border-radius: 2px; }
  .sw:hover:not(:disabled) { outline: 2px solid var(--accent); outline-offset: 0; z-index: 1; }
  .sw.on { outline: 2px solid var(--accent); outline-offset: 1px; z-index: 1; }
  hr { border: 0; border-top: 1px solid var(--surface-stroke); margin: 10px 0 8px; }
  .foot { display: flex; align-items: center; gap: 6px; }
  .cur { width: 20px; height: 20px; flex: none; border-radius: 2px; border: 1px solid rgba(0, 0, 0, 0.16); }
  .hex { display: inline-flex; align-items: center; gap: 2px; }
  .hex input { width: 64px; font-variant-numeric: tabular-nums; text-transform: uppercase; }
  .more { margin-left: auto; padding: 2px 8px; white-space: nowrap; }
  .native { position: absolute; width: 0; height: 0; opacity: 0; pointer-events: none; border: 0; padding: 0; }
</style>
