<script lang="ts">
  // 두 자리 숫자 칸(시·분 등): 직접 입력하거나 목록에서 고른다. 위아래 화살표로 하나씩 바꾼다.
  // 범위 밖이나 숫자가 아닌 값은 받지 않고 원래 값으로 돌린다. 확정은 Enter·포커스 이동·목록 선택.
  let { value, min, max, label, disabled = false, onchange }: {
    value: number; min: number; max: number; label: string; disabled?: boolean; onchange: (v: number) => void;
  } = $props();
  const id = `nf-${Math.random().toString(36).slice(2, 8)}`;
  const pad = (n: number) => String(n).padStart(2, '0');
  const opts = $derived(Array.from({ length: max - min + 1 }, (_, i) => pad(min + i)));
  let text = $state('');
  $effect(() => { text = pad(value); });

  function commit() {
    const n = Number(text.trim());
    if (text.trim() === '' || !Number.isInteger(n) || n < min || n > max) { text = pad(value); return; }
    text = pad(n);
    if (n !== value) onchange(n);
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'Enter') { commit(); (e.currentTarget as HTMLInputElement).blur(); return; }
    if (e.key !== 'ArrowUp' && e.key !== 'ArrowDown') return;
    e.preventDefault();
    const span = max - min + 1;
    const cur = Number.isInteger(Number(text)) ? Number(text) : value;
    const n = min + ((cur - min + (e.key === 'ArrowUp' ? 1 : -1) + span) % span);
    text = pad(n);
    onchange(n);
  }
  // 숫자만 두 자리까지 받는다.
  function input(e: Event) {
    const el = e.currentTarget as HTMLInputElement;
    text = el.value.replace(/\D/g, '').slice(0, 2);
    el.value = text;
    // 목록에서 고르면 바로 확정한다(입력 이벤트로 온다).
    if (opts.includes(text) && (e as InputEvent).inputType !== 'insertText') commit();
  }
</script>

<input class="num" type="text" inputmode="numeric" maxlength="2" list={id} aria-label={label} {disabled}
  value={text} oninput={input} onkeydown={key} onblur={commit} onfocus={(e) => (e.currentTarget as HTMLInputElement).select()} />
<datalist {id}>{#each opts as o (o)}<option value={o}></option>{/each}</datalist>

<style>
  .num { width: 56px; text-align: center; font-variant-numeric: tabular-nums; flex: none; }
</style>
