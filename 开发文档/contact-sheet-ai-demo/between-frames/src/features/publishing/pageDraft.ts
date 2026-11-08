import type { PublicationLayout, PublicationPageUpdate } from '../../domain/publication';

type Page = PublicationLayout['pages'][number];

export const MIN_PAGE_MM = 50;
export const MAX_PAGE_MM = 2000;
export const MAX_BLEED_MM = 25;
export const pageFields = ['widthMm', 'heightMm', 'marginMm', 'bleedMm'] as const;
export type PageValues = Record<(typeof pageFields)[number], string>;
export type PageDraft = { widthMm: number; heightMm: number; marginMm: number; bleedMm: number };

// 常见印刷开本只填尺寸：边距和出血由用户自己定，不替他们决定成品规格。
// A3 是给展览用的：展签和大幅面打样的常见尺寸，只建 A4 的书没法直接开一本展墙册。
export const paperPresets: Array<{ id: string; name: string; widthMm: number; heightMm: number }> = [
  { id: 'a4', name: 'A4', widthMm: 210, heightMm: 297 },
  { id: 'a5', name: 'A5', widthMm: 148, heightMm: 210 },
  { id: 'a3', name: 'A3', widthMm: 297, heightMm: 420 },
  { id: 'square', name: '', widthMm: 210, heightMm: 210 },
  { id: 'landscape', name: '', widthMm: 297, heightMm: 210 },
];

export const presetLabel = (preset: { name: string; widthMm: number; heightMm: number }) =>
  preset.name ? `${preset.name} ${preset.widthMm}×${preset.heightMm}` : `${preset.widthMm}×${preset.heightMm}`;

// 按输入框里的原文比对：210×297 认得出 A4，210.0×297 算自定义——用户手填的数不冒充开本。
export const findPreset = (values: Pick<PageValues, 'widthMm' | 'heightMm'>) =>
  paperPresets.find(preset => String(preset.widthMm) === values.widthMm.trim() && String(preset.heightMm) === values.heightMm.trim())?.id ?? '';

export const pageValues = (page: Page): PageValues =>
  Object.fromEntries(pageFields.map(key => [key, String(page[key])])) as PageValues;

// 规则与版面校验同源：页宽高一律 50–2000mm，出血 0–25mm，边距不许吃掉整页。
export function pageDraft(values: PageValues): PageDraft | undefined {
  const numbers = pageFields.map(key => (values[key].trim() === '' ? NaN : Number(values[key].trim())));
  if (!numbers.every(Number.isFinite)) return undefined;
  const [widthMm, heightMm, marginMm, bleedMm] = numbers;
  if (widthMm < MIN_PAGE_MM || widthMm > MAX_PAGE_MM) return undefined;
  if (heightMm < MIN_PAGE_MM || heightMm > MAX_PAGE_MM) return undefined;
  if (marginMm < 0 || bleedMm < 0 || bleedMm > MAX_BLEED_MM) return undefined;
  if (marginMm * 2 >= Math.min(widthMm, heightMm)) return undefined;
  return { widthMm, heightMm, marginMm, bleedMm };
}

export const editPage = (pageId: string, draft: PageDraft): PublicationPageUpdate => ({ pageId, ...draft });

// 整本统一开本时"以哪一组数为准"：已存页面直接取四个毫米数，不经过输入框那层字符串。
export const pageGeometry = (page: Page): PageDraft =>
  ({ widthMm: page.widthMm, heightMm: page.heightMm, marginMm: page.marginMm, bleedMm: page.bleedMm });

// 一条命令最多带 100 个操作，这是契约里的上限，整本统一开本不能超。
export const MAX_WHOLE_BOOK_PAGES = 100;

type Content = { xMm: number; yMm: number; widthMm: number; heightMm: number };

// 与服务端 rectangle() 同源：内容必须落在 [-出血, 页面+出血] 之间，越一寸整笔命令就会被拒。
export const boxFits = (box: Content, page: PageDraft) =>
  box.xMm >= -page.bleedMm && box.yMm >= -page.bleedMm
  && box.xMm + box.widthMm <= page.widthMm + page.bleedMm
  && box.yMm + box.heightMm <= page.heightMm + page.bleedMm;

// 整本统一开本要一次改所有页面，先把按新尺寸放不下内容的那几页挑出来：
// 后端只回一句笼统的"内容超出页面及出血范围"，不点名是哪一页，所以点名这件事得在出门前做。
export const wholeBookBlockers = <T extends { pageNumber: number; frames: readonly Content[]; texts: readonly Content[] }>(
  pages: readonly T[], draft: PageDraft,
) => pages.filter(page => [...page.frames, ...page.texts].some(box => !boxFits(box, draft)));
