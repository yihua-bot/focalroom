import { describe, expect, it } from 'vitest';
import { selectPhotos } from './selection';

describe('选区语义', () => {
  it('范围只使用当前可见顺序，逆向选择与追加不重复', () => {
    expect(selectPhotos(['b'], 'b', 'd', ['a', 'b', 'c', 'd'], { range: true })).toEqual(['b', 'c', 'd']);
    expect(selectPhotos(['d'], 'd', 'b', ['a', 'b', 'c', 'd'], { range: true })).toEqual(['b', 'c', 'd']);
    expect(selectPhotos(['b'], 'b', 'd', ['b', 'd'], { range: true, toggle: true })).toEqual(['b', 'd']);
    expect(selectPhotos(['a'], 'hidden', 'd', ['b', 'd'], { range: true })).toEqual(['d']);
  });
  it('点选替换，修饰键切换，原数组保持不变', () => {
    const before = ['a', 'b'];
    expect(selectPhotos(before, 'a', 'b', before, { toggle: true })).toEqual(['a']);
    expect(selectPhotos(before, 'a', 'c', [...before, 'c'])).toEqual(['c']);
    expect(before).toEqual(['a', 'b']);
  });
});
