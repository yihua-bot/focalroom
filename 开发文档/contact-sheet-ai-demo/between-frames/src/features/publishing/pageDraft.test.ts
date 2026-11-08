import { bundles, setCopyLocale, zh } from '../../app/copy';
import { expect, it } from 'vitest';
import type { PublicationLayout } from '../../domain/publication';
import {
  MAX_BLEED_MM,
  MAX_PAGE_MM,
  MAX_WHOLE_BOOK_PAGES,
  MIN_PAGE_MM,
  boxFits,
  editPage,
  findPreset,
  pageDraft,
  pageGeometry,
  pageValues,
  paperPresets,
  presetLabel,
  wholeBookBlockers,
} from './pageDraft';
import type { PageValues } from './pageDraft';

type Page = PublicationLayout['pages'][number];
const page = (over: Partial<Page> = {}) => ({
  id: 'p1', pageNumber: 3, side: 'right', widthMm: 190, heightMm: 257,
  marginMm: 15, bleedMm: 3, layoutJson: null, frames: [], texts: [], ...over,
} as Page);
const values = (over: Partial<PageValues> = {}): PageValues => ({
  widthMm: '190', heightMm: '257', marginMm: '15', bleedMm: '3', ...over,
});

it('读回页面几何：毫米数变回可编辑的文本，不提前算成数字', () => {
  expect(pageValues(page())).toEqual({ widthMm: '190', heightMm: '257', marginMm: '15', bleedMm: '3' });
  // 小数原样留着：12.5mm 的边距不该在输入框里被四舍五入成 12。
  expect(pageValues(page({ marginMm: 12.5 }))).toMatchObject({ marginMm: '12.5' });
});

it('判定与版面校验同源：尺寸 50–2000mm、出血 0–25mm、边距不许吃掉整页', () => {
  expect(pageDraft(values())).toEqual({ widthMm: 190, heightMm: 257, marginMm: 15, bleedMm: 3 });
  expect(pageDraft(values({ widthMm: String(MIN_PAGE_MM) }))).toBeDefined();
  expect(pageDraft(values({ widthMm: String(MIN_PAGE_MM - 0.1) }))).toBeUndefined();
  expect(pageDraft(values({ heightMm: String(MAX_PAGE_MM) }))).toBeDefined();
  expect(pageDraft(values({ heightMm: String(MAX_PAGE_MM + 1) }))).toBeUndefined();
  expect(pageDraft(values({ bleedMm: String(MAX_BLEED_MM) }))).toBeDefined();
  expect(pageDraft(values({ bleedMm: String(MAX_BLEED_MM + 0.5) }))).toBeUndefined();
  expect(pageDraft(values({ bleedMm: '-1' }))).toBeUndefined();
  expect(pageDraft(values({ marginMm: '0' }))).toBeDefined();
  expect(pageDraft(values({ marginMm: '-1' }))).toBeUndefined();
  // 边距按短边算：两倍的边距吃掉短边就不成立，这条与后端 validate_layout 一致。
  expect(pageDraft(values({ widthMm: '120', marginMm: '60' }))).toBeUndefined();
  expect(pageDraft(values({ widthMm: '120', marginMm: '59.9' }))).toBeDefined();
  // 空串不算 0，非数字与无穷大也不算尺寸。
  expect(pageDraft(values({ widthMm: '' }))).toBeUndefined();
  expect(pageDraft(values({ widthMm: 'abc' }))).toBeUndefined();
  expect(pageDraft(values({ heightMm: 'Infinity' }))).toBeUndefined();
});

