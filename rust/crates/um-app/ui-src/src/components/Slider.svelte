<script lang="ts">
  // 슬라이더 + 숫자 칸 + 화살표(Swift LabeledSlider, SettingsView.swift:1162-1201).
  // 방향키로 한 단계씩 바꾸는 것은 슬라이더·숫자 칸이 기본으로 해 준다.
  let { label, value, min, max, step = 1, unit = '', disabled = false, onchange }:
    { label: string; value: number; min: number; max: number; step?: number; unit?: string; disabled?: boolean; onchange: (v: number) => void } = $props();
  const clamp = (v: number) => Math.min(max, Math.max(min, Math.round(v / step) * step));
  const fmt = (v: number) => (step < 1 ? v.toFixed(1).replace(/\.0$/, '') : String(Math.round(v)));
</script>

<div class="slider row" class:disabled role="group" aria-label={label}>
  <span class="label">{label}</span>
  <input class="grow" type="range" {min} {max} {step} {value} {disabled} oninput={(e) => onchange(clamp(+(e.currentTarget as HTMLInputElement).value))} />
  <input class="num" type="number" {min} {max} {step} value={fmt(value)} {disabled} onchange={(e) => onchange(clamp(+(e.currentTarget as HTMLInputElement).value))} />
  {#if unit}<span class="caption">{unit}</span>{/if}
  <span class="stepper">
    <button {disabled} aria-label="-" onclick={() => onchange(clamp(value - step))}>−</button>
    <button {disabled} aria-label="+" onclick={() => onchange(clamp(value + step))}>+</button>
  </span>
</div>

<style>
  .slider.disabled { opacity: 0.45; }
  .label { width: 150px; flex: none; }
  .num { width: 64px; text-align: right; }
  .stepper { display: inline-flex; }
  .stepper button { min-height: 22px; padding: 0 7px; border-radius: 0; }
  .stepper button:first-child { border-radius: var(--r-control) 0 0 var(--r-control); }
  .stepper button:last-child { border-radius: 0 var(--r-control) var(--r-control) 0; border-left: none; }
  input[type="range"] { accent-color: var(--accent); }
</style>
