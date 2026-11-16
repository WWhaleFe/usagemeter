// 설정 탭 목록. macOnly = Windows에서 숨김(Windows는 화면 테두리만: 구분선·노치 없음).
// glyph = Windows 탐색 창 아이콘(Segoe Fluent Icons 코드).
import type { Component } from 'svelte';
import Login from './tabs/Login.svelte';
import Presets from './tabs/Presets.svelte';
import Display from './tabs/Display.svelte';
import Interval from './tabs/Interval.svelte';
import Line from './tabs/Line.svelte';
import SubItems from './tabs/SubItems.svelte';
import Partition from './tabs/Partition.svelte';
import Layout from './tabs/Layout.svelte';
import Corner from './tabs/Corner.svelte';
import Notch from './tabs/Notch.svelte';
import Support from './tabs/Support.svelte';

export interface TabDef { id: string; component: Component; glyph: string; macOnly?: boolean }

export const TABS: TabDef[] = [
  { id: 'login', component: Login, glyph: '' },
  { id: 'presets', component: Presets, glyph: '' },
  { id: 'display', component: Display, glyph: '' },
  { id: 'interval', component: Interval, glyph: '' },
  { id: 'line', component: Line, glyph: '' },
  { id: 'subItems', component: SubItems, glyph: '' },
  { id: 'partition', component: Partition, glyph: '', macOnly: true },
  { id: 'layout', component: Layout, glyph: '' },
  { id: 'corner', component: Corner, glyph: '' },
  { id: 'notch', component: Notch, glyph: '', macOnly: true },
  { id: 'support', component: Support, glyph: '' },
];
