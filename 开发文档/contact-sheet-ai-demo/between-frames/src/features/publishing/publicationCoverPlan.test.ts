import { userFacingError } from '../../services/userFacingError';
import { expect,it } from 'vitest';
import { setPublicationCoverSchema,type SetPublicationCover } from '@betweenframes/contracts';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { publicationCoverPlan } from './publicationCoverPlan';
import { checkPublicationCover,publicationPageCover } from '../../domain/publicationCover';
import { publicationLayers,publicationLayerPreview } from '../../domain/publicationLayers';
function fixture(){const {layout}=continuousFixture();const data:SetPublicationCover={pageId:layout.pages[0].id,backPageId:null,settings:{panelWidthMm:190,spineWidthMm:10},heightMm:257,marginMm:15,bleedMm:3,contentPlacement:'followPanels',clearPageSettingsConfirmed:false};return {layout,data};}
it('普通封面转真实展开页只移动现有对象，完整来源与所有内页保持，原版面不变',()=>{
 const {layout,data}=fixture(),before=structuredClone(layout),plan=publicationCoverPlan(layout,data);
 expect(plan.page.widthMm).toBe(390);expect(plan.page.frames[0]).toEqual({...before.pages[0].frames[0],xMm:215});
 expect(plan.preview.pages.slice(1)).toEqual(before.pages.slice(1));expect(plan.preview.schemaVersion).toBe(3);
 expect(publicationPageCover(plan.page)).toEqual(data.settings);expect(layout).toEqual(before);
 expect(()=>publicationCoverPlan(plan.preview,data)).toThrow();
 const native=structuredClone(plan.preview);native.pages[0].layoutJson=' {"focalroomCoverV1":{"spineWidthMm":10.0,"panelWidthMm":190.0}} ';
 expect(()=>publicationCoverPlan(native,data)).toThrow();
});
it('独立封底合入同一展开页，图层分组、文字与来源沿用原编号',()=>{
 const {layout,data}=fixture(),back=structuredClone(layout.pages.pop()!);back.pageNumber=-1;back.side='back';layout.pages.push(back);data.backPageId=back.id;
 const tree=publicationLayers(layout),group=crypto.randomUUID();tree.nodes.find(n=>n.id===back.frames[0].id)!.parentId=group;tree.nodes.push({id:group,kind:'group',parentId:null,name:'封底',visible:true,locked:false});
 const before=publicationLayerPreview(layout,tree),plan=publicationCoverPlan(before,data);
 expect(plan.preview.pages.map(p=>p.id)).toEqual(before.pages.filter(p=>p.id!==back.id).map(p=>p.id));
 expect(plan.page.frames.find(f=>f.id===back.frames[0].id)).toEqual(back.frames[0]);expect(publicationLayers(plan.preview)).toEqual(tree);
 expect(()=>publicationCoverPlan(before,{...data,backPageId:null})).toThrow();
});
it('尺寸变化可明确跟随区域或保留位置，跨分界对象不自动移动或裁切',()=>{
 const {layout,data}=fixture(),initial=publicationCoverPlan(layout,data).preview;
 const followed=publicationCoverPlan(initial,{...data,settings:{panelWidthMm:200,spineWidthMm:20}});
 expect(followed.page.frames[0].xMm).toBe(235);
 const kept=publicationCoverPlan(initial,{...data,settings:{panelWidthMm:200,spineWidthMm:20},contentPlacement:'keepPosition'});
 expect(kept.page.frames[0]).toEqual(initial.pages[0].frames[0]);
 const across=structuredClone(initial);across.pages[0].frames[0].xMm=10;across.pages[0].frames[0].widthMm=350;
 expect(()=>publicationCoverPlan(across,{...data,settings:{panelWidthMm:200,spineWidthMm:20}})).toThrow();
 expect(publicationCoverPlan(across,{...data,settings:{panelWidthMm:200,spineWidthMm:20},contentPlacement:'keepPosition'}).page.frames[0]).toEqual(across.pages[0].frames[0]);
 expect(()=>publicationCoverPlan(initial,{...data,heightMm:100})).toThrow();
});
it('替换旧页设置须明确确认；隐藏继承锁和损坏展开数据不能绕过',()=>{
 const {layout,data}=fixture();layout.pages[0].layoutJson='{"legacy":{"keep":true}}';
 expect(()=>publicationCoverPlan(layout,data)).toThrow();
 const converted=publicationCoverPlan(layout,{...data,clearPageSettingsConfirmed:true});
 expect(JSON.parse(converted.page.layoutJson!)).toEqual({focalroomCoverV1:data.settings});
 const tree=publicationLayers(converted.preview);tree.nodes.find(n=>n.id===converted.page.frames[0].id)!.locked=true;tree.nodes.find(n=>n.id===converted.page.frames[0].id)!.visible=false;
 expect(()=>publicationCoverPlan(publicationLayerPreview(converted.preview,tree),{...data,settings:{panelWidthMm:200,spineWidthMm:10}})).toThrow();
 const corrupt=structuredClone(converted.preview);corrupt.pages[0].widthMm+=1;expect(()=>checkPublicationCover(corrupt)).toThrow();let error:unknown;try{checkPublicationCover(corrupt);}catch(value){error=value;}expect(userFacingError(error,'无法读取这份版面。')).toBe('无法读取这份版面。');
});
it('空书可建空白展开封面，尺寸和明确许可字段严格校验，旧普通封面不回写',()=>{
 const {layout,data}=fixture();expect(publicationPageCover(layout.pages[0])).toBeUndefined();
 const empty={...layout,pages:[]};expect(publicationCoverPlan(empty,data).page.frames).toEqual([]);
 expect(()=>publicationCoverPlan(layout,{...data,contentPlacement:'keepPosition'})).toThrow();
 expect(setPublicationCoverSchema.safeParse({...data,settings:{panelWidthMm:1000,spineWidthMm:1}}).success).toBe(false);
 const missing={...data} as Partial<SetPublicationCover>;delete missing.backPageId;expect(setPublicationCoverSchema.safeParse(missing).success).toBe(false);
 delete missing.clearPageSettingsConfirmed;expect(setPublicationCoverSchema.safeParse(missing).success).toBe(false);
 expect(()=>publicationCoverPlan(layout,{...data,pageId:layout.pages[1].id})).toThrow();
});
