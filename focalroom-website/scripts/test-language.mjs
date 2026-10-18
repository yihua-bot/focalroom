import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import vm from 'node:vm';
import test from 'node:test';

const root = path.resolve(import.meta.dirname, '../dist');
const source = await fs.readFile(path.join(root, 'i18n.js'), 'utf8');
const catalog = await fs.readFile(path.join(root, 'locales.js'), 'utf8');

// 仅测试语言解析和存储失败边界，不操作真实浏览器或用户数据。
function load({ query = '', remembered, languages = ['en-US'], storageBlocked = false, historyBlocked = false } = {}) {
  const stored = new Map(remembered ? [['focalroom.locale', remembered]] : []);
  const events = [];
  const urls = [];
  const document = { documentElement: { lang: '' }, createTreeWalker: () => ({ nextNode: () => false }), querySelectorAll: () => [] };
  const window = { dispatchEvent: event => events.push(event), history: { replaceState: (_, __, url) => { if (historyBlocked) throw new Error('blocked'); urls.push(url); } } };
  const context = vm.createContext({
    window, document, URL, location: { href: 'https://example.test/index.html' + query },
    navigator: { languages }, NodeFilter: { SHOW_TEXT: 4 },
    CustomEvent: class { constructor(type, options) { this.type = type; this.detail = options.detail; } },
    localStorage: { getItem(key) { if (storageBlocked) throw new Error('blocked'); return stored.get(key) ?? null; }, setItem(key, value) { if (storageBlocked) throw new Error('blocked'); stored.set(key, value); } },
  });
  vm.runInContext(catalog, context);
  vm.runInContext(source, context);
  return { api: window.FocalI18n, document, stored, events, urls };
}
test('explicit URL language overrides saved and browser language', () => {
  const { api, document } = load({ query: '?lang=JA', remembered: 'ko', languages: ['en-US'] });
  assert.equal(api.language, 'ja');
  assert.equal(document.documentElement.lang, 'ja');
});
test('saved language overrides browser, invalid values fall back to supported browser language', () => {
  assert.equal(load({ remembered: 'ko', languages: ['en'] }).api.language, 'ko');
  assert.equal(load({ query: '?lang=xx', remembered: 'invalid', languages: ['de', 'ko-KR'] }).api.language, 'ko');
  assert.equal(load({ languages: ['de'] }).api.language, 'zh');
});
test('switch persists only a supported language, retaining URL hash', () => {
  const state = load({ query: '?campaign=photo#playground' });
  state.api.setLanguage('ko');
  assert.equal(state.stored.get('focalroom.locale'), 'ko');
  assert.equal(state.urls.at(-1), '/index.html?campaign=photo&lang=ko#playground');
  state.api.setLanguage('invalid');
  assert.equal(state.api.language, 'ko');
  assert.equal(state.events.at(-1).detail.language, 'ko');
});
test('blocked storage and history do not prevent language switching', () => {
  const state = load({ storageBlocked: true, historyBlocked: true });
  state.api.setLanguage('ja');
  assert.equal(state.document.documentElement.lang, 'ja');
  assert.equal(state.api.t('{count} 张', { count: 3 }), '3枚');
});
test('dynamic notices translate photo names and numbers together', () => {
  const { api } = load({ query: '?lang=en' });
  assert.equal(api.t('已保留「{name}」。当前序列 {count} 张。', { name: '海岸上的独坐者', count: 1 }), 'Kept “A figure by the sea”. Your sequence has 1 photo.');
  api.setLanguage('ko');
  assert.equal(api.t('前移：{name}', { name: '风里的小屋' }), '앞으로 이동: 바람 속 작은 집');
});
