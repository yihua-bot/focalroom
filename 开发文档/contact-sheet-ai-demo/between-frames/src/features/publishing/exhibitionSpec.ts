// 展览的观看距离与悬挂中心线存在 spec_json 里（没有单独的表，也不该为两个数建表）。
// 这里只负责读回与出门前的校验；分辨率到底怎么判，是后端按距离算出来的事。
import type { PublicationSpecUpdate } from '@betweenframes/contracts';

// 30 厘米到手里的画册距离以下、20 米的一面展墙以上，这两个数已经没有物理意义了。
export const distanceLimits = { min: 300, max: 20000 };
export const hangLimits = { min: 300, max: 3000 };

export interface SpecValues {
  viewingDistanceMm: string;
  hangCenterMm: string;
}

const blank: SpecValues = { viewingDistanceMm: '', hangCenterMm: '' };

/** 读回已记录的规格。脏值一律当"没记录"，不把库里的垃圾当成用户填过的数。 */
export function specValues(specJson: string): SpecValues {
  let parsed: unknown;
  try {
    parsed = JSON.parse(specJson);
  } catch {
    return blank;
  }
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return blank;
  const record = parsed as Record<string, unknown>;
  const stored = (key: string, limits: { min: number; max: number }) => {
    const value = record[key];
    return typeof value === 'number' && Number.isFinite(value) && value >= limits.min && value <= limits.max ? String(value) : '';
  };
  return { viewingDistanceMm: stored('viewing_distance_mm', distanceLimits), hangCenterMm: stored('hang_center_mm', hangLimits) };
}

/** 草稿是否还能出门：留空是"明确不记录"（null），填了就必须落在范围内。
 *  整条命令一次替换两个数，所以不能出现"一个是新值、一个是上次的"半份规格。 */
export function specDraft(values: SpecValues): PublicationSpecUpdate | undefined {
  const field = (raw: string, limits: { min: number; max: number }): number | null | undefined => {
    const text = raw.trim();
    if (!text) return null;
    const value = Number(text);
    return Number.isFinite(value) && value >= limits.min && value <= limits.max ? value : undefined;
  };
  const viewing = field(values.viewingDistanceMm, distanceLimits);
  const hang = field(values.hangCenterMm, hangLimits);
  if (viewing === undefined || hang === undefined) return undefined;
  return { viewingDistanceMm: viewing, hangCenterMm: hang };
}
