<script lang="ts">
  // 켜기/끄기. macOS = 체크박스 모양 스위치, Windows = Fluent 토글 스위치(라벨 왼쪽, 스위치 오른쪽).
  let { label, checked, disabled = false, onchange }: { label: string; checked: boolean; disabled?: boolean; onchange: (v: boolean) => void } = $props();
</script>

<label class="toggle" class:disabled>
  <span class="text">{label}</span>
  <input type="checkbox" role="switch" {checked} {disabled} onchange={(e) => onchange((e.currentTarget as HTMLInputElement).checked)} />
</label>

<style>
  .toggle { display: flex; align-items: center; gap: 8px; min-height: 26px; }
  .toggle.disabled { opacity: 0.45; }
  .text { flex: 1; }
  input { appearance: none; width: 32px; height: 18px; border-radius: 9px; background: var(--fg3); position: relative; margin: 0; flex: none; transition: background 0.15s; }
  input::after { content: ""; position: absolute; top: 2px; left: 2px; width: 14px; height: 14px; border-radius: 50%; background: #fff; transition: transform 0.15s; box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25); }
  input:checked { background: var(--accent); }
  input:checked::after { transform: translateX(14px); }
  :global(:root[data-skin="win"]) input { width: 40px; height: 20px; border-radius: 10px; background: transparent; border: 1px solid var(--fg2); }
  :global(:root[data-skin="win"]) input::after { top: 3px; left: 3px; width: 12px; height: 12px; background: var(--fg2); box-shadow: none; }
  :global(:root[data-skin="win"]) input:checked { background: var(--accent); border-color: var(--accent); }
  :global(:root[data-skin="win"]) input:checked::after { transform: translateX(20px); background: var(--accent-fg); }
</style>
