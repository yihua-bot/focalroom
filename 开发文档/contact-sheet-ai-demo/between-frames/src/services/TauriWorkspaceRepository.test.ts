import { beforeEach, expect, it, vi } from 'vitest';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc, convertFileSrc: (path: string) => 'asset:' + path }));
import { TauriCanvasRepository } from './TauriCanvasRepository';
import { TauriPublicationRepository } from './TauriPublicationRepository';
import { useUIStore } from '../stores/uiStore';
import { MemoryStorage } from '../test/memoryStorage';
const bookId='11111111-1111-4111-8111-111111111111';
const book={id:bookId,project_id:'project',pub_type:'photobook',title:'书',state:'draft',created_at:'2026-10-21',revision:1};
const receipt=(request:import('@betweenframes/contracts').PublicationCommandRequest,results:unknown[])=>({commandId:request.commandId,projectId:request.projectId,publicationId:request.publicationId,baseRevision:request.baseRevision,revision:request.baseRevision+1,changed:true,results});
beforeEach(() => { ipc.mockReset(); useUIStore.getState().reset(); Object.defineProperty(window,'localStorage',{configurable:true,value:new MemoryStorage()}); });

it('适配 Rust 的 board/publication snake_case，不能把空字段渲染为已接通', async () => {
  ipc.mockResolvedValueOnce([{ id: 'board', project_id: 'project', title: '叙事', state: 'draft', owning_task_id: null, revision: 1 }]);
  expect(await new TauriCanvasRepository().listBoards('project')).toEqual([{ id: 'board', projectId: 'project', title: '叙事', state: 'draft', owningTaskId: null, revision: 1 }]);
  ipc.mockImplementationOnce(async (_command,{request})=>receipt(request,[{...book,id:request.publicationId}]));
  expect(await new TauriPublicationRepository().create('project', '书')).toMatchObject({ projectId: 'project', pubType: 'photobook', createdAt: '2026-10-21' });
  expect(ipc).toHaveBeenLastCalledWith('pub_execute', { request: expect.objectContaining({ baseRevision:0, projectId:'project', operations:[expect.objectContaining({action:{type:'Create',data:{title:'书',pubType:'photobook'}}})] }) });
  ipc.mockResolvedValueOnce([{ id: 'broken', projectId: 'project' }]);
  await expect(new TauriCanvasRepository().listBoards('project')).rejects.toThrow();
});

it('画布照片通过资产别名取得授权缓存路径，过滤隐藏节点', async () => {
  const node = { id: 'node', boardId: 'board', nodeType: 'photo', x: 0, y: 0, width: 100, height: 80,
    rotation: 0, zOrder: 1, assetId: 'asset-not-photo-id', renditionRef: null, contentJson: null, styleJson: null,
    childrenJson: null, locked: false, visible: true };
  const board = { id: 'board', project_id: 'project', title: '叙事', state: 'draft', owning_task_id: null, revision: 1 };
  ipc.mockImplementation(async (name: string) => name === 'canvas_list_boards' ? [board] : {
    board, nodes: [node, { ...node, id: 'hidden', visible: false }], annotations: [],
    photoSources: [{ assetId: 'asset-not-photo-id', photoId: 'original-photo-id', thumbnailPath: '/authorized/cache.jpg' }],
  });
  const repository = new TauriCanvasRepository();
  await repository.listBoards('project');
  const nodes = await repository.getNodes('board');
  expect(nodes).toHaveLength(1);
  expect(nodes[0].thumbnailPath).toBe('asset:/authorized/cache.jpg');
  expect(nodes[0].photoId).toBe('original-photo-id');
  expect(ipc).toHaveBeenCalledWith('canvas_snapshot', { projectId: 'project', boardId: 'board' });
});

it('生成页面返回值必须为正整数，错误回执不能显示成功', async () => {
  for (const invalid of [null, -1, 0, 1.5, '1']) {
    Object.defineProperty(window,'localStorage',{configurable:true,value:new MemoryStorage()});
    const repository=new TauriPublicationRepository();
    ipc.mockResolvedValueOnce([book]).mockImplementationOnce(async(_command,{request})=>receipt(request,[invalid]));
    await expect(repository.generate('project',bookId,2)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});
    expect(repository.pendingCommands('project')).toHaveLength(1);
  }
  Object.defineProperty(window,'localStorage',{configurable:true,value:new MemoryStorage()});
  const repository=new TauriPublicationRepository();
  ipc.mockResolvedValueOnce([book]).mockImplementationOnce(async(_command,{request})=>receipt(request,[3]));
  expect(await repository.generate('project',bookId,2)).toBe(3);
  expect(ipc).toHaveBeenLastCalledWith('pub_execute',{request:expect.objectContaining({baseRevision:1,operations:[expect.objectContaining({action:{type:'Generate',data:{photosPerPage:2,replaceExisting:false}}})]})});
  ipc.mockRejectedValueOnce({code:'CONFLICT',message:'该出版物已有页面',retriable:false});
  await expect(repository.generate('project',bookId)).rejects.toThrow('该出版物已有页面');
  expect(repository.pendingCommands('project')).toEqual([]);
});

it('从选片工作台进入画布保留照片选区，切换项目仍清空', () => {
  useUIStore.getState().selectAll(['a', 'b']);
  useUIStore.getState().setView('canvas');
  expect(useUIStore.getState().selectedIds).toEqual(['a', 'b']);
  useUIStore.getState().reset();
  expect(useUIStore.getState().selectedIds).toEqual([]);
});

it('页面缩略预览只转换后端返回的缓存路径，拒绝无效开本尺寸', async () => {
  const repository = new TauriPublicationRepository();
  const page = { id: 'page', pageNumber: 0, widthMm: 190, heightMm: 257,
    frames: [{ id: 'frame', xMm: 15, yMm: 15, widthMm: 160, heightMm: 227, fitMode: 'fit', thumbnailPath: '/authorized/thumb.jpg' }] };
  ipc.mockResolvedValueOnce([page]);
  expect((await repository.pages('project', 'book'))[0].frames[0].thumbnailSrc).toBe('asset:/authorized/thumb.jpg');
  expect(ipc).toHaveBeenLastCalledWith('pub_pages', { projectId: 'project', publicationId: 'book' });
  ipc.mockResolvedValueOnce([{ ...page, widthMm: 0 }]);
  await expect(repository.pages('project', 'book')).rejects.toThrow();
});
