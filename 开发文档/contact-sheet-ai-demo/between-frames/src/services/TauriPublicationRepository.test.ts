import { beforeEach, expect, it, vi } from 'vitest';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc, convertFileSrc: (path: string) => `local:${path}` }));
import { TauriPublicationRepository } from './TauriPublicationRepository';
import { publicationRequestSchema } from '@betweenframes/contracts';

const blocking = { code: 'low_resolution', severity: 'blocking', pageNumber: 0, frameId: 'f1', textId: null, measured: 169, required: 300 };
const report = (over: Record<string, unknown> = {}) => ({ minPpi: 300, pages: 1, frames: 1, texts: 0, blocking: 1, warnings: 0, infos: 0, truncated: false, issues: [blocking], ...over });
const repo = () => new TauriPublicationRepository();

beforeEach(() => {
  ipc.mockReset();
  window.localStorage.clear();
});

it('印前检查是只读通道：不带修订号、不进命令队列', async () => {
  ipc.mockResolvedValue(report());
  const result = await repo().preflight('p1', 'b1', 300);
  expect(ipc).toHaveBeenCalledWith('pub_preflight', { projectId: 'p1', publicationId: 'b1', minPpi: 300 });
  expect(result).toMatchObject({ blocking: 1, issues: [blocking] });
  expect(ipc.mock.calls.map(call => call[0])).toEqual(['pub_preflight']);
});

it('没有指定门槛时把空值交给后端定，不自己填一个数字', async () => {
  ipc.mockResolvedValue(report({ blocking: 0, warnings: 0, issues: [] }));
  const result = await repo().preflight('p1', 'b1');
  expect(ipc).toHaveBeenCalledWith('pub_preflight', { projectId: 'p1', publicationId: 'b1', minPpi: null });
  expect(result.minPpi).toBe(300);
});

it('汇总与逐条对不上、或数字与严重级越界的结果都不许上屏', async () => {
  ipc.mockResolvedValue(report({ blocking: 0 }));
  await expect(repo().preflight('p1', 'b1')).rejects.toThrow('印前检查结果与版面不一致，请重新检查。');

  ipc.mockResolvedValue(report({ truncated: true }));
  await expect(repo().preflight('p1', 'b1')).rejects.toThrow('印前检查结果不完整，请重新检查。');

  for (const broken of [
    report({ minPpi: 30 }),
    report({ minPpi: 4800 }),
    report({ issues: [{ ...blocking, severity: 'fatal' }] }),
    report({ issues: [{ ...blocking, measured: -1 }] }),
    report({ issues: [{ ...blocking, code: 'Low Resolution' }] }),
    report({ issues: [{ ...blocking, pageNumber: 99_999 }] }),
    report({ pages: -1 }),
  ]) {
    ipc.mockResolvedValue(broken);
    await expect(repo().preflight('p1', 'b1')).rejects.toThrow();
  }
});

const book = 'b2c3d4e5-f6a7-4b8c-9d0e-1f2a3b4c5d6e';
const pageId = 'c3d4e5f6-a7b8-4c9d-8e0f-2a3b4c5d6e7f';
const textId = 'd4e5f6a7-b8c9-4d0e-9f1a-3b4c5d6e7f80';
const caption = { textId, text: '十胜，雪原尽头', xMm: 15, yMm: 264, widthMm: 180, heightMm: 18, sizePt: 10, align: 'left' as const };
const receipt = (request: { commandId: string; baseRevision: number }) => ({
  commandId: request.commandId, projectId: 'p1', publicationId: book,
  baseRevision: request.baseRevision, revision: request.baseRevision + 1, changed: true, results: [null],
});

it('图注的增、改、删都走带重放的命令通道，一次一条并原样送达', async () => {
  ipc.mockImplementation(async (_name: string, args: { request: { commandId: string; baseRevision: number } }) => receipt(args.request));
  const r = repo();
  await r.addText('p1', book, 4, { ...caption, pageId });
  await r.editText('p1', book, 5, caption);
  await r.removeText('p1', book, 6, textId);
  expect(ipc.mock.calls.map(call => call[0])).toEqual(['pub_execute', 'pub_execute', 'pub_execute']);
  expect(ipc.mock.calls.map(call => call[1].request.operations.map((op: { action: { type: string } }) => op.action.type))).toEqual([['AddText'], ['UpdateText'], ['DeleteText']]);
  expect(ipc.mock.calls[0][1].request.operations[0].action.data).toEqual({ ...caption, pageId });
  expect(ipc.mock.calls[2][1].request.operations[0].action.data).toEqual({ textId });
  expect(r.pendingCommands('p1')).toEqual([]);
});

