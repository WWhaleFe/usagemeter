<script lang="ts">
  // 두 플랫폼이 Swift 기본 체크박스 모양을 함께 쓴다.
  import InfoTip from './InfoTip.svelte';
  let { label, checked, disabled = false, help = '', onchange }: { label: string; checked: boolean; disabled?: boolean; help?: string; onchange: (v: boolean) => void } = $props();
</script>

<label class="toggle mac" class:disabled>
  <span class="text">{label}{#if help}<InfoTip text={help} />{/if}</span>
  <input type="checkbox" {checked} {disabled} onchange={(e) => onchange((e.currentTarget as HTMLInputElement).checked)} />
</label>

<style>
  .toggle { display: flex; align-items: center; gap: 8px; min-height: 26px; }
  .toggle.disabled { opacity: 0.45; }
  .text { flex: 1; }
  input { appearance: none; width: 32px; height: 18px; border-radius: 9px; background: var(--fg3); position: relative; margin: 0; flex: none; transition: background 0.15s; }
  input::after { content: ""; position: absolute; top: 2px; left: 2px; width: 14px; height: 14px; border-radius: 50%; background: #fff; transition: transform 0.15s; box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25); }
  input:checked { background: var(--accent); }
  input:checked::after { transform: translateX(14px); }
  .mac { flex-direction: row-reverse; justify-content: flex-end; gap: 5px; min-height: 16px; line-height: 16px; }
  .mac .text { flex: none; }
  .mac input { transition: none; }
  .mac.disabled input { opacity: 1; }
  .mac input::after { content: none; }
</style>
