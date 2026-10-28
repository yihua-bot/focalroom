import { beforeEach,expect,it,vi } from 'vitest';
const ipc=vi.hoisted(()=>vi.fn());vi.mock('@tauri-apps/api/core',()=>({invoke:ipc,convertFileSrc:(path:string)=>path}));
import { TauriPublicationRepository } from './TauriPublicationRepository';
import { MemoryStorage } from '../test/memoryStorage';
import type { PublicationCommandRequest,PublicationFrameUpdate,PublicationTextGeometry } from '@betweenframes/contracts';
const project='project',book=crypto.randomUUID();
const frame:PublicationFrameUpdate={frameId:crypto.randomUUID(),assetId:'asset',renditionId:'render',xMm:15,yMm:20,widthMm:40,heightMm:50,fitMode:'fill',cropX:.3,cropY:.7,cropZoom:1.5};
const text:PublicationTextGeometry={textId:crypto.randomUUID(),pageId:crypto.randomUUID(),xMm:25,yMm:30,widthMm:60,heightMm:20};
beforeEach(()=>{ipc.mockReset();Object.defineProperty(window,'localStorage',{configurable:true,value:new MemoryStorage()});});
it('照片与文字整组一次发送，丢回执后重开只确认同一完整请求',async()=>{
  ipc.mockRejectedValueOnce(new Error('closed after commit')).mockImplementation(async(_command,args:{request:PublicationCommandRequest})=>({commandId:args.request.commandId,projectId:project,publicationId:book,baseRevision:2,revision:3,changed:true,results:[null,null]}));const repo=new TauriPublicationRepository();await expect(repo.arrangeObjects(project,book,2,[frame],[text])).rejects.toMatchObject({code:'PUBLICATION_PENDING'});expect(ipc).toHaveBeenCalledOnce();expect(ipc.mock.calls[0][1].request.operations.map((op:{action:unknown})=>op.action)).toEqual([{type:'UpdateFrame',data:frame},{type:'UpdateTextGeometry',data:text}]);
  const reopened=new TauriPublicationRepository();await reopened.retryPending(project,book);expect(ipc.mock.calls[1]).toEqual(ipc.mock.calls[0]);expect(reopened.pendingCommands(project)).toEqual([]);
});
it('空组、重复目标或超过100项在写队列前拒绝',async()=>{
  const repo=new TauriPublicationRepository();for(const [frames,texts]of [[[],[]],[[frame,frame],[]],[Array.from({length:101},()=>({...frame,frameId:crypto.randomUUID()})),[]]] as [PublicationFrameUpdate[],PublicationTextGeometry[]][]){await expect(repo.arrangeObjects(project,book,2,frames,texts)).rejects.toThrow();}expect(ipc).not.toHaveBeenCalled();expect(repo.pendingCommands(project)).toEqual([]);
});

it('跨页链接与转单页复用持久命令，回执丢失后原样确认且不接收替换来源字段',async()=>{
  const geometry={xMm:15,yMm:20,widthMm:350,heightMm:220,fitMode:'fill' as const,cropX:.3,cropY:.7,cropZoom:1.5},data={frameId:frame.frameId,leftPageId:crypto.randomUUID(),rightPageId:crypto.randomUUID(),geometry};
  ipc.mockRejectedValueOnce(new Error('closed after commit')).mockImplementation(async(_command,args:{request:PublicationCommandRequest})=>({commandId:args.request.commandId,projectId:project,publicationId:book,baseRevision:args.request.baseRevision,revision:args.request.baseRevision+1,changed:true,results:[null]}));
  await expect(new TauriPublicationRepository().setSpreadFrame(project,book,8,data)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});const sent=ipc.mock.calls[0];await new TauriPublicationRepository().retryPending(project,book);expect(ipc.mock.calls[1]).toEqual(sent);
  const repo=new TauriPublicationRepository();await repo.unsetSpreadFrame(project,book,9,{frameId:frame.frameId,targetPageId:data.rightPageId,geometry:{...geometry,widthMm:160}});
  expect(ipc.mock.calls[2][1].request.operations[0].action).toMatchObject({type:'UnsetSpreadFrame',data:{frameId:frame.frameId,targetPageId:data.rightPageId}});
  await expect(repo.setSpreadFrame(project,book,10,{...data,assetId:'foreign'} as typeof data)).rejects.toThrow();expect(ipc).toHaveBeenCalledTimes(3);
});

