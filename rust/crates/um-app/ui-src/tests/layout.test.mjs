// 실행: node --test tests/layout.test.mjs (Node 22.18 이상). 네 방향의 실제 Svelte 출력을 함께 검사한다.
import { test, after } from 'node:test';
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir, mkdtemp, rm } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';
import { diagramLines, ORDER, segmentKey, capVector } from '../src/components/BDiagram.ts';

const root = fileURLToPath(new URL('../', import.meta.url));
const temp = await mkdtemp(path.join(root, '.layout-test-'));
after(() => rm(temp, { recursive: true, force: true }));
const table = Object.assign({}, ...await Promise.all(['strings', 'extra'].map(async (name) =>
  JSON.parse(await readFile(path.join(root, '../../um-core/src/i18n', name + '.json'), 'utf8')))));
await writeFile(path.join(temp, 'api.mjs'), `export const ui = { s: null }; export const t = k => ui.s.strings[k] ?? k; export const set = () => {}; export const act = () => {};`);
const api = await import(pathToFileURL(path.join(temp, 'api.mjs')));
const styles = [];
const built = new Map();
async function component(file) {
  if (built.has(file)) return built.get(file);
  const output = path.join(temp, path.basename(file) + '.mjs');
  built.set(file, output);
  const result = compile(await readFile(file, 'utf8'), { filename: file, generate: 'server' });
  let code = result.js.code;
  for (const match of [...code.matchAll(/from '([^']+)'/g)]) {
    const spec = match[1];
    if (!spec.startsWith('.')) continue;
    let target;
    if (spec.endsWith('api.svelte')) target = path.join(temp, 'api.mjs');
    else if (spec.endsWith('.svelte')) target = await component(path.resolve(path.dirname(file), spec));
    else target = path.resolve(path.dirname(file), spec + '.ts');
    code = code.replaceAll(`'${spec}'`, `'${pathToFileURL(target)}'`);
  }
  if (result.css) styles.push(result.css.code);
  await writeFile(output, code);
  return output;
}
const Layout = (await import(pathToFileURL(await component(path.join(root, 'src/components/BAiLayout.svelte'))))).default;
const Partition = (await import(pathToFileURL(await component(path.join(root, 'src/settings/tabs/Partition.svelte'))))).default;

