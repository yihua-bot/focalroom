import type { PublicationLayout, PublicationTextNew, PublicationTextUpdate } from '../../domain/publication';

type Page = PublicationLayout['pages'][number];
type StoredText = Page['texts'][number];

export const MAX_CAPTION_CHARS = 300;
export const MIN_SIZE_PT = 6;
export const MAX_SIZE_PT = 72;
const DEFAULT_SIZE_PT = 10;
const DEFAULT_HEIGHT_MM = 18;

export type CaptionAlign = 'left' | 'center' | 'right';
export const captionAligns: CaptionAlign[] = ['left', 'center', 'right'];
export const captionPositions = ['xMm', 'yMm', 'widthMm', 'heightMm'] as const;
export type CaptionValues = Record<(typeof captionPositions)[number], string> & {
  text: string;
  sizePt: string;
  align: CaptionAlign;
};
export type CaptionDraft = {
  text: string;
  xMm: number;
  yMm: number;
  widthMm: number;
  heightMm: number;
  sizePt: number;
  align: CaptionAlign;
};

const number = (raw: string) => (raw.trim() === '' ? NaN : Number(raw.trim()));
const isAlign = (value: unknown): value is CaptionAlign =>
  value === 'left' || value === 'center' || value === 'right';

// 文字内容 JSON 的写法由后端定；这里只读回能编辑的值，坏记录退回空串而不是替它编一段说法。
export function readCaption(text: StoredText) {
  let content = '';
  try {
    const value: unknown = JSON.parse(text.contentJson);
    if (value && typeof value === 'object' && typeof (value as { text?: unknown }).text === 'string') {
      content = (value as { text: string }).text;
    }
  } catch {
    content = '';
  }
  let sizePt = DEFAULT_SIZE_PT;
  let align: CaptionAlign = 'left';
  try {
    const value: unknown = JSON.parse(text.styleJson ?? '');
    if (value && typeof value === 'object') {
      const style = value as { sizePt?: unknown; align?: unknown };
      if (typeof style.sizePt === 'number' && Number.isFinite(style.sizePt)
        && style.sizePt >= MIN_SIZE_PT && style.sizePt <= MAX_SIZE_PT) sizePt = style.sizePt;
      if (isAlign(style.align)) align = style.align;
    }
  } catch {
    // 样式缺失或读不回时按默认值显示，保存时会被界面上的合法值覆盖。
  }
  return { text: content, sizePt, align };
}

export function captionValues(text: StoredText): CaptionValues {
  const read = readCaption(text);
  return {
    text: read.text,
    xMm: String(text.xMm),
    yMm: String(text.yMm),
    widthMm: String(text.widthMm),
    heightMm: String(text.heightMm),
    sizePt: String(read.sizePt),
    align: read.align,
  };
}

// 新图注默认落在下边距之上的横条里：不必让用户先猜一组坐标，也不越进出血区。
export function newCaption(page: Page): CaptionValues {
  const height = Math.min(DEFAULT_HEIGHT_MM, (page.heightMm - page.marginMm * 2) / 2);
  return {
    text: '',
    xMm: String(page.marginMm),
    yMm: String(page.heightMm - page.marginMm - height),
    widthMm: String(page.widthMm - page.marginMm * 2),
    heightMm: String(height),
    sizePt: String(DEFAULT_SIZE_PT),
    align: 'left',
  };
}

export function captionDraft(values: CaptionValues): CaptionDraft | undefined {
  const text = values.text.trim();
  const [xMm, yMm, widthMm, heightMm, sizePt] = [
    ...captionPositions.map(key => number(values[key])),
    number(values.sizePt),
  ];
  if (!text || [...text].length > MAX_CAPTION_CHARS) return undefined;
  if (![xMm, yMm, widthMm, heightMm, sizePt].every(Number.isFinite)) return undefined;
  if (widthMm <= 0 || heightMm <= 0 || sizePt < MIN_SIZE_PT || sizePt > MAX_SIZE_PT) return undefined;
  return { text, xMm, yMm, widthMm, heightMm, sizePt, align: values.align };
}

export const addCaption = (textId: string, pageId: string, draft: CaptionDraft): PublicationTextNew =>
  ({ textId, pageId, ...draft });
export const editCaption = (textId: string, draft: CaptionDraft): PublicationTextUpdate =>
  ({ textId, ...draft });
