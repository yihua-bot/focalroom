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
