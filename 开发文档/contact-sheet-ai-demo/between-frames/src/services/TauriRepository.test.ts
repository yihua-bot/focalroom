import { beforeEach, describe, expect, it, vi } from 'vitest';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc, convertFileSrc: (path: string) => path, isTauri: () => true }));
import { TauriRepository } from './TauriRepository';

const projection = (revision: number) => ({
  id: 'project', archived: false, title: `项目 ${revision}`, brief: '', revision, updatedAt: '2026-10-13',
  photos: [], sequence: [], events: [], versions: [],
  history: { undoGroupId: null, redoGroupId: null, undoCount: 0, redoCount: 0 },
});
const success = (revision: number) => ({
  ok: true, requestId: 'ack', revision, error: null,
  data: { requestId: 'original', revision, changed: true, transactionGroupId: 'group', project: projection(revision) },
});
beforeEach(() => ipc.mockReset());

describe('类型化 IPC 的重试与投影', () => {
  it('请求精简界面投影，只有显式检视时按项目和照片 ID 读取原图', async () => {
    const repository = new TauriRepository();
    const raw = { ...projection(2), photos: [{ id: 'photo', index: 1, name: 'detail.png', originalPath: '/private/source.png', thumbnailPath: 'thumb', previewPath: 'preview', contentSha256: 'hash', width: 2400, height: 1600, fileSize: 42, status: 'undecided', note: '', rating: 0, isMissing: false }] };
    ipc.mockResolvedValueOnce(raw);
    const first = await repository.get('project');
    expect(ipc).toHaveBeenCalledExactlyOnceWith('get_project', { projectId: 'project', compact: true });
    ipc.mockResolvedValueOnce(raw);
    const second = await repository.get('project');
    expect(second.photos[0].loadOriginal).toBe(first.photos[0].loadOriginal);
    ipc.mockResolvedValueOnce({ path: '/authorized/original.png', kind: 'original' });
    const loaded = await second.photos[0].loadOriginal!();
    expect(loaded.src).toBe('/authorized/original.png');
    expect(loaded.kind).toBe('original');
    expect(ipc).toHaveBeenLastCalledWith('get_original_photo', { projectId: 'project', photoId: 'photo' });
  });
  it('内嵌预览使用限定的 JPEG 数据源，拒绝其他协议与过大回执', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce({ ...projection(2), photos: [{ id: 'photo', index: 1, name: 'camera.CR2', originalPath: '/private/source.CR2', thumbnailPath: 'thumb', previewPath: 'preview', contentSha256: 'hash', width: 1936, height: 1288, fileSize: 42, status: 'undecided', note: '', rating: 0, isMissing: false }] });
    const photo = (await repository.get('project')).photos[0];
    const dataUrl = 'data:image/jpeg;base64,/9j/2Q==';
    ipc.mockResolvedValueOnce({ kind: 'embedded_preview', dataUrl });
    expect(await photo.loadOriginal!()).toEqual({ src: dataUrl, kind: 'embedded_preview' });
    for (const invalid of ['https://example.com/camera.jpg', 'data:image/svg+xml;base64,abcd', 'data:image/jpeg;base64,' + 'a'.repeat(5_600_001)]) {
      ipc.mockResolvedValueOnce({ kind: 'embedded_preview', dataUrl: invalid });
      await expect(photo.loadOriginal!()).rejects.toThrow();
    }
  });
  it('通信回执丢失时重用完整 envelope，不生成第二个命令身份', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce(projection(2));
    await repository.get('project');
    ipc.mockRejectedValueOnce(new Error('connection lost')).mockResolvedValueOnce(success(3));
    await repository.edit('project', '新标题', '');
    const first = ipc.mock.calls[1];
    const retry = ipc.mock.calls[2];
    expect(first).toEqual(retry);
    expect(first[0]).toBe('execute_gui_command');
    expect(first[1].request.expectedProjectRevision).toBe(2);
    expect(first[1].request).not.toHaveProperty('actorType');
    expect(first[1].request).not.toHaveProperty('source');
  });
  it('业务 revision 冲突直接返回，不能自动刷新 revision 强行写入', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce(projection(2));
    await repository.get('project');
    ipc.mockResolvedValueOnce({ ok: false, requestId: 'ack', data: null, revision: 3, error: { code: 'REVISION_CONFLICT', message: '版本冲突', retryable: true, currentRevision: 3 } });
    await expect(repository.edit('project', '新标题', '')).rejects.toMatchObject({ code: 'REVISION_CONFLICT', currentRevision: 3 });
    expect(ipc).toHaveBeenCalledTimes(2);
  });
  it('旧命令回放不会让界面倒退，重新查询当前权威投影', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce(projection(4));
    await repository.get('project');
    ipc.mockResolvedValueOnce(success(3)).mockResolvedValueOnce(projection(5));
    expect((await repository.edit('project', '历史重试', '')).revision).toBe(5);
    expect(ipc.mock.calls[2][0]).toBe('get_project');
    ipc.mockResolvedValueOnce(projection(4));
    expect((await repository.get('project')).revision).toBe(5);
  });
});
