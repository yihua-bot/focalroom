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
