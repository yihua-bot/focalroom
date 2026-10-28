import { expect,it } from 'vitest';
import { publicationActionSchema,parsePublicationReceipt } from '@betweenframes/contracts';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { publicationLinkPreview } from '../../domain/publicationContinuous';
import { publicationLayerPreview,publicationLayers } from '../../domain/publicationLayers';
import { publicationChapters } from '../../domain/publicationChapters';
import { publicationChaptersPreview } from './publicationChapterPlans';
import { publicationChapterRemovalIds,publicationPageRemoval } from './publicationPageRemoval';
function fixture(){
 const {layout}=continuousFixture();layout.pages.push(...[4,5,6].map(n=>({...layout.pages[0],id:crypto.randomUUID(),pageNumber:n,side:n%2?'right':'left',frames:[],texts:[]})));
 return layout;
}
it('整章页面移除的纯预览清理章节与图层，不变更原始布局或幸存来源',()=>{
 const layout=fixture(),defs=[{id:crypto.randomUUID(),title:'夜行',startPageId:layout.pages[1].id},{id:crypto.randomUUID(),title:'归途',startPageId:layout.pages[3].id}];
 const before=publicationChaptersPreview(layout,{chapters:defs,pageIds:layout.pages.map(p=>p.id)}),copy=structuredClone(before),ids=publicationChapterRemovalIds(before,defs[0].id);
 const plan=publicationPageRemoval(before,{pageIds:ids,chapterId:defs[0].id,retainBlankPageId:null});
 expect(plan.removedPageIds).toEqual(ids);expect(publicationChapters(plan.preview)).toEqual([defs[1]]);expect(plan.preview.pages).toHaveLength(before.pages.length-2);expect(before).toEqual(copy);
 for(const p of plan.preview.pages)expect(p.frames).toEqual(before.pages.find(old=>old.id===p.id)!.frames);
 expect(publicationLayers(plan.preview).nodes.every(n=>!before.pages.slice(1,3).some(p=>p.frames.some(f=>f.id===n.id)))).toBe(true);
});
it('奇数章节的后续跨页需要显式空白：保留尺寸并清空全部内容与设置',()=>{
 const layout=fixture(),frame=layout.pages[3].frames[0];layout.pages[3].frames=[];layout.pages[4].frames=[frame];
 const pair={frameId:frame.id,leftPageId:layout.pages[4].id,rightPageId:layout.pages[5].id,geometry:{xMm:40,yMm:20,widthMm:250,heightMm:100,fitMode:'fill' as const,cropX:.2,cropY:.7,cropZoom:1.4}};
 const spread=publicationLinkPreview(layout,pair),defs=[{id:crypto.randomUUID(),title:'夜行',startPageId:layout.pages[1].id},{id:crypto.randomUUID(),title:'归途',startPageId:layout.pages[4].id}];
 const before=publicationChaptersPreview(spread,{chapters:defs,pageIds:spread.pages.map(p=>p.id)}),ids=publicationChapterRemovalIds(before,defs[0].id);
 expect(()=>publicationPageRemoval(before,{pageIds:ids,chapterId:defs[0].id,retainBlankPageId:null})).toThrowError(expect.objectContaining({reason:'parity'}));
 const plan=publicationPageRemoval(before,{pageIds:ids,chapterId:defs[0].id,retainBlankPageId:ids[0]}),blank=plan.preview.pages.find(p=>p.id===ids[0])!;
 expect(blank).toMatchObject({widthMm:190,heightMm:257,frames:[],texts:[],layoutJson:null});expect(plan.focusId).toBe(ids[0]);
 expect(plan.preview.pages.find(p=>p.id===pair.leftPageId)!.pageNumber).toBe(2);expect(plan.preview.pages.find(p=>p.id===pair.leftPageId)!.frames).toEqual(before.pages[4].frames);
});
it('整组移除连续照片的两页，不能单独切半或误删其他章节起始页',()=>{
 const f=continuousFixture(),before=publicationLinkPreview(f.layout,f.data),ids=[f.data.leftPageId,f.data.rightPageId];
 expect(()=>publicationPageRemoval(before,{pageIds:[ids[0]],chapterId:null,retainBlankPageId:null})).toThrowError(expect.objectContaining({reason:'boundary'}));
 const plan=publicationPageRemoval(before,{pageIds:ids,chapterId:null,retainBlankPageId:null});expect(plan.preview.schemaVersion).toBe(1);expect(plan.preview.pages.flatMap(p=>p.frames).some(frame=>frame.id===f.data.frameId)).toBe(false);
 const defs=[{id:crypto.randomUUID(),title:'一章',startPageId:ids[0]}],withChapter=publicationChaptersPreview(before,{chapters:defs,pageIds:before.pages.map(p=>p.id)});
 expect(()=>publicationPageRemoval(withChapter,{pageIds:ids,chapterId:null,retainBlankPageId:null})).toThrowError(expect.objectContaining({reason:'chapter'}));
});
it('隐藏的继承锁定阻止全部移除，解锁后只清理被移除的组及叶对象',()=>{
 const layout=fixture(),data=publicationLayers(layout),frame=layout.pages[1].frames[0],group=crypto.randomUUID();
 data.nodes.find(n=>n.id===frame.id)!.parentId=group;data.nodes.push({id:group,kind:'group',parentId:null,name:'隐藏组',visible:false,locked:true});
 const locked=publicationLayerPreview(layout,data),scope={pageIds:[layout.pages[1].id,layout.pages[2].id],chapterId:null,retainBlankPageId:null};
 expect(()=>publicationPageRemoval(locked,scope)).toThrowError(expect.objectContaining({reason:'locked'}));
 data.nodes.find(n=>n.id===group)!.locked=false;const unlocked=publicationLayerPreview(layout,data),plan=publicationPageRemoval(unlocked,scope);
 expect(publicationLayers(plan.preview).nodes.some(n=>n.id===group||n.id===frame.id)).toBe(false);
});
it('不连续范围、封面、错保留页和空白无动作被拒绝；新命令必须明确选择范围',()=>{
 const layout=fixture(),pageId=layout.pages[1].id,base={pageIds:[pageId],retainBlankPageId:null,chapterId:null};
 for(const data of [{...base,pageIds:[]},{...base,pageIds:[pageId,pageId]},{...base,pageIds:[pageId,layout.pages[3].id]},{...base,pageIds:[layout.pages[0].id]},{...base,retainBlankPageId:layout.pages[2].id}]){
  expect(()=>publicationPageRemoval(layout,data)).toThrow();
 }
 expect(publicationActionSchema.safeParse({type:'RemovePages',data:{pageIds:[pageId],chapterId:null}}).success).toBe(false);
 const first=publicationPageRemoval(layout,{...base,retainBlankPageId:pageId});
 expect(()=>publicationPageRemoval(first.preview,{...base,retainBlankPageId:pageId})).toThrowError(expect.objectContaining({reason:'noop'}));
 const id=crypto.randomUUID(),request={schemaVersion:3 as const,commandId:id,idempotencyKey:id,projectId:'p',publicationId:crypto.randomUUID(),baseRevision:2,operations:[{opId:crypto.randomUUID(),action:{type:'RemovePages' as const,data:base}}]};
 expect(()=>parsePublicationReceipt(request,{commandId:id,projectId:'p',publicationId:request.publicationId,baseRevision:2,revision:2,changed:false,results:[null]})).toThrow();
});