it('纸张尺寸走同一条命令通道：一次 UpdatePage，四个毫米数原样送达', async () => {
  ipc.mockImplementation(async (_name: string, args: { request: { commandId: string; baseRevision: number } }) => receipt(args.request));
  const r = repo();
  await r.editPage('p1', book, 7, { pageId, widthMm: 148, heightMm: 210, marginMm: 12, bleedMm: 3 });
  expect(ipc).toHaveBeenCalledTimes(1);
  const request = ipc.mock.calls[0][1].request;
  expect(request.baseRevision).toBe(7);
  expect(request.operations.map((op: { action: { type: string } }) => op.action.type)).toEqual(['UpdatePage']);
  expect(request.operations[0].action.data).toEqual({ pageId, widthMm: 148, heightMm: 210, marginMm: 12, bleedMm: 3 });
  expect(r.pendingCommands('p1')).toEqual([]);
  // 契约层管的是量程：出血超 25mm、页面短边不足 50mm、编号不像编号，都不该占住命令队列。
  const attempts = [
    r.editPage('p1', book, 8, { pageId, widthMm: 148, heightMm: 210, marginMm: 12, bleedMm: 26 }),
    r.editPage('p1', book, 8, { pageId, widthMm: 40, heightMm: 210, marginMm: 12, bleedMm: 3 }),
    r.editPage('p1', book, 8, { pageId, widthMm: 148, heightMm: 2001, marginMm: 12, bleedMm: 3 }),
    r.editPage('p1', book, 8, { pageId: 'not-a-uuid', widthMm: 148, heightMm: 210, marginMm: 12, bleedMm: 3 }),
  ];
  for (const attempt of attempts) await expect(attempt).rejects.toThrow();
  expect(ipc).toHaveBeenCalledTimes(1);
  expect(r.pendingCommands('p1')).toEqual([]);
});

it('整本统一开本走同一条命令：一页一条操作、一次版本、不留待重放', async () => {
  ipc.mockImplementation(async (_name: string, args: { request: { commandId: string; baseRevision: number; operations: unknown[] } }) => ({
    commandId: args.request.commandId, projectId: 'p1', publicationId: book,
    baseRevision: args.request.baseRevision, revision: args.request.baseRevision + 1, changed: true,
    // 非创建/生成的操作每条回执都是 null；改三页就是三个 null，版本只 +1。
    results: args.request.operations.map(() => null),
  }));
  const r = repo();
  const ids = [pageId, 'a3d4e5f6-a7b8-4c9d-8e0f-2a3b4c5d6e7f', 'b4e5f6a7-b8c9-4d0e-9f1a-3b4c5d6e7f81'];
  const draft = { widthMm: 148, heightMm: 210, marginMm: 12, bleedMm: 3 };
  await r.editPages('p1', book, 9, ids.map(id => ({ pageId: id, ...draft })));
  expect(ipc).toHaveBeenCalledTimes(1);
  const request = ipc.mock.calls[0][1].request;
  expect(request.baseRevision).toBe(9);
  expect(request.operations.map((op: { action: { type: string } }) => op.action.type)).toEqual(['UpdatePage', 'UpdatePage', 'UpdatePage']);
  // 每页各带自己的编号，四个毫米数一致——这正是不用逐页点三次的意义。
  expect(request.operations.map((op: { action: { data: unknown } }) => op.action.data)).toEqual(ids.map(id => ({ pageId: id, ...draft })));
  expect(r.pendingCommands('p1')).toEqual([]);
  // 空列表和超过契约上限的整本都出不了门，不占命令队列。上限是契约里的 100 条操作，界面按同一个数决定要不要给"整本应用"。
  await expect(r.editPages('p1', book, 10, [])).rejects.toThrow();
  const tooMany = Array.from({ length: 101 }, (_unused, index) => ({
    pageId: `${index.toString(16).padStart(8, '0')}-1111-4111-8111-111111111111`, ...draft,
  }));
  await expect(r.editPages('p1', book, 10, tooMany)).rejects.toThrow();
  expect(ipc).toHaveBeenCalledTimes(1);
  expect(r.pendingCommands('p1')).toEqual([]);
  // 100 页正好卡在上限内。
  await expect(r.editPages('p1', book, 10, tooMany.slice(0, 100))).resolves.toBeUndefined();
  expect(ipc).toHaveBeenCalledTimes(2);
});

