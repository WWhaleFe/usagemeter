<script lang="ts">
  // macOS는 Swift의 SF Symbol 이름별 도형, Windows는 기존 선 아이콘을 사용한다.
  import { ui } from '../lib/api.svelte';
  import { systemSymbol } from '../lib/symbols';
  let { name, size = 14 }: { name: string; size?: number } = $props();
  const P: Record<string, string> = {
    send: 'M3 11 21 3l-8 18-2-8z M11 13 21 3',
    copy: 'M8 8h12v13H8z M16 8V3H3v13h5',
    check: 'm4 12 5 5L20 6',
    link: 'm10 14 4-4 M8 16l-1 1a4 4 0 0 1-6-6l4-4a4 4 0 0 1 6 0 M16 8l1-1a4 4 0 0 1 6 6l-4 4a4 4 0 0 1-6 0',
    bars: 'M5 20V11M12 20V4M19 20v-6',
    bars2: 'M4 20v-6M9 20V8M14 20v-9M19 20V4',
    pie: 'M12 3a9 9 0 1 0 9 9h-9z M15 3.5A9 9 0 0 1 20.5 9H15z',
    clock: 'M12 3a9 9 0 1 0 0 18a9 9 0 0 0 0-18z M12 7v5l3.5 2',
    line: 'M3 5l5 7 4-3 9 10',
    grid: 'M4 4h4v4H4z M10 4h4v4h-4z M16 4h4v4h-4z M4 10h4v4H4z M10 10h4v4h-4z M16 10h4v4h-4z M4 16h4v4H4z M10 16h4v4h-4z M16 16h4v4h-4z',
    table: 'M4 5h16v14H4z M4 10h16 M10 5v14',
    info: 'M12 3a9 9 0 1 0 0 18a9 9 0 0 0 0-18z M12 11v5 M12 8v.01',
    warn: 'M12 4l9 16H3z M12 10v4 M12 17v.01',
    cal: 'M4 6h16v14H4z M4 10h16 M8 3v4 M16 3v4',
    octagon: 'M8 3h8l5 5v8l-5 5H8l-5-5V8z M12 8v5 M12 16v.01',
    gear: 'M12 9a3 3 0 1 0 0 6a3 3 0 0 0 0-6z M12 3v3 M12 18v3 M3 12h3 M18 12h3 M5.6 5.6l2.1 2.1 M16.3 16.3l2.1 2.1 M5.6 18.4l2.1-2.1 M16.3 7.7l2.1-2.1',
    doc: 'M6 3h9l4 4v14H6z M9 17v-3 M12 17v-6 M15 17v-4',
  };
  const MAC_NAMES: Record<string, string> = {
    send: 'paperplane.fill', copy: 'doc.on.doc', check: 'checkmark', link: 'link',
    bars: 'chart.bar.fill', bars2: 'chart.bar', pie: 'chart.pie', clock: 'clock',
    line: 'chart.line.downtrend.xyaxis', grid: 'square.grid.3x3.fill', table: 'tablecells',
    info: 'info.circle', warn: 'exclamationmark.triangle', cal: 'calendar',
    octagon: 'exclamationmark.octagon', gear: 'gearshape', doc: 'chart.bar.doc.horizontal',
  };
  const symbol = $derived(MAC_NAMES[name] ?? name);
  const fallback = $derived(P[name] ?? P[Object.keys(MAC_NAMES).find((k) => MAC_NAMES[k] === name) ?? ''] ?? ({
    'chart.bar.xaxis': P.bars, 'clock.fill': P.clock,
    'chevron.up': 'm6 15 6-6 6 6', 'chevron.down': 'm6 9 6 6 6-6',
  } as Record<string, string>)[name] ?? P.info);
  let mask = $state<string | null>(null);
  $effect(() => {
    const mac = ui.s?.platform === 'mac';
    const requestedSymbol = symbol;
    const requestedSize = size;
    let active = true;
    mask = null;
    if (mac) systemSymbol(requestedSymbol, requestedSize).then((data) => { if (active) mask = data; });
    return () => { active = false; };
  });

</script>

{#if ui.s?.platform === 'mac' && mask}
  <span class="symbol" style:width="{size}px" style:height="{size}px" style:mask-image={`url("${mask}")`} aria-hidden="true"></span>
{:else}
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={fallback} /></svg>
{/if}

<style>
  svg, .symbol { flex: none; }
  .symbol { display: inline-block; background: currentColor; mask-size: contain; mask-position: center; mask-repeat: no-repeat; }
</style>
