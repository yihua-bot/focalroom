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
  it('导入任务把"内容已变化"单独带回来，缺字段或脏值都不出屏', async () => {
    const repository = new TauriRepository();
    const change = { photoId: 'photo', path: '/private/source.jpg', filename: 'source.jpg', previousSha256: 'a'.repeat(64), incomingSha256: 'b'.repeat(64), previousWidth: 3000, previousHeight: 2000, previousSize: 4_194_304, incomingWidth: 4000, incomingHeight: 3000, incomingSize: 8_388_608 };
    const job = (changes: unknown[]) => ({ jobId: 'job', projectId: 'project', status: 'completed', completed: changes.length, total: changes.length, imported: 0, skipped: 0, changes, failures: [] });
    ipc.mockResolvedValueOnce(job([change]));
    expect((await repository.importJob('job')).changes).toEqual([change]);
    expect(ipc).toHaveBeenCalledExactlyOnceWith('get_import_job', { jobId: 'job' });
    for (const broken of [{ ...change, photoId: '' }, { ...change, previousSha256: '' }, { ...change, incomingWidth: Number.NaN }, { ...change, previousSize: '4194304' }]) {
      ipc.mockResolvedValueOnce(job([broken]));
      await expect(repository.importJob('job')).rejects.toThrow();
    }
    // 没有 changes 字段的回执不当成"没有变化"，宁可报错也不静默吞掉外部改写。
    const withoutChanges: Record<string, unknown> = { ...job([change]) };
    delete withoutChanges.changes;
    ipc.mockResolvedValueOnce(withoutChanges);
    await expect(repository.importJob('job')).rejects.toThrow();
  });
  it('确认内容更新只发一条命令，参数里不带界面提供的文件路径', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce(projection(2));
    await repository.get('project');
    ipc.mockResolvedValueOnce(projection(3));
    const updated = await repository.applyPhotoChange('project', 'photo');
    expect(ipc).toHaveBeenLastCalledWith('apply_photo_change', { projectId: 'project', photoId: 'photo', expectedRevision: 2 });
    expect(Object.keys(ipc.mock.calls[1][1] as Record<string, unknown>).sort()).toEqual(['expectedRevision', 'photoId', 'projectId']);
    expect(updated.revision).toBe(3);
  });
  it('读边车是只读一条命令，回执脏值或坏状态都不上屏', async () => {
    const repository = new TauriRepository();
    const proposal = { photoId: 'photo', filename: 'frame-007.jpg', sidecarPath: '/Library/frame-007.xmp', sidecarRating: 4, localRating: 0, appliesRating: true, sidecarPick: true, localStatus: 'undecided', appliesStatus: true, colorLabel: 6, ambiguousRating: false, batchSafe: true };
    ipc.mockResolvedValueOnce([proposal]);
    expect(await repository.sidecarProposals?.('project')).toEqual([proposal]);
    expect(ipc).toHaveBeenLastCalledWith('sidecar_proposals', { projectId: 'project' });
    for (const broken of [{ ...proposal, photoId: '' }, { ...proposal, sidecarRating: 9 }, { ...proposal, sidecarRating: '4' }, { ...proposal, localStatus: 'archived' }, { ...proposal, ambiguousRating: 'false' }, { ...proposal, sidecarPath: '' }, { ...proposal, batchSafe: 'true' }, { ...proposal, batchSafe: undefined }]) {
      ipc.mockResolvedValueOnce([broken]);
      await expect(repository.sidecarProposals?.('project')).rejects.toThrow();
    }
    // 空值走 null，不是把字段省掉：省掉说明后端换了形状，宁可报错。
    ipc.mockResolvedValueOnce([{ ...proposal, sidecarRating: undefined }]);
    await expect(repository.sidecarProposals?.('project')).rejects.toThrow();
    ipc.mockResolvedValueOnce([{ ...proposal, sidecarRating: null, appliesRating: false }]);
    expect((await repository.sidecarProposals?.('project'))?.[0].sidecarRating).toBeNull();
  });
  it('回填一条判断只发照片编号，参数里没有边车路径', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce(projection(2));
    await repository.get('project');
    ipc.mockResolvedValueOnce(projection(5));
    const updated = await repository.applySidecarJudgement?.('project', 'photo');
    expect(ipc).toHaveBeenLastCalledWith('apply_sidecar_judgement', { projectId: 'project', photoId: 'photo', expectedRevision: 2 });
    expect(Object.keys(ipc.mock.calls[1][1] as Record<string, unknown>).sort()).toEqual(['expectedRevision', 'photoId', 'projectId']);
    expect(updated?.revision).toBe(5);
  });
  it('一键回填只发项目编号与期望修订号，计数越界或没有台账就不认', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce(projection(2));
    await repository.get('project');
    ipc.mockResolvedValueOnce({ applied: 3, skipped: 1, project: projection(5) });
    expect(await repository.backfillSidecarJudgements?.('project')).toEqual({ applied: 3, skipped: 1 });
    expect(ipc).toHaveBeenLastCalledWith('backfill_sidecar_judgements', { projectId: 'project', expectedRevision: 2 });
    expect(Object.keys(ipc.mock.calls[1][1] as Record<string, unknown>).sort()).toEqual(['expectedRevision', 'projectId']);
    for (const broken of [{ applied: -1, skipped: 0, project: projection(5) }, { applied: 1.5, skipped: 0, project: projection(5) }, { applied: '3', skipped: 0, project: projection(5) }, { applied: 3, project: projection(5) }]) {
      ipc.mockResolvedValueOnce(broken);
      await expect(repository.backfillSidecarJudgements?.('project')).rejects.toThrow();
    }
  });
});
describe('整项目点名与一次修完的回执边界', () => {
  const ledger = { total: 120, scanned: 120, healthy: 112, needsRepair: 5, blocked: 3, reasons: { original_missing: 3 }, items: [{ photoId: 'photo', filename: '海边.jpg', outcome: 'needs_repair', rebuilt: ['preview'], detail: '可只读原图重算' }] };
  it('点名单只认非负计数与逐张原因，越界数字、文字计数和没有文件名的条目一律拒收', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce(ledger);
    expect(await repository.auditPreviews!('project')).toEqual(ledger);
    expect(ipc).toHaveBeenLastCalledWith('audit_photo_previews', { projectId: 'project' });
    for (const invalid of [{ ...ledger, total: -1 }, { ...ledger, reasons: { original_missing: '很多' } }, { ...ledger, items: [{ photoId: 12 }] }, { ...ledger, items: [{ photoId: 'photo', outcome: 'needs_repair', rebuilt: [], detail: '' }] }]) {
      ipc.mockResolvedValueOnce(invalid);
      await expect(repository.auditPreviews!('project')).rejects.toThrow();
    }
  });
  it('一次修完的计数缺了就当没修，不把空值冒充零张', async () => {
    const repository = new TauriRepository();
    const recheck = { drafts: 3, draftsScanned: 3, recheckOwed: 3, recheckedDrafts: 3, recheckFailed: 0, recheckRemaining: 0 };
    ipc.mockResolvedValueOnce({ ...ledger, repaired: 5, refused: 0, remaining: 2, recheck });
    const report = await repository.repairAllPreviews!('project');
    expect(report.repaired).toBe(5);
    expect(report.remaining).toBe(2);
    expect(report.recheck).toEqual(recheck);
    expect(ipc).toHaveBeenLastCalledWith('repair_all_broken_previews', { projectId: 'project' });
    ipc.mockResolvedValueOnce(ledger);
    const plain = await repository.repairAllPreviews!('project');
    expect(plain.repaired).toBeUndefined();
    expect(plain.recheck).toBeUndefined();
  });
  it('修完那一轮的口径欠账跟着点名一起过界，自相矛盾的数字与没修成的时候分开口径', async () => {
    const repository = new TauriRepository();
    const recheck = { drafts: 40, draftsScanned: 40, recheckOwed: 23, recheckedDrafts: 3, recheckFailed: 2, recheckRemaining: 20 };
    ipc.mockResolvedValueOnce({ ...ledger, repaired: 5, recheck });
    expect((await repository.repairAllPreviews!('project')).recheck).toEqual(recheck);
    ipc.mockResolvedValueOnce({ ...ledger, repaired: 5, recheck: { ...recheck, recheckRemaining: 0 } });
    await expect(repository.repairAllPreviews!('project')).rejects.toThrow();
    ipc.mockResolvedValueOnce({ ...ledger, repaired: 5, recheckError: { code: 'BACKGROUND', message: '本地评估这趟没跑成' } });
    const failed = await repository.repairAllPreviews!('project');
    expect(failed.repaired).toBe(5);
    expect(failed.recheck).toBeUndefined();
    expect(failed.recheckError).toEqual({ code: 'BACKGROUND', message: '本地评估这趟没跑成' });
    ipc.mockResolvedValueOnce({ ...ledger, repaired: 5, recheckError: { code: '', message: '本地评估这趟没跑成' } });
    await expect(repository.repairAllPreviews!('project')).rejects.toThrow();
  });
  it('接着补口径走自己的命令，几个数字对不上就整份拒收', async () => {
    const repository = new TauriRepository();
    const report = { drafts: 40, draftsScanned: 40, recheckOwed: 23, recheckedDrafts: 3, recheckFailed: 2, recheckRemaining: 20 };
    ipc.mockResolvedValueOnce(report);
    expect(await repository.recheckProjectEvidence!('project')).toEqual(report);
    expect(ipc).toHaveBeenLastCalledWith('recheck_project_draft_evidence', { projectId: 'project' });
    for (const invalid of [
      { ...report, recheckRemaining: 0 },
      { ...report, recheckedDrafts: 30 },
      { ...report, recheckOwed: 41 },
      { ...report, draftsScanned: 41 },
      { ...report, recheckFailed: -1 },
      { ...report, drafts: '四十' },
      { ...report, recheckRemaining: 1.5 },
    ]) {
      ipc.mockResolvedValueOnce(invalid);
      await expect(repository.recheckProjectEvidence!('project')).rejects.toThrow();
    }
  });
  it('单张只读体检走自己的命令，回执缺字段或脏值不上屏', async () => {
    const repository = new TauriRepository();
    const outcome = { photoId: 'photo', outcome: 'needs_repair', rebuilt: [], detail: '可只读原图重算' };
    ipc.mockResolvedValueOnce(outcome);
    expect(await repository.inspectPreview!('project', 'photo')).toEqual(outcome);
    expect(ipc).toHaveBeenLastCalledWith('inspect_photo_preview', { projectId: 'project', photoId: 'photo' });
    for (const broken of [{ ...outcome, outcome: 7 }, { ...outcome, rebuilt: 'preview' }, { photoId: 'photo', outcome: 'healthy' }]) {
      ipc.mockResolvedValueOnce(broken);
      await expect(repository.inspectPreview!('project', 'photo')).rejects.toThrow();
    }
  });
});

