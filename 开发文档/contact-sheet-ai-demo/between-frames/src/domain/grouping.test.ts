import { describe, expect, it } from 'vitest';
import { groupPhotos, namePrefix, type GridGrouping } from './grouping';
import type { Photo } from './models';

const photo = (id: string, name: string, status: Photo['status'] = 'undecided', rating = 0): Photo =>
  ({ id, name, status, rating, index: 1, src: '', note: '' });

describe('网格分组', () => {
  it('不分组时返回单一组', () => {
    const groups = groupPhotos([photo('a', 'a.jpg'), photo('b', 'b.jpg')], 'none');
    expect(groups).toHaveLength(1);
    expect(groups[0].label).toBe('');
    expect(groups[0].photos).toHaveLength(2);
  });
  it('按状态分组且顺序固定为保留→待定→淘汰', () => {
    const groups = groupPhotos([photo('k', 'a.jpg', 'keep'), photo('r', 'b.jpg', 'reject'), photo('u', 'c.jpg', 'undecided')], 'status');
    expect(groups.map(group => group.key)).toEqual(['keep', 'undecided', 'reject']);
    expect(groups[0].label).toContain('保留 (1)');
  });
  it('按评分分组且高分在前', () => {
    const groups = groupPhotos([photo('a', 'a.jpg', 'keep', 3), photo('b', 'b.jpg', 'keep', 5), photo('c', 'c.jpg', 'keep', 0)], 'rating');
    expect(groups.map(group => group.key)).toEqual(['5', '3', '0']);
    expect(groups[2].label).toContain('未评分');
  });
  it('按文件名前缀分组并剥离结尾序号', () => {
    const groups: ReturnType<typeof groupPhotos> = groupPhotos([photo('a', 'DSC_012.jpg'), photo('b', 'DSC_003.jpg'), photo('c', 'IMG_001.jpg')], 'prefix');
    expect(groups.map(group => group.key)).toEqual(['DSC', 'IMG']);
    expect(groups[0].photos).toHaveLength(2);
  });
  it('前缀剥离处理纯数字与分隔符', () => {
    expect(namePrefix('DSC_012.jpg')).toBe('DSC');
    expect(namePrefix('IMG-9.png')).toBe('IMG');
    expect(namePrefix('12345.jpg')).toBe('12345.jpg');
    expect(namePrefix('portrait 04.tif')).toBe('portrait');
  });
  it('空输入安全', () => {
    for (const key of ['none', 'status', 'rating', 'prefix'] as GridGrouping[]) {
      const groups = groupPhotos([], key);
      expect(groups.every(group => group.photos.length === 0)).toBe(true);
    }
  });
});
