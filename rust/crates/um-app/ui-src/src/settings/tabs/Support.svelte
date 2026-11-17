<script lang="ts">
  // 개발자에게 탭(Swift SupportView.swift): 리포트 메일, 후원(GitHub Sponsors, 카카오페이 QR).
  import { ui, t, act } from '../../lib/api.svelte';
  import Section from '../../components/Section.svelte';
  import Icon from '../../analytics/Icon.svelte';
  const EMAIL = 'feedback.wwhale@gmail.com';
  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let qr = $state<{ svg: string; url: string } | null>(null);
  $effect(() => {
    act('kakaoQrSvg').then((r) => (qr = r as { svg: string; url: string }));
    return () => clearTimeout(timer);
  });
  function copy() {
    act('copyEmail');
    copied = true;
    clearTimeout(timer);
    timer = setTimeout(() => (copied = false), 1600);
  }
</script>

<div class="center">
  <Section title={t('report.title')}>
    <div class="col">
      <p class="caption desc">{t('report.desc')}</p>
      <code class="mail">{EMAIL}</code>
      <div class="row btns">
        <button class="primary" onclick={() => act('openMail')}>{#if ui.s?.platform === 'mac'}<Icon name="send" />{/if}{t('report.openMail')}</button>
        <button onclick={() => copy()} aria-live="polite">{#if ui.s?.platform === 'mac'}<Icon name={copied ? 'check' : 'copy'} />{/if}{copied ? t('report.copied') : t('report.copyEmail')}</button>
      </div>
    </div>
  </Section>

  <Section title={t('support.moreTitle')}>
    <div class="col">
      <p class="caption desc">{t('support.desc')}</p>
      <button class="heart" onclick={() => act('openUrl', { url: 'https://github.com/sponsors/WWhaleFe' })}>&#9829; {t('support.sponsors')}</button>
      <strong>{t('support.kakaoTitle')}</strong>
      <p class="caption desc">{t('support.kakaoScan')}</p>
      {#if qr}
        <div class="qr" role="img" aria-label={t('support.kakaoTitle')}>{@html qr.svg}</div>
        <button onclick={() => act('openUrl', { url: qr!.url })}>{#if ui.s?.platform === 'mac'}<Icon name="link" />{/if}{t('support.kakaoOpen')}</button>
      {/if}
      <p class="caption desc">{t('support.thanks')}</p>
    </div>
  </Section>
</div>

<style>
  .center { text-align: center; }
  .col { display: flex; flex-direction: column; align-items: center; gap: 8px; }
  .desc { margin: 0; }
  .btns { justify-content: center; }
  .mail { font: 600 var(--fs)/1.4 ui-monospace, Menlo, Consolas, monospace; user-select: text; padding: 6px 12px; border-radius: var(--r-control); background: color-mix(in srgb, var(--accent) 10%, transparent); border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent); }
  .heart { color: #e5398a; }
  .qr { background: #fff; padding: 8px; border-radius: 8px; border: 1px solid var(--surface-stroke); line-height: 0; }
  .qr :global(svg) { width: 130px; height: 130px; display: block; }
  :global(:root[data-skin="macos"]) .col > .desc { font-size: 12px; }
  :global(:root[data-skin="macos"]) .col > .desc:nth-last-child(2) { font-size: 11px; }
  :global(:root[data-skin="macos"]) .mail { padding: 8px 12px; font-weight: 600; }
  :global(:root[data-skin="macos"]) .heart { color: var(--fg); }
  :global(:root[data-skin="macos"]) button { display: inline-flex; gap: 8px; align-items: center; }
</style>