it('类型和开本跟着创建一起出门：展览 A3 不会被静默写成摄影书 A4', async () => {
  ipc.mockImplementation(async (_name: string, args: { request: { commandId: string; projectId: string; publicationId: string } }) => ({
    commandId: args.request.commandId, projectId: args.request.projectId, publicationId: args.request.publicationId,
    baseRevision: 0, revision: 1, changed: true,
    results: [{ id: args.request.publicationId, project_id: args.request.projectId, pub_type: 'exhibition', title: '展墙册', state: 'draft', created_at: '2026-11-05', revision: 1 }],
  }));
  const r = repo();
  const created = await r.create('p1', '展墙册', { pubType: 'exhibition', widthMm: 297, heightMm: 420 });
  expect(ipc.mock.calls.map(call => call[0])).toEqual(['pub_execute']);
  const request = ipc.mock.calls[0][1].request;
  expect(request.baseRevision).toBe(0);
  expect(request.operations.map((op: { action: { type: string } }) => op.action.type)).toEqual(['Create']);
  expect(request.operations[0].action.data).toEqual({ title: '展墙册', pubType: 'exhibition', widthMm: 297, heightMm: 420 });
  // 新册的编号由前端先定，回执要落在同一条上，否则待确认按钮找不到它。
  expect(created.id).toBe(request.publicationId);
  expect(created).toMatchObject({ projectId: 'p1', pubType: 'exhibition', revision: 1 });
  expect(r.pendingCommands('p1')).toEqual([]);
});

it('陌生类型与越界开本出不了门：不拿默认值补，也不留下待确认的创建', async () => {
  const r = repo();
  const attempts = [
    r.create('p1', '年历', { pubType: 'calendar' as 'photobook', widthMm: 210, heightMm: 297 }),
    r.create('p1', '过头的海报', { pubType: 'exhibition', widthMm: 2001, heightMm: 420 }),
    r.create('p1', '明信片', { pubType: 'photobook', widthMm: 210, heightMm: 49 }),
    r.create('p1', '尺寸不是数', { pubType: 'photobook', widthMm: Number.NaN, heightMm: 297 }),
  ];
  for (const attempt of attempts) await expect(attempt).rejects.toThrow();
  expect(ipc).not.toHaveBeenCalled();
  expect(r.pendingCommands('p1')).toEqual([]);
});

it('越界的图注在出门前就拦下：超长文字、越界字号和不像编号的编号都到不了后端', async () => {
  const r = repo();
  const attempts = [
    r.addText('p1', book, 4, { ...caption, pageId, text: '雪'.repeat(301) }),
    r.addText('p1', book, 4, { ...caption, pageId, sizePt: 73 }),
    r.addText('p1', book, 4, { ...caption, pageId, xMm: Number.NaN }),
    r.addText('p1', book, 4, { ...caption, pageId, text: '' }),
    r.editText('p1', book, 4, { ...caption, textId: 'not-a-uuid' }),
    r.removeText('p1', book, 4, ''),
  ];
  for (const attempt of attempts) await expect(attempt).rejects.toThrow();
  expect(ipc).not.toHaveBeenCalled();
  expect(r.pendingCommands('p1')).toEqual([]);
});

it("审校状态一次只送一个动作，回执落在同一条命令上", async () => {
  const r = repo();
  ipc.mockImplementation((command: string, args: { request: { commandId: string; baseRevision: number } }) => {
    expect(command).toBe("pub_execute");
    return Promise.resolve(receipt(args.request));
  });
  await r.changeState("p1", book, 4, "in_review");
  expect(ipc.mock.calls.map(call => call[0])).toEqual(["pub_execute"]);
  const request = ipc.mock.calls[0][1].request;
  expect(request.baseRevision).toBe(4);
  expect(request.operations).toHaveLength(1);
  expect(request.operations[0].action).toEqual({ type: "UpdateState", data: { state: "in_review" } });
  // 状态改完不留待确认：下一步按钮要立刻按新状态给。
  expect(r.pendingCommands("p1")).toEqual([]);
});

