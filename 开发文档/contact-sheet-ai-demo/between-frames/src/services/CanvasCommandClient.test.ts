import { beforeEach, expect, it, vi } from 'vitest';
import { CanvasCommandClient } from './CanvasCommandClient';
import type { CanvasCommandRequest } from '../domain/canvasCommands';
const projectId = '15740517-8824-468c-b22b-6e099d67a197';
const boardId = 'ff4082f2-44d9-483b-a82f-2fa07212f0a7';
class MemoryStorage implements Storage {
  private data = new Map<string, string>();
  get length() { return this.data.size; }
  clear() { this.data.clear(); }
  getItem(key: string) { return this.data.get(key) ?? null; }
  setItem(key: string, value: string) { this.data.set(key, value); }
  removeItem(key: string) { this.data.delete(key); }
  key(index: number) { return [...this.data.keys()][index] ?? null; }
}
const localStorage = new MemoryStorage();
const actions = [{ type: 'Undo' as const, data: { boardId } }];
function receipt(request: CanvasCommandRequest) {
  return { commandId: request.commandId, projectId, boardId, baseRevision: request.baseRevision, revision: request.baseRevision + 1, changed: true, results: [true] };
}
beforeEach(() => localStorage.clear());

it('丢失结果后阻止新编辑，刷新后以相同完整请求确认，不新建命令', async () => {
  let saved: CanvasCommandRequest | undefined;
  const firstSend = vi.fn(async (request: CanvasCommandRequest) => { saved = request; throw new Error('connection closed after commit'); });
  const client = new CanvasCommandClient(firstSend, localStorage);
  await expect(client.execute(projectId, boardId, 2, actions)).rejects.toMatchObject({ code: 'CANVAS_PENDING' });
  expect(client.pending(projectId)).toHaveLength(1);
  await expect(client.execute(projectId, boardId, 2, actions)).rejects.toMatchObject({ code: 'CANVAS_PENDING' });
  expect(firstSend).toHaveBeenCalledTimes(1);
  const send = vi.fn(async (request: CanvasCommandRequest) => receipt(request));
  const reopened = new CanvasCommandClient(send, localStorage);
  expect(reopened.pending('another-account-project')).toEqual([]);
  await reopened.retry(projectId, boardId);
  expect(send).toHaveBeenCalledWith(saved);
  expect(reopened.pending(projectId)).toEqual([]);
});

it('明确版本冲突结束旧请求，用户重新编辑时创建新请求且不自动重试', async () => {
  const send = vi.fn<(request: CanvasCommandRequest) => Promise<unknown>>().mockRejectedValueOnce(Object.assign(new Error('画布已更新'), { code: 'REVISION_CONFLICT' }));
  const client = new CanvasCommandClient(send, localStorage);
  await expect(client.execute(projectId, boardId, 2, actions)).rejects.toMatchObject({ code: 'REVISION_CONFLICT' });
  expect(send).toHaveBeenCalledTimes(1); expect(client.pending(projectId)).toEqual([]);
  send.mockImplementation(async request => receipt(request));
  await client.execute(projectId, boardId, 3, actions);
  expect(send.mock.calls[0][0].commandId).not.toBe(send.mock.calls[1][0].commandId);
  expect(send.mock.calls[1][0].baseRevision).toBe(3);
});

it('结果属于其他画布时保留待确认请求，不能伪装成功', async () => {
  const client = new CanvasCommandClient(async request => ({ ...receipt(request), boardId: projectId }), localStorage);
  await expect(client.execute(projectId, boardId, 2, actions)).rejects.toMatchObject({ code: 'CANVAS_PENDING' });
  expect(client.pending(projectId)).toHaveLength(1);
});

it('持久化失败时不向后端发出操作', async () => {
  const send = vi.fn();
  const storage = new MemoryStorage();
  storage.setItem = () => { throw new Error('quota'); };
  const client = new CanvasCommandClient(send, storage);
  await expect(client.execute(projectId, boardId, 2, actions)).rejects.toThrow('存储空间');
  expect(send).not.toHaveBeenCalled();
});

it('外层回执正确但节点结果损坏时，仍保留原操作以供确认', async () => {
  const client = new CanvasCommandClient(async request => ({ ...receipt(request), results: [{ id: request.operations[0].opId, boardId, nodeType: 'sticky' }] }), localStorage);
  await expect(client.execute(projectId, boardId, 2, [{ type: 'AddText', data: { boardId, nodeType: 'sticky', x: 0, y: 0, width: 200, height: 100, content: '想法' } }])).rejects.toMatchObject({ code: 'CANVAS_PENDING' });
  expect(client.pending(projectId)).toHaveLength(1);
});

it('重试尚未完成时重复点击只发送一次', async () => {
  let finish!: (value: unknown) => void;
  let request!: CanvasCommandRequest;
  const send = vi.fn((value: CanvasCommandRequest) => { request = value; return new Promise(resolve => { finish = resolve; }); });
  const client = new CanvasCommandClient(send, localStorage);
  const waiting = client.execute(projectId, boardId, 2, actions);
  await expect(client.retry(projectId, boardId)).rejects.toThrow('请稍候');
  expect(send).toHaveBeenCalledTimes(1);
  finish(receipt(request)); await waiting;
});
