// 두 플랫폼이 같은 탭과 순서를 사용한다.
import type { Component } from 'svelte';
import Login from './tabs/Login.svelte';
import Presets from './tabs/Presets.svelte';
import Display from './tabs/Display.svelte';
import MenuBar from './tabs/MenuBar.svelte';
import Interval from './tabs/Interval.svelte';
import Line from './tabs/Line.svelte';
import SubItems from './tabs/SubItems.svelte';
import Partition from './tabs/Partition.svelte';
import Layout from './tabs/Layout.svelte';
import Corner from './tabs/Corner.svelte';
import Notch from './tabs/Notch.svelte';
import Support from './tabs/Support.svelte';

export interface TabDef { id: string; component: Component }

export const TABS: TabDef[] = [
  { id: 'login', component: Login },
  { id: 'presets', component: Presets },
  { id: 'display', component: Display },
  { id: 'menuBar', component: MenuBar },
  { id: 'interval', component: Interval },
  { id: 'line', component: Line },
  { id: 'subItems', component: SubItems },
  { id: 'partition', component: Partition },
  { id: 'layout', component: Layout },
  { id: 'corner', component: Corner },
  { id: 'notch', component: Notch },
  { id: 'support', component: Support },
];

// 직접 해시로 접근해도 스토어판에서는 개발자 탭을, Windows에서는 노치 탭(노치 없는 OS)을 선택할 수 없다.
export const tabsForStore = (store: boolean, win = false) =>
  TABS.filter((tab) => !(store && tab.id === 'support') && !(win && tab.id === 'notch'));