it("没听过的状态出不了门，状态改动也不跟版面动作拼成一条命令", async () => {
  const r = repo();
  await expect(r.changeState("p1", book, 4, "archived" as "in_review")).rejects.toThrow();
  // 版面改动可以和版面改动同一条，状态改动必须单独成条：前后端两份规则都要拦住，否则用户只收到一条拒收。
  const mixed = publicationRequestSchema.safeParse({
    schemaVersion: 3, commandId: crypto.randomUUID(), idempotencyKey: crypto.randomUUID(),
    projectId: "p1", publicationId: book, baseRevision: 4, operations: [
      { opId: crypto.randomUUID(), action: { type: "UpdateState", data: { state: "in_review" } } },
      { opId: crypto.randomUUID(), action: { type: "UpdatePage", data: { pageId, widthMm: 210, heightMm: 297, marginMm: 15, bleedMm: 3 } } },
    ],
  });
  expect(mixed.success).toBe(false);
  expect(ipc).not.toHaveBeenCalled();
  expect(r.pendingCommands("p1")).toEqual([]);
});
it('展览规格走同一条命令通道：一次 UpdateSpec，两个毫米数原样送达', async () => {
  const r = repo();
  ipc.mockImplementation((_command: string, args: { request: { commandId: string; baseRevision: number } }) => Promise.resolve(receipt(args.request)));
  await r.editSpec!('p1', book, 4, { viewingDistanceMm: 2500, hangCenterMm: 1450 });
  expect(ipc.mock.calls.map(call => call[0])).toEqual(['pub_execute']);
  const request = ipc.mock.calls[0][1].request;
  expect(request.baseRevision).toBe(4);
  expect(request.operations).toHaveLength(1);
  expect(request.operations[0].action).toEqual({ type: 'UpdateSpec', data: { viewingDistanceMm: 2500, hangCenterMm: 1450 } });
  // 留空是"明确不记录"，要送的是 null，而不是缺字段或 0。
  await r.editSpec!('p1', book, 5, { viewingDistanceMm: null, hangCenterMm: null });
  expect(ipc.mock.calls[1][1].request.operations[0].action).toEqual({ type: 'UpdateSpec', data: { viewingDistanceMm: null, hangCenterMm: null } });
  expect(r.pendingCommands('p1')).toEqual([]);
});

it('越界的观看距离出不了门，规格改动也不跟版面动作拼成一条命令', async () => {
  const r = repo();
  const attempts = [
    r.editSpec!('p1', book, 4, { viewingDistanceMm: 299, hangCenterMm: 1450 }),
    r.editSpec!('p1', book, 4, { viewingDistanceMm: 20001, hangCenterMm: 1450 }),
    r.editSpec!('p1', book, 4, { viewingDistanceMm: 2500, hangCenterMm: 3001 }),
    r.editSpec!('p1', book, 4, { viewingDistanceMm: Number.NaN, hangCenterMm: null }),
  ];
  for (const attempt of attempts) await expect(attempt).rejects.toThrow();
  expect(ipc).not.toHaveBeenCalled();
  expect(r.pendingCommands('p1')).toEqual([]);
  // 换判据不是排版：和改图框拼一条，会做出"图框改了、判据没换"的半份结果。
  const mixed = publicationRequestSchema.safeParse({
    schemaVersion: 3, commandId: crypto.randomUUID(), idempotencyKey: crypto.randomUUID(),
    projectId: 'p1', publicationId: book, baseRevision: 4, operations: [
      { opId: crypto.randomUUID(), action: { type: 'UpdateSpec', data: { viewingDistanceMm: 2500, hangCenterMm: null } } },
      { opId: crypto.randomUUID(), action: { type: 'UpdateFrame', data: { frameId: pageId, xMm: 15, yMm: 15, widthMm: 180, heightMm: 267, fitMode: 'fit', assetId: null, renditionId: null, cropX: 0.5, cropY: 0.5, cropZoom: 1 } } },
    ],
  });
  expect(mixed.success).toBe(false);
});

