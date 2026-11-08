import { bundles, setCopyLocale, zh } from '../../app/copy';
import { expect, it } from 'vitest';
import { publicationActionSchema } from '@betweenframes/contracts';
import { distanceLimits, hangLimits, specDraft, specValues } from './exhibitionSpec';
import type { SpecValues } from './exhibitionSpec';

const values = (over: Partial<SpecValues> = {}): SpecValues => ({ viewingDistanceMm: '2500', hangCenterMm: '1450', ...over });

it('读回观看距离与悬挂中心线：页幅那些键不是展览规格，脏值当没记录', () => {
  expect(specValues('{"width_mm":210,"height_mm":297,"viewing_distance_mm":2500,"hang_center_mm":1450}'))
    .toEqual({ viewingDistanceMm: '2500', hangCenterMm: '1450' });
  // 库里的垃圾不该被回填成"用户填过的数"：一显示出来，下次保存就等于替它作保。
  expect(specValues('{"viewing_distance_mm":"两米五"}')).toEqual({ viewingDistanceMm: '', hangCenterMm: '' });
  expect(specValues('{"viewing_distance_mm":50}')).toEqual({ viewingDistanceMm: '', hangCenterMm: '' });
  expect(specValues('{"hang_center_mm":9000}')).toEqual({ viewingDistanceMm: '', hangCenterMm: '' });
  expect(specValues('{"viewing_distance_mm":null}')).toEqual({ viewingDistanceMm: '', hangCenterMm: '' });
  // 界面上允许的是闭区间，读回也要允许同一条边界，否则保存一次就把合法值显示成空。
  expect(specValues(`{"viewing_distance_mm":${distanceLimits.min},"hang_center_mm":${hangLimits.max}}`))
    .toEqual({ viewingDistanceMm: String(distanceLimits.min), hangCenterMm: String(hangLimits.max) });
  for (const junk of ['', 'not json', '[]', 'null', '"2500"', '{"viewing_distance_mm":1e999}']) {
    expect(specValues(junk), junk).toEqual({ viewingDistanceMm: '', hangCenterMm: '' });
  }
});

it('留空是"明确不记录"，填了就必须落在物理范围内，而且不允许半份规格出门', () => {
  expect(specDraft(values())).toEqual({ viewingDistanceMm: 2500, hangCenterMm: 1450 });
  expect(specDraft({ viewingDistanceMm: '', hangCenterMm: '' })).toEqual({ viewingDistanceMm: null, hangCenterMm: null });
  // 清掉观看距离、留下悬挂中心线：这是两个明确的动作，不是忘了填。
  expect(specDraft({ viewingDistanceMm: '', hangCenterMm: '1450' })).toEqual({ viewingDistanceMm: null, hangCenterMm: 1450 });
  expect(specDraft(values({ viewingDistanceMm: '  '}))).toEqual({ viewingDistanceMm: null, hangCenterMm: 1450 });
  expect(specDraft(values({ viewingDistanceMm: '2500.5' }))).toEqual({ viewingDistanceMm: 2500.5, hangCenterMm: 1450 });
  // 一个数越界，整条就不出门——否则后端要么收到半份新规格，要么把越界值拒了而用户不知道是哪个字段。
  expect(specDraft(values({ viewingDistanceMm: String(distanceLimits.min - 1) }))).toBeUndefined();
  expect(specDraft(values({ viewingDistanceMm: String(distanceLimits.max + 1) }))).toBeUndefined();
  expect(specDraft(values({ hangCenterMm: String(hangLimits.min - 1) }))).toBeUndefined();
  expect(specDraft(values({ hangCenterMm: String(hangLimits.max + 1) }))).toBeUndefined();
  for (const bad of ['abc', 'Infinity', 'NaN', '2500 1450', '-1']) {
    expect(specDraft(values({ viewingDistanceMm: bad })), bad).toBeUndefined();
    expect(specDraft(values({ hangCenterMm: bad })), bad).toBeUndefined();
  }
});

it('界面上的范围和契约是同一套数：UI 放行的后端不会拒，UI 拦下的契约也拦', () => {
  const passes = (draft: ReturnType<typeof specDraft>) => draft !== undefined
    && publicationActionSchema.safeParse({ type: 'UpdateSpec', data: draft }).success;
  for (const draft of [
    values(),
    { viewingDistanceMm: '', hangCenterMm: '' },
    { viewingDistanceMm: String(distanceLimits.min), hangCenterMm: String(hangLimits.min) },
    { viewingDistanceMm: String(distanceLimits.max), hangCenterMm: String(hangLimits.max) },
  ]) {
    expect(passes(specDraft(draft)), JSON.stringify(draft)).toBe(true);
  }
  for (const draft of [
    values({ viewingDistanceMm: '299.9' }),
    values({ viewingDistanceMm: '20000.1' }),
    values({ hangCenterMm: '3000.1' }),
  ]) {
    expect(passes(specDraft(draft)), JSON.stringify(draft)).toBe(false);
  }
});

it('读回来的规格原样再保存一次还是同一组数，不会被当成改动', () => {
  const stored = '{"width_mm":210,"viewing_distance_mm":2500,"hang_center_mm":1450}';
  expect(specDraft(specValues(stored))).toEqual({ viewingDistanceMm: 2500, hangCenterMm: 1450 });
  // 空着没记录的也是同理：库里没这个键，界面显示为空，出门时是 null 而不是 0。
  expect(specDraft(specValues('{"width_mm":210}'))).toEqual({ viewingDistanceMm: null, hangCenterMm: null });
});

it('四套语言都补齐了展览规格文案，回落中文不算翻译完成', () => {
  const keys = ['publicationSpec', 'publicationSpecHint', 'publicationSpecDistance', 'publicationSpecHang',
    'publicationSpecNote', 'publicationSpecInvalid', 'publicationSpecSave', 'publicationSpecSaved',
    'publicationSpecVersion', 'preflight_exhibition_low_resolution', 'preflight_viewing_distance_missing'];
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
  // "定展览规格"是后端写进版本标签的那句原话，界面靠它认这条版本，改动要两头一起改。
  expect(zh.publicationSpecVersion).toBe('定展览规格');
});
