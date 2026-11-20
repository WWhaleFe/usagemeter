<script lang="ts">
  // 차트 카드(Swift `card`, AnalyticsView.swift:613-624: 아이콘+제목, 설명 2줄, 내용).
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  // height = null이면 내용 높이를 따른다(목록 카드).
  let { title, desc, icon, height = 250, children }:
    { title: string; desc: string; icon: string; height?: number | null; children: Snippet } = $props();
</script>

<section class="card" style:height={height == null ? null : `${height}px`}>
  <h2><Icon name={icon} size={15} />{title}</h2>
  <p class="desc">{desc}</p>
  {@render children()}
</section>

<style>
  .card { display: flex; flex-direction: column; gap: 8px; padding: 16px; min-width: 0; background: var(--card-bg); border: var(--card-border); border-radius: var(--card-r); }
  h2 { display: flex; align-items: center; gap: 6px; margin: 0; font-size: 13px; font-weight: 600; }
  .desc { margin: 0; font-size: var(--fs-small); color: var(--fg2); display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
</style>