it('导出作品清单只发一条只读命令：不带修订号、不进幂等队列', async () => {
  const saved = { path: '/Users/me/清单/worklist.csv', sha256: 'a'.repeat(64), bytes: 4096 };
  ipc.mockResolvedValue(saved);
  await expect(repo().exportWorklist!('p1', 'b1')).resolves.toEqual(saved);
  expect(ipc).toHaveBeenCalledWith('pub_export', { projectId: 'p1', publicationId: 'b1' });
  expect(ipc.mock.calls.map(call => call[0])).toEqual(['pub_export']);
  expect(repo().pendingCommands('p1')).toEqual([]);
});

it('保存对话框里取消不是失败，坏回执不许当成已导出', async () => {
  ipc.mockResolvedValue(null);
  await expect(repo().exportWorklist!('p1', 'b1')).resolves.toBeNull();

  for (const broken of [
    { path: '', sha256: 'a'.repeat(64), bytes: 12 },
    { path: '/tmp/w.csv', sha256: 'short', bytes: 12 },
    { path: '/tmp/w.csv', sha256: 'a'.repeat(64), bytes: -1 },
    { path: '/tmp/w.csv', sha256: 'a'.repeat(64), bytes: 1.5 },
    { path: 42, sha256: 'a'.repeat(64), bytes: 12 },
  ]) {
    ipc.mockResolvedValue(broken);
    await expect(repo().exportWorklist!('p1', 'b1')).rejects.toThrow();
  }
});

const reviewThread = { id: 'c3d4e5f6-a7b8-4c9d-8e0f-2a3b4c5d6e7f', revision: 7, state: 'open', comments: [{ id: 1, body: '第 3 页留白太多', author: 'user:local-owner', createdAt: '2026-11-05T02:00:00Z' }] };
const reviewPage = { publicationId: book, revision: 7, open: 1, threads: [reviewThread] };

it('审阅意见读侧只发一条只读命令，写侧一次一条且不占幂等队列', async () => {
  ipc.mockResolvedValue(reviewPage);
  const r = repo();
  await expect(r.review!('p1', book)).resolves.toMatchObject({ open: 1 });
  expect(ipc).toHaveBeenCalledWith('pub_review', { projectId: 'p1', publicationId: book });
  await r.addReview!('p1', book, '封面挪到后一页');
  expect(ipc.mock.calls[1]).toEqual(['pub_review_write', { projectId: 'p1', publicationId: book, request: { action: 'add', data: { body: '封面挪到后一页' } } }]);
  await r.resolveReview!('p1', book, reviewThread.id, true);
  expect(ipc.mock.calls[2][1].request).toEqual({ action: 'resolve', data: { threadId: reviewThread.id, resolved: true } });
  expect(r.pendingCommands('p1')).toEqual([]);
});

it('空意见和不像编号的编号出不了门；对不上数的回执不许当成"都处理完了"', async () => {
  const r = repo();
  const attempts = [
    r.addReview!('p1', book, ''),
    r.addReview!('p1', book, '   '),
    r.addReview!('p1', book, '意'.repeat(5001)),
    r.resolveReview!('p1', book, '', true),
  ];
  for (const attempt of attempts) await expect(attempt).rejects.toThrow();
  expect(ipc).not.toHaveBeenCalled();
  // 汇总与逐条对不上、意见不属于这本、版本号比当前还新、状态没见过、线程没有正文——都不配上屏。
  for (const broken of [
    { ...reviewPage, publicationId: pageId },
    { ...reviewPage, open: 0 },
    { ...reviewPage, threads: [reviewThread, reviewThread] },
    { ...reviewPage, revision: 6 },
    { ...reviewPage, threads: [{ ...reviewThread, state: 'closed' }] },
    { ...reviewPage, threads: [{ ...reviewThread, comments: [] }] },
    { ...reviewPage, threads: [{ ...reviewThread, comments: [{ ...reviewThread.comments[0], body: '  ' }] }] },
  ]) {
    ipc.mockResolvedValue(broken);
    await expect(r.review!('p1', book)).rejects.toThrow();
  }
});
