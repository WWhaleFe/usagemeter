<script lang="ts">
  // 선 탭(Swift SettingsView.swift:462-510): 두께·투명도·트랙(편집 중인 화면), 시작 시 숨김(전역),
  // 페이드(화면), 겹침 분할(화면), AI 색(전역, 프리셋 7색).
  import { ui, t, set, toHex, fromHex } from '../../lib/api.svelte';
  import Section from '../../components/Section.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import Slider from '../../components/Slider.svelte';
  const s = $derived(ui.s!);
  const b = $derived(s.border);
  const colors = $derived((s.main.providerColors ?? {}) as Record<string, number[]>);
  function setColor(id: string, hex: string) {
    set('main', { providerColors: { ...colors, [id]: fromHex(hex) } });
  }
  // 같은 색으로 볼 범위: 채널마다 0.02 이내(Swift :500).
  const near = (a: string, b: string) => {
    const x = fromHex(a), y = fromHex(b);
    return x.slice(0, 3).every((v, i) => Math.abs(v - y[i]) < 0.02);
  };
</script>

<Section title={t('sec.line')}>
  <Slider label={t('line.thickness')} value={b.thickness ?? 2} min={1} max={15} step={0.5} unit="pt" onchange={(v) => set('border', { thickness: v })} />
  <Slider label={t('line.opacity')} value={Math.round((1 - (b.lineOpacity ?? 1)) * 100)} min={0} max={100} unit="%" onchange={(v) => set('border', { lineOpacity: 1 - v / 100 })} />
  <Toggle label={t('line.track')} checked={b.showTrack ?? true} onchange={(v) => set('border', { showTrack: v })} />
  <Toggle label={t('line.hideOnLaunch')} checked={!!s.main.hideOverlayOnLaunch} onchange={(v) => set('main', { hideOverlayOnLaunch: v })} />
  <Toggle label={t('line.fade')} checked={b.fadeEnabled ?? true} onchange={(v) => set('border', { fadeEnabled: v })} />
  <Slider label={t('line.fadeLen')} value={Math.round((b.fadeFraction ?? 0.012) * 100)} min={0} max={20} unit="%" disabled={!b.fadeEnabled} onchange={(v) => set('border', { fadeFraction: v / 100 })} />
</Section>

<Section title={t('sec.overlap')}>
  <Toggle label={t('lay.splitOverlap')} checked={!!b.splitOverlapLines} onchange={(v) => set('border', { splitOverlapLines: v })} />
  <span class="caption">{b.splitOverlapLines ? t('overlap.splitDesc') : t('overlap.layerDesc')}</span>
</Section>

<Section title={t('sec.aiColor')}>
  {#each s.providers as p (p.id)}
    <div class="row">
      <span class="name">{p.name}</span>
      <input type="color" value={p.color} onchange={(e) => setColor(p.id, (e.currentTarget as HTMLInputElement).value)} />
      <span class="swatches">
        {#each s.brandColors as c (c.name)}
          <button class="sw" class:on={near(p.color, c.hex)} style:background={c.hex} title={c.name} aria-label={c.name} onclick={() => setColor(p.id, c.hex)}></button>
        {/each}
      </span>
    </div>
  {/each}
</Section>

<style>
  .name { width: 70px; font-weight: 600; }
  input[type="color"] { width: 36px; height: 24px; padding: 0; border: 1px solid var(--control-stroke); border-radius: var(--r-control); background: none; }
  .swatches { display: inline-flex; gap: 6px; margin-left: 6px; }
  .sw { width: 18px; height: 18px; min-height: 0; padding: 0; border-radius: 50%; border: 1px solid var(--control-stroke); }
  .sw.on { outline: 2px solid var(--accent); outline-offset: 1px; }
</style>
