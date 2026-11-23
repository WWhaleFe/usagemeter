<script lang="ts">
  // 슬라이더 + 숫자 칸 + 화살표(Swift LabeledSlider, SettingsView.swift:1162-1201).
  // 레이블 위, 눈금 슬라이더와 숫자 칸 및 세로 스테퍼 아래로 배치한다.
  import InfoTip from './InfoTip.svelte';
  let { label, value, min, max, step = 1, unit = '', disabled = false, help = '', onchange }:
    { label: string; value: number; min: number; max: number; step?: number; unit?: string; disabled?: boolean; help?: string; onchange: (v: number) => void } = $props();
  const sliderId = $props.id();
  const clampValue = (v: number) => Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : value;
  function arrow(e: KeyboardEvent) {
    if (disabled || !['ArrowUp', 'ArrowDown', 'ArrowRight', 'ArrowLeft'].includes(e.key)) return;
    e.preventDefault();
    onchange(clampValue(value + (e.key === 'ArrowUp' || e.key === 'ArrowRight' ? step : -step)));
  }
</script>

<div class="mac-slider" class:disabled role="group" aria-label={label}>
  <label class="label" for={sliderId}>{label}{#if help}<InfoTip text={help} />{/if}</label>
  <div class="controls">
    <input id={sliderId} class="track" type="range" onkeydown={arrow} {min} {max} {step} {value} {disabled} style:--ticks={Math.round((max - min) / step)} aria-label={label} oninput={(e) => onchange(+e.currentTarget.value)} />
    <input class="num" type="number" onkeydown={arrow} {min} {max} step="any" value={value.toFixed(1).replace(/\.0$/, '')} {disabled} aria-label={label} onchange={(e) => onchange(clampValue(+e.currentTarget.value))} />
    <span class="mac-stepper">
      <button {disabled} aria-label={label + ' +'} onclick={() => onchange(clampValue(value + step))}><svg viewBox="0 0 8 5" aria-hidden="true"><path d="m1 4 3-3 3 3" /></svg></button>
      <button {disabled} aria-label={label + ' −'} onclick={() => onchange(clampValue(value - step))}><svg class="down" viewBox="0 0 8 5" aria-hidden="true"><path d="m1 4 3-3 3 3" /></svg></button>
    </span>
  </div>
</div>

<style>
  .label { width: 150px; flex: none; }
  .num { width: 64px; text-align: right; }
  input[type="range"] { accent-color: var(--accent); }
  .mac-slider { display: flex; flex-direction: column; gap: 3px; padding: 4px; min-width: 0; }
  .mac-slider:focus-within { outline: 2px solid var(--accent); border-radius: 6px; }
  .mac-slider input:focus-visible { outline: none; }
  .mac-slider.disabled { opacity: .4; }
  .mac-slider .label { width: auto; font-size: 12px; font-weight: 500; line-height: 15px; }
  .controls { display: flex; align-items: center; gap: 8px; height: 20px; }
  .mac-slider .num { width: 48px; flex: none; appearance: textfield; font-variant-numeric: tabular-nums; }
  .mac-slider .num::-webkit-inner-spin-button { appearance: none; }
  .track { appearance: none; flex: 1; min-width: 0; margin: 0; height: 20px; background: transparent; }
  .track::-webkit-slider-runnable-track { height: 8px; background: repeating-linear-gradient(to right, var(--slider-tick) 0 1px, transparent 1px max(3px, calc(100% / var(--ticks)))), linear-gradient(var(--slider-line), var(--slider-line)) center / 100% 3px no-repeat; }
  .track::-webkit-slider-thumb { appearance: none; width: 8px; height: 20px; border-radius: 4px; background: var(--slider-thumb); margin-top: -6px; }
  .mac-stepper { display: flex; flex-direction: column; flex: none; width: 13px; border-radius: 4px; overflow: hidden; background: var(--control); }
  .mac-stepper button { border: none; border-radius: 0; padding: 0; width: 13px; height: 10px; min-height: 0; line-height: 10px; font-size: 12px; font-weight: 700; box-shadow: none; }
  .mac-stepper button { display: flex; align-items: center; justify-content: center; }
  .mac-stepper svg { display: block; width: 8px; height: 5px; fill: none; stroke: currentColor; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; }
  .mac-stepper .down { transform: rotate(180deg); }
  .mac-stepper button + button { box-shadow: inset 0 1px var(--surface-stroke); }
</style>
