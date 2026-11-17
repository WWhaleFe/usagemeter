<script lang="ts">
  // 슬라이더 + 숫자 칸 + 화살표(Swift LabeledSlider, SettingsView.swift:1162-1201).
  // macOS는 레이블 위, 눈금 슬라이더와 숫자 칸 및 세로 스테퍼 아래로 배치한다.
  import { ui } from '../lib/api.svelte';
  let { label, value, min, max, step = 1, unit = '', disabled = false, onchange }:
    { label: string; value: number; min: number; max: number; step?: number; unit?: string; disabled?: boolean; onchange: (v: number) => void } = $props();
  const sliderId = $props.id();
  const clampValue = (v: number) => Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : value;
  function arrow(e: KeyboardEvent) {
    if (disabled || !['ArrowUp', 'ArrowDown', 'ArrowRight', 'ArrowLeft'].includes(e.key)) return;
    e.preventDefault();
    onchange(clampValue(value + (e.key === 'ArrowUp' || e.key === 'ArrowRight' ? step : -step)));
  }
  const clamp = (v: number) => Math.min(max, Math.max(min, Math.round(v / step) * step));
  const fmt = (v: number) => (step < 1 ? v.toFixed(1).replace(/\.0$/, '') : String(Math.round(v)));
</script>

{#if ui.s?.platform === 'win'}
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

{:else}
  <div class="mac-slider" class:disabled role="group" aria-label={label}>
    <label class="label" for={sliderId}>{label}</label>
    <div class="controls">
      <input id={sliderId} class="track" type="range" onkeydown={arrow} {min} {max} {step} {value} {disabled} style:--ticks={Math.round((max - min) / step)} aria-label={label} oninput={(e) => onchange(+e.currentTarget.value)} />
      <input class="num" type="number" onkeydown={arrow} {min} {max} step="any" value={value.toFixed(1).replace(/\.0$/, '')} {disabled} aria-label={label} onchange={(e) => onchange(clampValue(+e.currentTarget.value))} />
      <span class="mac-stepper">
        <button {disabled} aria-label={label + ' +'} onclick={() => onchange(clampValue(value + step))}>⌃</button>
        <button {disabled} aria-label={label + ' −'} onclick={() => onchange(clampValue(value - step))}>⌄</button>
      </span>
    </div>
  </div>
{/if}

<style>
  .slider.disabled { opacity: 0.45; }
  .label { width: 150px; flex: none; }
  .num { width: 64px; text-align: right; }
  .stepper { display: inline-flex; }
  .stepper button { min-height: 22px; padding: 0 7px; border-radius: 0; }
  .stepper button:first-child { border-radius: var(--r-control) 0 0 var(--r-control); }
  .stepper button:last-child { border-radius: 0 var(--r-control) var(--r-control) 0; border-left: none; }
  input[type="range"] { accent-color: var(--accent); }
  .mac-slider { display: flex; flex-direction: column; gap: 3px; padding: 4px; min-width: 0; }
  .mac-slider:focus-within { outline: 2px solid var(--accent); border-radius: 6px; }
  .mac-slider input:focus-visible { outline: none; }
  .mac-slider.disabled { opacity: .4; }
  .mac-slider .label { width: auto; font-size: 12px; line-height: 15px; }
  .controls { display: flex; align-items: center; gap: 8px; height: 20px; }
  .mac-slider .num { width: 48px; flex: none; appearance: textfield; font-variant-numeric: tabular-nums; }
  .mac-slider .num::-webkit-inner-spin-button { appearance: none; }
  .track { appearance: none; flex: 1; min-width: 0; margin: 0; height: 20px; background: transparent; }
  .track::-webkit-slider-runnable-track { height: 8px; background: repeating-linear-gradient(to right, var(--slider-tick) 0 2px, transparent 2px calc(100% / var(--ticks))), linear-gradient(var(--slider-line), var(--slider-line)) center / 100% 3px no-repeat; }
  .track::-webkit-slider-thumb { appearance: none; width: 8px; height: 20px; border-radius: 4px; background: var(--slider-thumb); margin-top: -6px; }
  .mac-stepper { display: flex; flex-direction: column; flex: none; width: 13px; border-radius: 4px; overflow: hidden; background: var(--control); }
  .mac-stepper button { border: none; border-radius: 0; padding: 0; width: 13px; height: 10px; min-height: 0; line-height: 10px; font-size: 12px; font-weight: 700; box-shadow: none; }
  .mac-stepper button + button { border-top: 1px solid var(--surface-stroke); }
</style>
