import { getCopyLocale, zh } from '../app/copy';
import type { Photo } from './models';

export type GridGrouping = 'none' | 'status' | 'rating' | 'prefix';

export interface PhotoGroup {
  key: string;
  label: string;
  photos: Photo[];
}

const STATUS_ORDER = ['keep', 'undecided', 'reject'] as const;
// 只存键名：分组标签在渲染时取，切语言才能立刻跟着变。
const STATUS_LABEL_KEYS: Record<string, string> = { keep: 'photo.keep', undecided: 'photo.undecided', reject: 'photo.reject' };

/** 文件名前缀：去掉扩展名与结尾的序号/分隔符，如 DSC_012.jpg → DSC。 */
export function namePrefix(name: string): string {
  const stem = name.replace(/\.[^.]+$/, '').replace(/[\s_-]*\d+$/, '').trim();
  return stem || name;
}

/** 网格分组：不改照片顺序语义，组内保持传入顺序；键值固定，便于测试与虚拟滚动稳定性。 */
export function groupPhotos(photos: Photo[], grouping: GridGrouping): PhotoGroup[] {
  if (grouping === 'none') return [{ key: 'all', label: '', photos }];
  const buckets = new Map<string, Photo[]>();
  for (const photo of photos) {
    const key = grouping === 'status' ? photo.status : grouping === 'rating' ? String(photo.rating) : namePrefix(photo.name);
    const bucket = buckets.get(key);
    if (bucket) bucket.push(photo);
    else buckets.set(key, [photo]);
  }
  const labels = (key: string) => grouping === 'status' ? `${STATUS_LABEL_KEYS[key] ? zh[STATUS_LABEL_KEYS[key]] : key} (${buckets.get(key)!.length})`
    : grouping === 'rating' ? `${key === '0' ? zh.unrated : zh.starOption.replace('{count}', key)} (${buckets.get(key)!.length})`
    : `${key} (${buckets.get(key)!.length})`;
  const keys = [...buckets.keys()].sort((a, b) => grouping === 'status'
    ? STATUS_ORDER.indexOf(a as typeof STATUS_ORDER[number]) - STATUS_ORDER.indexOf(b as typeof STATUS_ORDER[number])
    : grouping === 'rating' ? Number(b) - Number(a)
    : a.localeCompare(b, getCopyLocale()));
  return keys.map(key => ({ key, label: labels(key), photos: buckets.get(key)! }));
}