it('常见开本只认字面尺寸，手填的近似值不冒充标准开本', () => {
  expect(findPreset(values())).toBe('');
  expect(findPreset(values({ widthMm: '210', heightMm: '297' }))).toBe('a4');
  expect(findPreset(values({ widthMm: '148', heightMm: '210' }))).toBe('a5');
  expect(findPreset(values({ widthMm: '210', heightMm: '210' }))).toBe('square');
  expect(findPreset(values({ widthMm: '297', heightMm: '210' }))).toBe('landscape');
  expect(findPreset(values({ widthMm: ' 210 ', heightMm: '297' }))).toBe('a4');
  expect(findPreset(values({ widthMm: '210.0', heightMm: '297' }))).toBe('');
  expect(presetLabel(paperPresets[0])).toBe('A4 210×297');
  // 每个预设本身要过自己的校验，否则一点开本就是无效值。
  for (const preset of paperPresets) {
    expect(pageDraft(values({ widthMm: String(preset.widthMm), heightMm: String(preset.heightMm) })))
      .toMatchObject({ widthMm: preset.widthMm, heightMm: preset.heightMm });
  }
});

it('命令数据带页面归属，四个毫米数原样交给后端', () => {
  expect(editPage('p7', { widthMm: 148, heightMm: 210, marginMm: 12, bleedMm: 3 }))
    .toEqual({ pageId: 'p7', widthMm: 148, heightMm: 210, marginMm: 12, bleedMm: 3 });
});

it('四套语言都补齐了纸张文案，回落中文不算翻译完成', () => {
  const keys = ['publicationPaper', 'publicationPaperPreset', 'publicationPaperCustom',
    'publicationPaperInvalid', 'publicationPaperHint', 'publicationPaperSaved',
    'publicationPaperSave', 'publicationPaperDiscard',
    'publicationPaperApplyAll', 'publicationPaperApplyAllSaved',
    'publicationPaperApplyAllBlocked', 'publicationPaperApplyAllTooMany', 'publicationPaperApplyAllJoin',
    'publicationField_pageWidthMm', 'publicationField_pageHeightMm',
    'publicationField_marginMm', 'publicationField_bleedMm'];
  const zhCopy = bundles.zh.copy as Record<string, string>;
  for (const locale of ['zh', 'en', 'ja', 'ko'] as const) {
    const copy = bundles[locale].copy as Record<string, string>;
    for (const key of keys) expect(copy[key], `${locale}/${key}`).toBeTruthy();
  }
  // 标点会正经地重复（中日韩都用顿号），它不算"没翻译"，只查有没有填。
  for (const key of keys.filter(name => name.startsWith('publicationPaper') && !name.endsWith('Join'))) {
    setCopyLocale('en');
    expect(zh[key], key).not.toMatch(/[一-鿿]/);
    setCopyLocale('ja');
    expect(zh[key], key).not.toBe(zhCopy[key]);
    setCopyLocale('ko');
    expect(zh[key], key).not.toBe(zhCopy[key]);
  }
  // 日文串里混进谚文是写错了，不是翻译——中日共用汉字，这类错眼看不出来。
  setCopyLocale('ja');
  for (const key of keys.filter(name => !name.endsWith('Join'))) expect(zh[key], `ja/${key}`).not.toMatch(/[가-힣]/);
  setCopyLocale('zh');
});

it('整本统一开本的参照数取自页面本身，不经过输入框那层字符串', () => {
  expect(pageGeometry(page())).toEqual({ widthMm: 190, heightMm: 257, marginMm: 15, bleedMm: 3 });
  // 一寸命令能带的操作数就是契约里的上限，超了要提示逐页改，不能悄悄只改前 100 页。
  expect(MAX_WHOLE_BOOK_PAGES).toBe(100);
});