describe('历史观察像素版本的点名与回填回执边界', () => {
  const ledger = { rows: 5, scanned: 5, photos: 4, ready: 3, refused: 2, reasons: { original_missing: 1, pipeline_changed: 1 }, items: [{ observationId: 1, photoId: 'photo', filename: '旧版一.jpg', outcome: 'ready' }, { observationId: 2, photoId: 'photo2', filename: '原图丢了.jpg', outcome: 'original_missing' }] };
  const recheck = { drafts: 9, draftsScanned: 2, recheckOwed: 2, recheckedDrafts: 2, recheckFailed: 0, recheckRemaining: 0 };
  it('点名单两条命令各走各的，能证明与补不了凑不齐点到数就整份拒收', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce(ledger);
    expect(await repository.auditObservationVersions!('project')).toEqual(ledger);
    expect(ipc).toHaveBeenLastCalledWith('audit_observation_pixel_versions', { projectId: 'project' });
    ipc.mockResolvedValueOnce({ ...ledger, backfilled: 3, remaining: 0, recheck });
    expect(await repository.backfillObservationVersions!('project')).toEqual({ ...ledger, backfilled: 3, remaining: 0, recheck });
    expect(ipc).toHaveBeenLastCalledWith('backfill_observation_pixel_versions', { projectId: 'project' });
    for (const invalid of [
      { ...ledger, ready: 4 },
      { ...ledger, scanned: 6 },
      { ...ledger, photos: 6 },
      { ...ledger, refused: -1 },
      { ...ledger, reasons: { original_missing: '很多' } },
      { ...ledger, reasons: { original_missing: 2, pipeline_changed: 1 } },
      { ...ledger, items: [{ observationId: 0, photoId: 'photo', filename: '旧版一.jpg', outcome: 'ready' }] },
      { ...ledger, items: [{ observationId: 1, photoId: '', filename: '旧版一.jpg', outcome: 'ready' }] },
      { ...ledger, items: [{ observationId: 1, photoId: 'photo', outcome: 'ready' }] },
    ]) {
      ipc.mockResolvedValueOnce(invalid);
      await expect(repository.auditObservationVersions!('project')).rejects.toThrow();
    }
  });
  it('补了几条不能多于能证明的，剩下几条要能对得上，缺了就当没补', async () => {
    const repository = new TauriRepository();
    for (const invalid of [
      { ...ledger, backfilled: 4, remaining: 0 },
      { ...ledger, backfilled: 3, remaining: 3 },
      { ...ledger, backfilled: 1.5, remaining: 2 },
    ]) {
      ipc.mockResolvedValueOnce(invalid);
      await expect(repository.backfillObservationVersions!('project')).rejects.toThrow();
    }
    ipc.mockResolvedValueOnce(ledger);
    const plain = await repository.backfillObservationVersions!('project');
    expect(plain.backfilled).toBeUndefined();
    expect(plain.remaining).toBeUndefined();
  });
  it('版本号补上了但口径那趟没跑成，两件事分开收，自相矛盾的欠账不上屏', async () => {
    const repository = new TauriRepository();
    ipc.mockResolvedValueOnce({ ...ledger, backfilled: 3, remaining: 0, recheck: { ...recheck, recheckRemaining: 1 } });
    await expect(repository.backfillObservationVersions!('project')).rejects.toThrow();
    ipc.mockResolvedValueOnce({ ...ledger, backfilled: 3, remaining: 0, recheckError: { code: 'BACKGROUND', message: '本地评估这趟没跑成' } });
    const failed = await repository.backfillObservationVersions!('project');
    expect(failed.backfilled).toBe(3);
    expect(failed.recheck).toBeUndefined();
    expect(failed.recheckError).toEqual({ code: 'BACKGROUND', message: '本地评估这趟没跑成' });
    ipc.mockResolvedValueOnce({ ...ledger, backfilled: 3, remaining: 0, recheckError: { code: '', message: '本地评估这趟没跑成' } });
    await expect(repository.backfillObservationVersions!('project')).rejects.toThrow();
  });
});
