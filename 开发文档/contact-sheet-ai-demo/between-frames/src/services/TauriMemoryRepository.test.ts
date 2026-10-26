import { beforeEach, expect, it, vi } from 'vitest';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc }));
import { TauriMemoryRepository } from './TauriMemoryRepository';

beforeEach(() => ipc.mockReset());
it('通信重试保留记忆请求 ID，避免重复排除证据', async () => {
  ipc.mockRejectedValueOnce(new Error('lost response')).mockImplementationOnce((_name, { request }) => ({
    ok: true, requestId: request.requestId, data: { requestId: request.requestId, changed: true, memoryRevision: 4 }, error: null,
  }));
  const receipt = await new TauriMemoryRepository().request('p1', { type: 'SetEvidenceExcluded', evidenceId: 7, excluded: true });
  expect(receipt.memoryRevision).toBe(4);
  expect(ipc.mock.calls[0]).toEqual(ipc.mock.calls[1]);
  expect(ipc.mock.calls[0][1].request).not.toHaveProperty('actor');
});
it('业务冲突不自动重试或生成新身份', async () => {
  ipc.mockResolvedValue({ ok: false, requestId: 'r', data: null, error: { code: 'NOT_FOUND', message: '证据不属于本项目', retryable: false } });
  await expect(new TauriMemoryRepository().request('p1', { type: 'SetEvidenceExcluded', evidenceId: 8, excluded: true })).rejects.toMatchObject({ code: 'NOT_FOUND' });
  expect(ipc).toHaveBeenCalledTimes(1);
});
it('搜索传递关键词，拒绝缺失排除状态的投影及身份错误的回执', async () => {
  const repo = new TauriMemoryRepository();
  ipc.mockResolvedValue({ evidence: [], hypotheses: [], notice: 'n' });
  await repo.query('p1', '逆光');
  expect(ipc).toHaveBeenCalledWith('get_memory_projection', { projectId: 'p1', query: '逆光' });
  ipc.mockResolvedValue({ evidence: [{ id: 1, projectId: 'p1', photoId: null, kind: 'note', detail: {}, createdAt: 'now' }], hypotheses: [], notice: 'n' });
  await expect(repo.query('p1')).rejects.toThrow();
  ipc.mockResolvedValue({ ok: true, requestId: 'wrong', data: { requestId: 'wrong', changed: true, memoryRevision: 1 }, error: null });
  await expect(repo.request('p1', { type: 'ConfirmHypothesis', hypothesisId: 1 })).rejects.toThrow('身份不匹配');
});

it('旧投影保守默认隔离，拒绝混入可编辑列表的其他项目假设', async () => {
 const repo = new TauriMemoryRepository();
 ipc.mockResolvedValue({ evidence: [], hypotheses: [], notice: 'n' });
 expect(await repo.query('p1')).toMatchObject({ crossProjectEnabled: false, authorHypotheses: [] });
 const h = { id: 1, projectId: 'other', scope: 'project_local', statement: '留白', evidenceIds: [1], counterexampleIds: [],
  status: 'confirmed', confidence: 'tentative', userCorrection: null, disabled: false, createdAt: 'n', updatedAt: 'n' };
 ipc.mockResolvedValue({ evidence: [], hypotheses: [h], notice: 'n' });
 await expect(repo.query('p1')).rejects.toThrow('范围不匹配');
 ipc.mockResolvedValue({ evidence: [], hypotheses: [], notice: 'n', crossProjectEnabled: true, authorHypotheses: [{ ...h, scope: 'cross_project' }] });
 expect((await repo.query('p1')).authorHypotheses).toHaveLength(1);
 ipc.mockResolvedValue({ evidence: [], hypotheses: [], notice: 'n', crossProjectEnabled: false, authorHypotheses: [{ ...h, scope: 'cross_project' }] });
 await expect(repo.query('p1')).rejects.toThrow('范围不匹配');
 ipc.mockResolvedValue({ evidence: [], hypotheses: [], notice: 'n', crossProjectEnabled: true, authorHypotheses: [{ ...h, scope: 'cross_project', status: 'unconfirmed' }] });
 await expect(repo.query('p1')).rejects.toThrow('范围不匹配');
});

it('云端作者记忆转换字段命名并拒绝异常回包', async () => {
  const repo = new TauriMemoryRepository();
  ipc.mockResolvedValue([{ id: 'h1', statement: '偏好低饱和', status: 'confirmed', confidence: 'moderate', project_id: 'cloud-2', project_title: '另一个项目', created_at: '2026-10-27T00:00:00Z', updated_at: '2026-10-27T00:00:00Z' }]);
  await expect(repo.cloudAuthorMemories!('p1')).resolves.toEqual([{ id: 'h1', statement: '偏好低饱和', status: 'confirmed', confidence: 'moderate', projectId: 'cloud-2', projectTitle: '另一个项目', createdAt: '2026-10-27T00:00:00Z', updatedAt: '2026-10-27T00:00:00Z' }]);
  expect(ipc).toHaveBeenCalledWith('cloud_author_memories', { projectId: 'p1' });
  ipc.mockResolvedValue([{ id: 'h1', statement: 'x', status: 'bogus', confidence: 'moderate', project_id: 'p', project_title: 't', created_at: 'n', updated_at: 'n' }]);
  await expect(repo.cloudAuthorMemories!('p1')).rejects.toThrow();
  ipc.mockResolvedValue({ not: 'array' });
  await expect(repo.cloudAuthorMemories!('p1')).rejects.toThrow();
});