function fixture(menu = true, dock = true) {
  const node = (right, level) => ({ right, level });
  const ends = [[node(false, 0), node(true, 0)], [node(false, 1), node(true, 1)],
    [node(false, 2), node(true, 2)], [node(false, 3), node(true, 3)],
    ...[false, true].flatMap((right) => [[node(right, 0), node(right, 1)],
      [node(right, menu ? 1 : 0), node(right, dock ? 2 : 3)], [node(right, 2), node(right, 3)]])];
  return Object.fromEntries(ORDER.map((seg, i) => [seg, { ends: ends[i], on: true, toggleAllowed: true,
    available: !(['hMenu', 'lMenuBar', 'rMenuBar'].includes(seg) && !menu) && !(['hDock', 'lDock', 'rDock'].includes(seg) && !dock) }]));
}
function state(side, platform = 'mac', lang = 0, dock = true) {
  api.ui.s = { dockSide: side, platform, border: { menuLineEnabled: platform === 'mac', dockLineEnabled: dock },
    strings: Object.fromEntries(Object.entries(table).map(([key, value]) => [key,
      platform === 'win' && table['win.' + key] ? table['win.' + key][lang] : value[lang]])) };
}
function provider(menu = true, dock = true) {
  return { id: 'claude', name: 'Claude', color: '#dc804f', segOptions: fixture(menu, dock),
    layout: { segments: ORDER, hCaps: { 'hDock.L': 'up', 'hDock.R': 'down' } },
    anchorSideKind: 'hidden', chain: true,
    shapePresets: Object.fromEntries(['all', 'menuBar', 'main', 'dock'].map((key) => [key, { allowed: true, zones: [key], segments: [] }])) };
}
const previews = [];
for (const side of ['bottom', 'left', 'right', 'top']) {
  test(`${side}: Svelte 그림·캡 행·플랫폼 용어`, () => {
    const platform = side === 'top' ? 'win' : 'mac';
    state(side, platform);
    const html = render(Layout, { props: { p: provider(platform === 'mac') } }).body;
    assert(!html.includes('NaN'));
    assert(html.includes('role="button"'));
    const key = segmentKey('hDock', side, platform === 'mac');
    assert(html.includes(api.t(key)));
    if (side === 'left' || side === 'right') {
      assert(html.includes('세로선 캡'));
      const groups = html.split('<div class="caps');
      assert(!groups[1].includes('Dock선'));
      assert(groups[2].includes('Dock선 │'));
      const lines = diagramLines(fixture(), true, true, side);
      assert.equal(lines.find((l) => l.seg === 'hTop').points[0][1], 12);
      assert.equal(lines.find((l) => l.seg === 'hMenu').points[0][1], 48);
      assert.equal(lines.find((l) => l.seg === 'hDock').horiz, false);
      const part = render(Partition).body;
      assert(part.includes('Dock 경계 폭'));
    } else {
      assert(!html.includes('세로선 캡'));
    }
    if (side === 'top') assert(html.includes('작업 표시줄 감싸개'));
    previews.push(`<h2>${side} (${platform})</h2>${html}`);
  });
}
test('하단 Dock의 기존 좌표 유지', () => {
  const lines = diagramLines(fixture(), true, true, 'bottom');
  assert.deepEqual(lines.find((l) => l.seg === 'hTop').points, [[26, 12], [183, 12]]);
  assert.deepEqual(lines.find((l) => l.seg === 'hDock').points, [[26, 12 + 109 * 2 / 3], [183, 12 + 109 * 2 / 3]]);
  assert.deepEqual(lines.find((l) => l.seg === 'lMenuBar').points, [[14, 21], [14, 12 + 109 / 3 - 9]]);
  state('bottom');
  assert(render(Partition).body.includes('Dock 경계 높이'));
});
test('없음·구분선 해제 상태에 유한 좌표와 비활성 프리셋', () => {
  for (const menu of [false, true]) for (const dock of [false, true]) for (const side of [null, 'bottom', 'left', 'right', 'top']) {
    const lines = diagramLines(fixture(menu, dock), menu, dock && side !== null, side);
    assert(lines.every((line) => line.points.flat().every(Number.isFinite)));
  }
  state(null, 'win');
  const html = render(Layout, { props: { p: provider(false) } }).body;
  assert.match(html, /<button[^>]*disabled[^>]*>작업 표시줄 감싸개/);
  state('left', 'mac', 0, false);
  assert(!render(Layout, { props: { p: provider(true, false) } }).body.includes('세로선 캡'));
});
test('세 언어의 방향별 명칭과 캡 방향', () => {
  for (const side of ['bottom', 'top', 'left', 'right']) for (const menu of [false, true]) for (const seg of ORDER) {
    assert(table[segmentKey(seg, side, menu)], `${side} ${seg}`);
  }
  assert.deepEqual(capVector('hDock', true, 'left', true), [1, 0]);
  assert.deepEqual(capVector('hDock', true, 'right', true), [-1, 0]);
  assert.deepEqual(capVector('hMenu', true, 'left', true), [0, -1]);
  assert.deepEqual(capVector('hDock', true, 'top', false), [0, 1]);
  for (let lang = 0; lang < 3; lang++) {
    state('left', 'win', lang);
    const html = render(Layout, { props: { p: provider(false) } }).body;
    assert(html.includes(table['win.seg.dockVertical'][lang]));
    assert(html.includes(table['win.preset.dockWrap'][lang]));
  }
});
after(async () => {
  const out = path.resolve(root, '../../../target/layout-preview.html');
  await mkdir(path.dirname(out), { recursive: true });
  await writeFile(out, `<!doctype html><meta charset="utf-8"><style>:root{--surface:#f8f8f8;--surface-stroke:#ccc;--fg:#222;--fg2:#555;--accent:#d80;--r-card:8px;--fs-small:11px;--fs-title:14px}body{font:13px sans-serif;max-width:760px;margin:24px auto}${styles.join('\n')}</style>${previews.join('\n')}`);
});
