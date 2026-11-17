<script lang="ts">
  // 세그먼트 선택(macOS segmented picker / Windows는 라디오 버튼 묶음 모양).
  let { options, value, disabled = false, onchange }:
    { options: { value: string | number; label: string }[]; value: string | number; disabled?: boolean; onchange: (v: any) => void } = $props();
</script>

<div class="seg" class:disabled role="radiogroup">
  {#each options as o (o.value)}
    <button role="radio" aria-checked={o.value === value} class:on={o.value === value} {disabled} onclick={() => onchange(o.value)}>{o.label}</button>
  {/each}
</div>

<style>
  .seg { display: inline-flex; border: 1px solid var(--control-stroke); border-radius: var(--r-control); overflow: hidden; }
  .seg.disabled { opacity: 0.45; }
  button { border: none; border-radius: 0; background: var(--control); min-height: 24px; padding: 2px 12px; }
  button + button { border-left: 1px solid var(--control-stroke); }
  button.on { background: var(--accent); color: var(--accent-fg); }
  :global(:root[data-skin="macos"]) .seg { display: flex; width: 100%; border-radius: 5px; background: var(--segment-bg); }
  :global(:root[data-skin="macos"]) button { flex: 1; min-height: 20px; padding: 1px 10px; background: transparent; line-height: 16px; }
  :global(:root[data-skin="macos"]) button.on { background: var(--segment-selected); color: var(--fg); border-radius: 4px; }
</style>
