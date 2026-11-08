import { bundles, setCopyLocale, zh } from '../../app/copy';
import { expect, it } from 'vitest';
import type { PublicationLayout } from '../../domain/publication';
import {
  MAX_CAPTION_CHARS,
  addCaption,
  captionAligns,
  captionDraft,
  captionValues,
  editCaption,
  newCaption,
  readCaption,
} from './captionDraft';

type Page = PublicationLayout['pages'][number];
type StoredText = Page['texts'][number];

const page = (over: Partial<Page> = {}) => ({
  id: 'p1', pageNumber: 3, side: 'right', widthMm: 190, heightMm: 257,
  marginMm: 15, bleedMm: 3, layoutJson: null, frames: [], texts: [], ...over,
} as Page);
const stored = (over: Partial<StoredText> = {}) => ({
  id: 't1', xMm: 15, yMm: 200, widthMm: 160, heightMm: 18,
  contentJson: JSON.stringify({ text: '十胜，雪原尽头' }),
  styleJson: JSON.stringify({ sizePt: 12, align: 'center' }), ...over,
} as StoredText);
const values = (over: Partial<Parameters<typeof captionDraft>[0]> = {}) => ({
  text: '北海道，第一场雪', xMm: '15', yMm: '200', widthMm: '160', heightMm: '18',
  sizePt: '10', align: 'left' as const, ...over,
});

it('读回已存的图注：内容与样式各归各，坏记录只留空而不编一段话', () => {
  expect(readCaption(stored())).toEqual({ text: '十胜，雪原尽头', sizePt: 12, align: 'center' });
  // 内容 JSON 坏掉时不猜：显示空，让用户重新填，比显示一段机器拼的话诚实。
  expect(readCaption(stored({ contentJson: '{not json' }))).toEqual({ text: '', sizePt: 12, align: 'center' });
  expect(readCaption(stored({ contentJson: JSON.stringify({ text: 42 }) })).text).toBe('');
  expect(readCaption(stored({ contentJson: '{}' })).text).toBe('');
  expect(readCaption(stored({ styleJson: null }))).toEqual({ text: '十胜，雪原尽头', sizePt: 10, align: 'left' });
  // 越界的字号与陌生的对齐方式不能带进编辑表单，否则保存时又要报"数值无效"。
  expect(readCaption(stored({ styleJson: JSON.stringify({ sizePt: 999, align: 'justify' }) })))
    .toEqual({ text: '十胜，雪原尽头', sizePt: 10, align: 'left' });
  expect(captionValues(stored())).toEqual({
    text: '十胜，雪原尽头', xMm: '15', yMm: '200', widthMm: '160', heightMm: '18',
    sizePt: '12', align: 'center',
  });
});

it('新增图注落在下边距之上，不必用户先猜一组坐标', () => {
  const draft = newCaption(page());
  expect(draft).toEqual({
    text: '', xMm: '15', yMm: '224', widthMm: '160', heightMm: '18', sizePt: '10', align: 'left',
  });
  // 方形小页也不越界：横条取 18 mm，位置仍落在本页下边距之上。
  const small = newCaption(page({ widthMm: 120, heightMm: 120, marginMm: 15 }));
  const parsed = captionDraft({ ...small, text: '占位图注' })!;
  expect(parsed).toMatchObject({ xMm: 15, widthMm: 90, heightMm: 18, yMm: 87 });
  expect(parsed.yMm + parsed.heightMm).toBeLessThanOrEqual(120 - 15);
  // 新图注没写字之前不算可保存：草稿为空。
  expect(captionDraft(draft)).toBeUndefined();
});

it('只有字面和数值都说得过去才生成命令数据，空串与越界一律不出门', () => {
  expect(captionDraft(values())).toEqual({
    text: '北海道，第一场雪', xMm: 15, yMm: 200, widthMm: 160, heightMm: 18, sizePt: 10, align: 'left',
  });
  expect(captionDraft(values({ text: '  带空格的图注  ' }))!.text).toBe('带空格的图注');
  expect(captionDraft(values({ text: '' }))).toBeUndefined();
  expect(captionDraft(values({ text: '   ' }))).toBeUndefined();
  expect(captionDraft(values({ text: '雪'.repeat(MAX_CAPTION_CHARS) }))).toBeDefined();
  expect(captionDraft(values({ text: '雪'.repeat(MAX_CAPTION_CHARS + 1) }))).toBeUndefined();
  // 清空一个坐标不算 0，避免把图注甩到页面之外。
  expect(captionDraft(values({ xMm: '' }))).toBeUndefined();
  expect(captionDraft(values({ widthMm: '0' }))).toBeUndefined();
  expect(captionDraft(values({ heightMm: '-2' }))).toBeUndefined();
  expect(captionDraft(values({ sizePt: '5' }))).toBeUndefined();
  expect(captionDraft(values({ sizePt: '73' }))).toBeUndefined();
  expect(captionDraft(values({ sizePt: '72' }))!.sizePt).toBe(72);
  expect(captionDraft(values({ align: 'right' }))!.align).toBe('right');
});

it('新增与修改的命令数据只差页面归属，其余字段一致', () => {
  const draft = captionDraft(values())!;
  expect(addCaption('t9', 'p9', draft)).toEqual({ textId: 't9', pageId: 'p9', ...draft });
  expect(editCaption('t9', draft)).toEqual({ textId: 't9', ...draft });
  expect(captionAligns).toEqual(['left', 'center', 'right']);
});

it('四套语言都补齐了图注文案，回落中文不算翻译完成', () => {
  const keys = ['publicationCaption', 'publicationCaptionAdd', 'publicationCaptionEmpty',
    'publicationCaptionHint', 'publicationCaptionSelectPage', 'publicationCaptionText',
    'publicationCaptionSize', 'publicationCaptionAlign', 'publicationCaptionInvalid',
    'publicationCaptionSaved', 'publicationCaptionDeleted', 'publicationCaptionDelete',
    'publicationCaptionDeleteConfirm', 'publicationSelectCaption',
    ...captionAligns.map(align => `publicationCaptionAlign_${align}`),
    ...['xMm', 'yMm', 'widthMm', 'heightMm'].map(key => `publicationField_${key}`)];
  const zhCopy = bundles.zh.copy as Record<string, string>;
  for (const locale of ['zh', 'en', 'ja', 'ko'] as const) {
    const copy = bundles[locale].copy as Record<string, string>;
    for (const key of keys) expect(copy[key], `${locale}/${key}`).toBeTruthy();
  }
  for (const key of keys.filter(name => name.startsWith('publicationCaption'))) {
    setCopyLocale('en');
    expect(zh[key], key).not.toMatch(/[一-鿿]/);
    setCopyLocale('ja');
    expect(zh[key], key).not.toBe(zhCopy[key]);
    setCopyLocale('ko');
    expect(zh[key], key).not.toBe(zhCopy[key]);
  }
  setCopyLocale('zh');
});
