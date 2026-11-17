<script lang="ts">
  // SF Symbols 대신 쓰는 단순 선 아이콘(24x24, 현재 글자색).
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
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={P[name]} /></svg>

<style>
  svg { flex: none; }
</style>
