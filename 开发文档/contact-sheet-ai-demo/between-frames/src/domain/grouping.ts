import type { Photo } from './models';

export type GridGrouping = 'none' | 'status' | 'rating' | 'prefix';

export interface PhotoGroup {
  key: string;
  label: string;
  photos: Photo[];
}

const STATUS_ORDER = ['keep', 'undecided', 'reject'] as const;
const STATUS_LABELS: Record<string, string> = { keep: '保留', undecided: '待定', reject: '淘汰' };

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
  const labels = (key: string) => grouping === 'status' ? `${STATUS_LABELS[key] ?? key} (${buckets.get(key)!.length})`
    : grouping === 'rating' ? `${key === '0' ? '未评分' : `${key} 星`} (${buckets.get(key)!.length})`
    : `${key} (${buckets.get(key)!.length})`;
  const keys = [...buckets.keys()].sort((a, b) => grouping === 'status'
    ? STATUS_ORDER.indexOf(a as typeof STATUS_ORDER[number]) - STATUS_ORDER.indexOf(b as typeof STATUS_ORDER[number])
    : grouping === 'rating' ? Number(b) - Number(a)
    : a.localeCompare(b, 'zh-CN'));
  return keys.map(key => ({ key, label: labels(key), photos: buckets.get(key)! }));
}
