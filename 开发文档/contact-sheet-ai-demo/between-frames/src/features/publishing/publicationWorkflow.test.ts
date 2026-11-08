import { bundles, setCopyLocale, zh } from '../../app/copy';
import { expect, it } from 'vitest';
import {
  isPublicationState,
  nextStateActions,
  publicationActionLabel,
  publicationStateLabel,
  publicationStates,
} from './publicationWorkflow';

it('流转表和服务端那一条一致：只给相邻的格子按，跳级和自环都不出现', () => {
  expect([...publicationStates]).toEqual(['draft', 'in_review', 'finalized', 'delivered']);
  expect(nextStateActions('draft')).toEqual(['in_review']);
  expect(nextStateActions('in_review')).toEqual(['draft', 'finalized']);
  expect(nextStateActions('finalized')).toEqual(['in_review', 'delivered']);
  // 交付是本条路的尽头：这里少给一个按钮，用户就不会收到一句后端拒收的错误。
  expect(nextStateActions('delivered')).toEqual([]);
  for (const state of publicationStates) expect(nextStateActions(state), state).not.toContain(state);
  expect(nextStateActions('draft')).not.toContain('finalized');
  expect(nextStateActions('draft')).not.toContain('delivered');
});

it('库里读到没见过的状态按原样显示，并且不放开任何下一步', () => {
  expect(isPublicationState('archived')).toBe(false);
  expect(isPublicationState('draft')).toBe(true);
  expect(publicationStateLabel('archived')).toBe('archived');
  expect(nextStateActions('archived')).toEqual([]);
  expect(publicationStateLabel('draft')).toBe('草稿');
  expect(publicationActionLabel('in_review')).toBe('送审');
});

it('四套语言都补齐了审校文案，回落中文不算翻译完成', () => {
  const keys = ['publicationState', 'publicationStateChanged', 'publicationStateHint', 'publicationStateLocked'];
  for (const state of publicationStates) keys.push(`publicationState_${state}`, `publicationStateAction_${state}`);
  const zhCopy = bundles.zh.copy as Record<string, string>;
  for (const locale of ['zh', 'en', 'ja', 'ko'] as const) {
    const copy = bundles[locale].copy as Record<string, string>;
    for (const key of keys) expect(copy[key], `${locale}/${key}`).toBeTruthy();
  }
  for (const key of keys) {
    setCopyLocale('en');
    expect(zh[key], key).not.toMatch(/[一-鿿]/);
    setCopyLocale('ja');
    expect(zh[key], key).not.toBe(zhCopy[key]);
    setCopyLocale('ko');
    expect(zh[key], key).not.toBe(zhCopy[key]);
  }
  setCopyLocale('ja');
  for (const key of keys) expect(zh[key], `ja/${key}`).not.toMatch(/[가-힣]/);
  setCopyLocale('zh');
});

it('四套语言都补齐了导出文案，占位符一个都不能少', () => {
  const keys = [
    'publicationExport', 'publicationExportHint', 'publicationExportRun', 'publicationExportSaved',
    'publicationExportChecksum', 'publicationExportFailed', 'publicationExportCancelled',
  ];
  const zhCopy = bundles.zh.copy as Record<string, string>;
  for (const locale of ['zh', 'en', 'ja', 'ko'] as const) {
    const copy = bundles[locale].copy as Record<string, string>;
    for (const key of keys) expect(copy[key], `${locale}/${key}`).toBeTruthy();
    // 路径与条数只靠这两个占位符上屏：翻译时丢了它，回执就只剩"已导出"三个字，用户无从核对。
    for (const token of ['{items}', '{path}']) expect(copy.publicationExportSaved, `${locale}/saved`).toContain(token);
  }
  for (const key of keys) {
    setCopyLocale('en');
    expect(zh[key], key).not.toMatch(/[一-鿿]/);
    setCopyLocale('ja');
    expect(zh[key], key).not.toBe(zhCopy[key]);
    setCopyLocale('ko');
    expect(zh[key], key).not.toBe(zhCopy[key]);
  }
  setCopyLocale('ja');
  for (const key of keys) expect(zh[key], `ja/${key}`).not.toMatch(/[가-힣]/);
  setCopyLocale('zh');
});
