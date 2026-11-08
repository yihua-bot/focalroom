import { bundles, setCopyLocale } from '../../app/copy';
import { expect, it } from 'vitest';
import { preflightDetail, preflightLabel, preflightScope, preflightSeverity, preflightSummary } from './preflightDetails';
import type { PreflightIssue } from '../../domain/publication';

const CODES = ['frame_unbound', 'source_missing', 'original_offline', 'rendition_offline', 'low_resolution', 'exhibition_low_resolution', 'viewing_distance_missing', 'crop_enlargement', 'untagged_rendition', 'untagged_source', 'shallow_bit_depth', 'stretched_fit', 'bleed_shortfall', 'trim_crossing_no_bleed', 'spine_crossing', 'text_outside_safety', 'empty_page', 'mixed_page_size', 'page_gap', 'sequence_not_placed'];
const FRAMING = ['publicationPreflight', 'publicationPreflightRun', 'publicationPreflightRerun', 'publicationPreflightMinPpi', 'publicationPreflightFailed', 'publicationPreflightBlocking', 'publicationPreflightWarnings', 'publicationPreflightClean', 'publicationPreflightTruncated', 'publicationPreflightMeasured', 'publicationPreflightMeasuredRequired', 'publicationPreflightUnplaced', 'publicationPreflightWholeBook', 'preflight_severity_blocking', 'preflight_severity_warning', 'preflight_severity_info'];
const issue = (over: Partial<PreflightIssue>): PreflightIssue => ({ code: 'low_resolution', severity: 'blocking', pageNumber: 3, frameId: 'f1', textId: null, measured: 169.4, required: 300, ...over });

it('分辨率与出血成对报出实测和要求，单位随这条判定的含义变', () => {
  setCopyLocale('zh');
  expect(preflightDetail(issue({}))).toBe('实测 169 PPI，要求 300 PPI');
  expect(preflightDetail(issue({ code: 'bleed_shortfall', measured: 1.256, required: 3 }))).toBe('实测 1.26 mm，要求 3 mm');
  expect(preflightDetail(issue({ code: 'crop_enlargement', severity: 'info', measured: 2.5, required: 2 }))).toBe('实测 2.5 ×，要求 2 ×');
  expect(preflightDetail(issue({ code: 'shallow_bit_depth', severity: 'warning', measured: 8, required: null }))).toBe('实测 8 bit');
  expect(preflightDetail(issue({ code: 'trim_crossing_no_bleed', severity: 'warning', measured: 8, required: null }))).toBe('实测 8 mm');
  // 页码和"没有色彩配置"不是实测/要求的比对，硬拼数字只会误导。
  expect(preflightDetail(issue({ code: 'page_gap', severity: 'warning', frameId: null, measured: 3, required: 5 }))).toBe('');
  expect(preflightDetail(issue({ code: 'untagged_source', severity: 'warning', measured: null, required: null }))).toBe('');
  // 展览那条也是分辨率比对，单位同样是 PPI，但要求数是按观看距离算出来的，不是 300。
  expect(preflightDetail(issue({ code: 'exhibition_low_resolution', measured: 25, required: 34 }))).toBe('实测 25 PPI，要求 34 PPI');
  expect(preflightDetail(issue({ code: 'viewing_distance_missing', severity: 'warning', frameId: null, measured: null, required: null }))).toBe('');
});

it('问题落在哪一页要说清楚：整本、封面和正文页各有说法', () => {
  setCopyLocale('zh');
  expect(preflightScope(issue({ pageNumber: null }))).toBe('整本');
  expect(preflightScope(issue({ pageNumber: 0 }))).toBe('封面');
  expect(preflightScope(issue({ pageNumber: 7 }))).toBe('第 7 页');
  expect(preflightDetail(issue({ code: 'sequence_not_placed', severity: 'warning', pageNumber: null, frameId: null, measured: null, required: 3 }))).toBe('3 张已选照片还没进书稿');
});

it('结论先说能不能送印，再说要复核几项', () => {
  setCopyLocale('zh');
  expect(preflightSummary({ blocking: 2, warnings: 5, infos: 1 })).toBe('2 项必须先处理，现在还不能送印。');
  expect(preflightSummary({ blocking: 0, warnings: 5, infos: 1 })).toBe('没有阻断项，另有 5 项建议复核。');
  expect(preflightSummary({ blocking: 0, warnings: 0, infos: 3 })).toBe('没有发现问题，这份书稿可以送印。');
  expect(preflightSeverity('blocking')).toBe('不能送印');
  expect(preflightSeverity('warning')).toBe('建议复核');
  expect(preflightLabel('low_resolution')).toBe('按现在的版面尺寸印出来分辨率不足');
  // 展览的两条要说清判据从哪来，否则用户看到的是"同样的红字"，不知道这条不是按 300 判的。
  expect(preflightLabel('exhibition_low_resolution')).toBe('按记录的观看距离仍然不够清晰');
  expect(preflightLabel('viewing_distance_missing')).toBe('没有记录观看距离，分辨率仍按印刷基线判定');
  expect(preflightScope(issue({ code: 'viewing_distance_missing', severity: 'warning', pageNumber: null, frameId: null, textId: null, measured: null, required: null }))).toBe('整本');
});

it('引擎新增的码原样露出，不替它编一句好听的说法', () => {
  setCopyLocale('zh');
  expect(preflightLabel('brand_new_code')).toBe('brand_new_code');
  expect(preflightSeverity('future_severity' as PreflightIssue['severity'])).toBe('future_severity');
});

it('四套语言都补齐了印前检查文案，回落中文不算翻译完成', () => {
  for (const locale of ['zh', 'en', 'ja', 'ko']) {
    const copy = bundles[locale].copy as Record<string, string>;
    for (const code of CODES) expect(copy[`preflight_${code}`], `${locale}/preflight_${code}`).toBeTruthy();
    for (const key of FRAMING) expect(copy[key], `${locale}/${key}`).toBeTruthy();
  }
  for (const code of CODES) {
    setCopyLocale('en');
    expect(preflightLabel(code), code).not.toMatch(/[一-鿿]/);
    setCopyLocale('ja');
    expect(preflightLabel(code), code).not.toBe(code);
    setCopyLocale('ko');
    expect(preflightLabel(code), code).not.toBe(code);
  }
  setCopyLocale('zh');
});
