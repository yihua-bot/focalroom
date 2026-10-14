import { describe, expect, it } from 'vitest';
import { MockRepository, createSampleProject } from './MockRepository';

describe('M0 内存仓库的业务边界', () => {
  it('返回隔离快照，28 张示例图都有实际资源', async () => {
    const repository = new MockRepository();
    const project = await repository.get('sample-project');
    expect(project.photos).toHaveLength(28);
    expect(project.photos.every(photo => Boolean(photo.src))).toBe(true);
    project.photos[0].status = 'reject';
    expect((await repository.get(project.id)).photos[0].status).toBe('undecided');
  });
  it('明确设置状态，重复 K 不改变顺序或增加证据，U 才恢复待定', async () => {
    const repository = new MockRepository();
    expect((await repository.decide('sample-project', 'sample-1', 'keep')).sequence).toEqual(['sample-1']);
    expect((await repository.decide('sample-project', 'sample-1', 'keep')).sequence).toEqual(['sample-1']);
    const rejected = await repository.decide('sample-project', 'sample-1', 'reject');
    expect(rejected.photos[0].status).toBe('reject');
    expect(rejected.sequence).toEqual([]);
    expect(rejected.events).toHaveLength(2);
    expect((await repository.decide('sample-project', 'sample-1', 'undecided')).photos[0].status).toBe('undecided');
  });
  it('批量评分与选片验证整个选区，失败不产生部分更新', async () => {
    const repository = new MockRepository();
    await expect(repository.decideMany('sample-project', ['sample-1', 'missing'], 'keep')).rejects.toThrow();
    expect((await repository.get('sample-project')).sequence).toEqual([]);
    const kept = await repository.decideMany('sample-project', ['sample-1', 'sample-2'], 'keep');
    const rated = await repository.rate('sample-project', ['sample-1', 'sample-2'], 4);
    expect(rated.sequence).toEqual(kept.sequence);
    expect(rated.photos.slice(0, 2).map(photo => photo.rating)).toEqual([4, 4]);
    await expect(repository.rate('sample-project', ['sample-1'], 6)).rejects.toThrow();
    expect(await repository.rate('sample-project', ['sample-1', 'sample-2'], 4)).toEqual(rated);
  });
  it('比较与不确定判断都不会自动改写人工状态或顺序', async () => {
    const repository = new MockRepository();
    const before = await repository.decide('sample-project', 'sample-2', 'reject');
    await repository.compare(before.id, 'sample-1', 'sample-2', 'sample-2', '留白');
    const after = await repository.compare(before.id, 'sample-1', 'sample-2', null, '');
    expect(after.photos).toEqual(before.photos);
    expect(after.sequence).toEqual(before.sequence);
    expect(after.events.filter(event => event.type === 'compare')).toHaveLength(2);
  });
  it('拒绝自身比较和不存在的照片，保持原数据', async () => {
    const repository = new MockRepository();
    await expect(repository.compare('sample-project', 'sample-1', 'sample-1', 'sample-1', '')).rejects.toThrow();
    await expect(repository.decide('sample-project', 'missing', 'keep')).rejects.toThrow();
    expect((await repository.get('sample-project')).events).toEqual([]);
  });
  it('重排必须包含原集合且不重复，失败不污染序列', async () => {
    const repository = new MockRepository();
    await repository.decide('sample-project', 'sample-1', 'keep');
    await repository.decide('sample-project', 'sample-2', 'keep');
    await expect(repository.reorder('sample-project', ['sample-1', 'sample-1'])).rejects.toThrow();
    await expect(repository.reorder('sample-project', ['sample-1', 'sample-3'])).rejects.toThrow();
    expect((await repository.get('sample-project')).sequence).toEqual(['sample-1', 'sample-2']);
    expect((await repository.reorder('sample-project', ['sample-2', 'sample-1'])).sequence).toEqual(['sample-2', 'sample-1']);
  });
  it('备注幂等，项目编辑不改变选片', async () => {
    const repository = new MockRepository();
    await repository.note('sample-project', 'sample-1', '光线');
    await repository.note('sample-project', 'sample-1', '光线');
    const project = await repository.edit('sample-project', '测试项目', '克制');
    expect(project.photos[0].note).toBe('光线');
    expect(project.photos[0].status).toBe('undecided');
    expect(project.events).toHaveLength(2);
  });
  it('新建空项目，重新创建仓库后预览状态重置', async () => {
    const repository = new MockRepository();
    expect((await repository.create('空项目', '')).photos).toEqual([]);
    await repository.decide('sample-project', 'sample-1', 'keep');
    expect((await new MockRepository().get('sample-project')).sequence).toEqual([]);
    expect(createSampleProject().photos.some(photo => 'aiSuggested' in photo)).toBe(false);
  });
});
