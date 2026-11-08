// 印前检查的展示层：引擎只给码和数字，这里的职责是把"169 / 300"说成一句人话。
import { zh } from '../../app/copy';
import type { PreflightIssue } from '../../domain/publication';

/** 数字的单位随码变化：分辨率是 PPI，越界量是毫米，裁切放大是倍数。
 *  不在表里的码一律不显示数字——页码一类的数当成"实测值"只会误导。 */
const UNITS: Record<string, string> = {
  low_resolution: 'PPI',
  exhibition_low_resolution: 'PPI',
  bleed_shortfall: 'mm',
  trim_crossing_no_bleed: 'mm',
  mixed_page_size: 'mm',
  text_outside_safety: 'mm',
  spine_crossing: 'mm',
  crop_enlargement: '×',
  shallow_bit_depth: 'bit',
};

export function preflightLabel(code: string): string {
  const key = `preflight_${code}`;
  return zh[key] === key ? code : zh[key];
}

export function preflightSeverity(severity: PreflightIssue['severity']): string {
  const key = `preflight_severity_${severity}`;
  return zh[key] === key ? severity : zh[key];
}

export function preflightScope(issue: PreflightIssue): string {
  if (issue.pageNumber === null) return zh.publicationPreflightWholeBook;
  if (issue.pageNumber === 0) return zh.publicationCover;
  return zh.publicationPageNumber.replace('{number}', String(issue.pageNumber));
}

function value(raw: number, unit: string): string {
  const rounded = unit === 'PPI' ? Math.round(raw) : Number(raw.toFixed(2));
  return `${rounded}${unit ? ' ' : ''}${unit}`;
}

export function preflightDetail(issue: PreflightIssue): string {
  // 整本级问题不是"实测/要求"的比对着陆点，单独说一句更清楚。
  if (issue.code === 'sequence_not_placed') {
    return zh.publicationPreflightUnplaced.replace('{count}', String(issue.required ?? 0));
  }
  const unit = UNITS[issue.code];
  if (unit === undefined || issue.measured === null) return '';
  const measured = value(issue.measured, unit);
  if (issue.required === null) return zh.publicationPreflightMeasured.replace('{value}', measured);
  return zh.publicationPreflightMeasuredRequired
    .replace('{measured}', measured)
    .replace('{required}', value(issue.required, unit));
}

export function preflightSummary(report: { blocking: number; warnings: number; infos: number }): string {
  if (report.blocking > 0) {
    return zh.publicationPreflightBlocking.replace('{count}', String(report.blocking));
  }
  if (report.warnings > 0) {
    return zh.publicationPreflightWarnings.replace('{count}', String(report.warnings));
  }
  return zh.publicationPreflightClean;
}
