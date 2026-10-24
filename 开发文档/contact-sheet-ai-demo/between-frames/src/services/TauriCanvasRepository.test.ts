// @vitest-environment jsdom
import { beforeEach,expect,it,vi } from 'vitest';
const ipc=vi.hoisted(()=>vi.fn());
vi.mock('@tauri-apps/api/core',()=>({invoke:ipc,convertFileSrc:(path:string)=>path}));
import { TauriCanvasRepository } from './TauriCanvasRepository';
const context={draftId:'child',parent:{kind:'board',id:'parent',revision:2},parentTitle:'旧方案',discussion:{draftId:'parent',draftRevision:2,cursor:0,totalMessages:0,eventCount:0,nextOffset:null,messages:[]},decisions:[{changeId:'change',nodeType:'photo',kind:'change',decision:'rejected',fieldDecisions:{position:'rejected'}}],observations:[],totalDecisions:1,totalObservations:0,nextOffset:null,hasEarlier:false};
beforeEach(()=>{ipc.mockReset();});
it('修订依据校验父版、版本和稀疏属性决定，兼容没有记录的旧方案',async()=>{
  const repository=new TauriCanvasRepository();ipc.mockResolvedValue(context);
  expect(await repository.editorialRevisionContext('p','child')).toEqual(context);
  expect(ipc).toHaveBeenCalledWith('canvas_editorial_revision_context',{projectId:'p',draftId:'child',offset:0});
  ipc.mockResolvedValue(null);expect(await repository.editorialRevisionContext('p','child')).toBeNull();
});
it('拒绝串方案、串父版、串版本、未知属性和倒退页码',async()=>{
  const repository=new TauriCanvasRepository();
  for(const invalid of [{...context,draftId:'other'},{...context,parent:{...context.parent,id:'other'}},{...context,discussion:{...context.discussion,draftRevision:1}},{...context,decisions:[{...context.decisions[0],fieldDecisions:{unsafe:'rejected'}}]},{...context,nextOffset:0}]){
    ipc.mockResolvedValue(invalid);await expect(repository.editorialRevisionContext('p','child')).rejects.toThrow();
  }
});
it('即使旧观察意外返回当前预览，也不能在版本不一致时显示它',async()=>{
  ipc.mockResolvedValue({...context,observations:[{photoId:'p',photoHash:'a'.repeat(64),photoName:'frame.jpg',thumbnailPath:'/private/preview.jpg',modelId:'fixture',promptVersion:'v1',taskId:'source',callId:'inspect',visibleFacts:['fixture'],interpretation:'fixture',uncertainty:'fixture',currentHashMatches:false}]});
  const page=await new TauriCanvasRepository().editorialRevisionContext('p','child');expect(page!.observations[0].thumbnailPath).toBeNull();
});

it('关联冲突保留给界面明确处理，不信任串画布或未解决却可采纳的投影',async()=>{
  const repository=new TauriCanvasRepository();
  const selection={draftId:'d',draftRevision:2,targetBoardId:'b',expectedTargetRevision:3,selectedChangeIds:['c']};
  const conflict={changeId:'c',nodeId:'n',kind:'remove_changed',currentModified:true,relatedNodes:[{nodeId:'group',nodeType:'group',effect:'detach'}],preservedCommentCount:1,resolved:false};
  const result={simulationId:'digest',draftRevision:2,targetRevision:3,effectiveChangeIds:['c'],autoIncludedChangeIds:[],conflicts:[],attributeConflicts:[],structuralConflicts:[conflict],mergedNodes:[],canApply:false,nodeIdMap:{}};
  ipc.mockResolvedValue(result);expect((await repository.simulateMerge('p',selection)).structuralConflicts).toEqual([conflict]);
  for(const bad of [{...result,canApply:true},{...result,structuralConflicts:[{...conflict,changeId:'other'}]},{...result,structuralConflicts:[{...conflict,resolved:true}]},{...result,structuralConflicts:[{...conflict,preservedCommentCount:-1}]},{...result,targetRevision:4}]) {
    ipc.mockResolvedValue(bad);await expect(repository.simulateMerge('p',selection)).rejects.toThrow();
  }
  ipc.mockResolvedValue({...result,canApply:true,structuralConflicts:[{...conflict,resolved:true}]});
  expect((await repository.simulateMerge('p',{...selection,structuralResolutions:[{changeId:'c',choice:'remove_proposed'}]})).canApply).toBe(true);
});