it('整份图层与分组只有一个待确认人工命令，重开确认不重新生成编号或树',async()=>{
 const data={nodes:[{id:frame.frameId,kind:'frame' as const,parentId:null,name:'封面',visible:false,locked:true}]};
 ipc.mockRejectedValueOnce(new Error('lost after commit')).mockImplementation(async(_cmd,args:{request:PublicationCommandRequest})=>({commandId:args.request.commandId,projectId:project,publicationId:book,baseRevision:4,revision:5,changed:true,results:[null]}));
 await expect(new TauriPublicationRepository().setLayers(project,book,4,data)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});await new TauriPublicationRepository().retryPending(project,book);expect(ipc.mock.calls[1]).toEqual(ipc.mock.calls[0]);expect(ipc.mock.calls[0][1].request.operations).toHaveLength(1);expect(ipc.mock.calls[0][1].request.operations[0].action).toEqual({type:'SetPublicationLayers',data});
});

it('整组移除照片和文字使用同一持久请求，丢回执重开不会拆成重复删除',async()=>{
  const objects=[{kind:'frame' as const,id:frame.frameId},{kind:'text' as const,id:text.textId}];
  ipc.mockRejectedValueOnce(new Error('lost after commit')).mockImplementation(async(_cmd,args:{request:PublicationCommandRequest})=>({commandId:args.request.commandId,projectId:project,publicationId:book,baseRevision:6,revision:7,changed:true,results:[null,null]}));
  await expect(new TauriPublicationRepository().removeObjects(project,book,6,objects)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});
  expect(ipc.mock.calls[0][1].request.operations.map((op:{action:unknown})=>op.action)).toEqual([{type:'RemoveFrame',data:{frameId:frame.frameId}},{type:'RemoveText',data:{textId:text.textId}}]);
  await new TauriPublicationRepository().retryPending(project,book);expect(ipc.mock.calls[1]).toEqual(ipc.mock.calls[0]);const count=ipc.mock.calls.length;
  const repo=new TauriPublicationRepository();for(const invalid of [[],[objects[0],objects[0]],Array.from({length:101},()=>({kind:'text' as const,id:crypto.randomUUID()})),[{kind:'group',id:crypto.randomUUID()}]as unknown as typeof objects])await expect(repo.removeObjects(project,book,7,invalid)).rejects.toThrow();
  expect(ipc).toHaveBeenCalledTimes(count);expect(repo.pendingCommands(project)).toEqual([]);
});

it('页面网格与参考线是一个完整持久请求，重开确认原设置与基础版本，不混入几何写入',async()=>{
 const data={pageId:crypto.randomUUID(),settings:{columns:3,rows:4,columnGapMm:5,rowGapMm:4,verticalGuidesMm:[25,100],horizontalGuidesMm:[30]}};
 ipc.mockRejectedValueOnce(new Error('lost after commit')).mockImplementation(async(_cmd,args:{request:PublicationCommandRequest})=>({commandId:args.request.commandId,projectId:project,publicationId:book,baseRevision:9,revision:10,changed:true,results:[null]}));
 await expect(new TauriPublicationRepository().setPageGuides(project,book,9,data)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});await new TauriPublicationRepository().retryPending(project,book);expect(ipc.mock.calls[1]).toEqual(ipc.mock.calls[0]);expect(ipc.mock.calls[0][1].request.operations.map((op:{action:unknown})=>op.action)).toEqual([{type:'SetPageGuides',data}]);
 await expect(new TauriPublicationRepository().setPageGuides(project,book,10,{...data,settings:{...data.settings,source:'foreign'}} as typeof data)).rejects.toThrow();expect(ipc).toHaveBeenCalledTimes(2);
});

it('母版包含网格的选择随原待确认请求恢复，旧请求不补写新的授权字段',async()=>{
 const data={masterId:crypto.randomUUID(),name:'网格母版',sourcePageId:crypto.randomUUID(),expectedMasterRevision:null,includeText:false,includeGuides:true};
 ipc.mockRejectedValueOnce(new Error('lost after commit')).mockImplementation(async(_cmd,args:{request:PublicationCommandRequest})=>({commandId:args.request.commandId,projectId:project,publicationId:book,baseRevision:args.request.baseRevision,revision:args.request.baseRevision+1,changed:true,results:[null]}));
 await expect(new TauriPublicationRepository().saveMaster(project,book,10,data)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});await new TauriPublicationRepository().retryPending(project,book);expect(ipc.mock.calls[1]).toEqual(ipc.mock.calls[0]);expect(ipc.mock.calls[0][1].request.operations[0].action.data.includeGuides).toBe(true);
 const {includeGuides:unused,...legacy}=data;void unused;await new TauriPublicationRepository().saveMaster(project,book,11,legacy);expect(ipc.mock.calls[2][1].request.operations[0].action.data).toEqual(legacy);expect(ipc.mock.calls[2][1].request.operations[0].action.data).not.toHaveProperty('includeGuides');
});