it('内容落点与服务端同一条边界：出血内算放下，出一寸就拒', () => {
  const a5 = { widthMm: 148, heightMm: 210, marginMm: 15, bleedMm: 3 };
  // 正好压着出血外沿：左上一个负 3，右下一个 151/213，服务端 rectangle() 也判通过。
  expect(boxFits({ xMm: -3, yMm: -3, widthMm: 154, heightMm: 216 }, a5)).toBe(true);
  expect(boxFits({ xMm: -4, yMm: -3, widthMm: 154, heightMm: 216 }, a5)).toBe(false);
  expect(boxFits({ xMm: -3, yMm: -3, widthMm: 155, heightMm: 216 }, a5)).toBe(false);
  expect(boxFits({ xMm: -3, yMm: 4, widthMm: 154, heightMm: 216 }, a5)).toBe(false);
  expect(boxFits({ xMm: -3, yMm: -3, widthMm: 154, heightMm: 217 }, a5)).toBe(false);
  // 零出血时贴边可用，但越出去一寸就不行。
  expect(boxFits({ xMm: 0, yMm: 0, widthMm: 148, heightMm: 210 }, { ...a5, bleedMm: 0 })).toBe(true);
  expect(boxFits({ xMm: 0, yMm: 0, widthMm: 149, heightMm: 210 }, { ...a5, bleedMm: 0 })).toBe(false);
});

it('放不下要按页点名，后端那句笼统的话不能让用户自己猜是哪一页', () => {
  const a4 = { xMm: 15, yMm: 15, widthMm: 180, heightMm: 267 };
  const a5 = { widthMm: 148, heightMm: 210, marginMm: 15, bleedMm: 3 };
  const book = [
    { pageNumber: 0, frames: [a4], texts: [] },
    { pageNumber: 1, frames: [], texts: [] },
    { pageNumber: 2, frames: [], texts: [a4] },
    { pageNumber: 3, frames: [{ xMm: 15, yMm: 15, widthMm: 118, heightMm: 150 }], texts: [] },
  ];
  // 封面（图框超）和第 2 页（图注超）要点名；空页与放得下的第 3 页不算。
  expect(wholeBookBlockers(book, a5).map(item => item.pageNumber)).toEqual([0, 2]);
  // 一本全是空页的书，缩到多小都不会被服务端拒。
  expect(wholeBookBlockers([{ pageNumber: 1, frames: [], texts: [] }], { widthMm: 50, heightMm: 50, marginMm: 0, bleedMm: 0 })).toEqual([]);
  // 按原尺寸比对自己的每一页，本来就该全部放下——否则"应用到全部页面"一开就禁着。
  expect(wholeBookBlockers(book.map(item => ({ ...item, widthMm: 210, heightMm: 297 })),
    { widthMm: 210, heightMm: 297, marginMm: 15, bleedMm: 3 })).toEqual([]);
});

it('三种出版物在四套语言里各有各的名字，不是一句中文糊过去', () => {
  const keys = ['publicationType', 'publicationType_photobook', 'publicationType_exhibition', 'publicationType_portfolio'];
  const zhCopy = bundles.zh.copy as Record<string, string>;
  for (const locale of ['zh', 'en', 'ja', 'ko'] as const) {
    const copy = bundles[locale].copy as Record<string, string>;
    for (const key of keys) expect(copy[key], `${locale}/${key}`).toBeTruthy();
  }
  setCopyLocale('zh');
  const names = keys.filter(key => key !== 'publicationType').map(key => zh[key]);
  // 选完还是同一个词，等于没得选。
  expect(new Set(names).size).toBe(3);
  for (const key of keys) {
    setCopyLocale('en');
    expect(zh[key], key).not.toMatch(/[一-鿿]/);
    setCopyLocale('ja');
    expect(zh[key], key).not.toBe(zhCopy[key]);
    setCopyLocale('ko');
    expect(zh[key], key).not.toBe(zhCopy[key]);
  }
  setCopyLocale('zh');
});

it('每个开本都有能读出来的标签，A4 之外也不能只显示一串数字', () => {
  for (const preset of paperPresets) {
    expect(presetLabel(preset)).toBeTruthy();
    expect(presetLabel(preset)).toContain(`${preset.widthMm}×${preset.heightMm}`);
  }
  // 展览常用的大幅面要在列表里，不然创建展览册还是只能选书的尺寸。
  expect(paperPresets.map(preset => presetLabel(preset))).toContain('A3 297×420');
});
