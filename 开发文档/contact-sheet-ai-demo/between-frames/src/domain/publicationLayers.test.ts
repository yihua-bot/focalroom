import { expect,it } from 'vitest';
import { continuousFixture } from '../test/publicationContinuousFixture';
import { publicationLayers,publicationLayerViews,publicationChangeLayers,publicationLayerPreview,publicationLayerMembers } from './publicationLayers';
import { publicationLinkPreview } from './publicationContinuous';
function fixture(){const {layout}=continuousFixture(),page=layout.pages[0];const text={id:crypto.randomUUID(),xMm:20,yMm:20,widthMm:70,heightMm:20,contentJson:'{"kind":"text","text":"原文"}',styleJson:'{"custom":"原样保留"}'};page.texts=[text];return {layout,frame:page.frames[0],text};}
it('混合照片和文字有同一叠放顺序，前后移动只生成预览且不改变原文、来源和几何',()=>{
  const {layout,frame,text}=fixture(),before=structuredClone(layout),initial=publicationLayers(layout),data=publicationChangeLayers(layout,initial,{type:'front',id:frame.id,sceneIds:[frame.id,text.id]}),preview=publicationLayerPreview(layout,data),views=publicationLayerViews(publicationLayers(preview));expect(views.find(node=>node.id===frame.id)!.rank).toBeGreaterThan(views.find(node=>node.id===text.id)!.rank);expect(preview.schemaVersion).toBe(3);expect(preview.pages).toBe(layout.pages);expect(layout).toEqual(before);
});
it('嵌套分组继承显示和锁定，解除分组仍保留外观、锁定与完整对象身份',()=>{
  const {layout,frame,text}=fixture(),group=crypto.randomUUID();let data=publicationChangeLayers(layout,publicationLayers(layout),{type:'group',id:group,ids:[frame.id,text.id],name:'封面'});
  data=publicationChangeLayers(layout,data,{type:'visible',id:group});data=publicationChangeLayers(layout,data,{type:'locked',id:group});const members=publicationLayerMembers(data,group);expect(members.map(node=>node.id).sort()).toEqual([frame.id,text.id].sort());expect(publicationLayerViews(data).filter(node=>members.some(member=>member.id===node.id)).every(node=>!node.effectiveVisible&&node.effectiveLocked)).toBe(true);
  data=publicationChangeLayers(layout,data,{type:'ungroup',id:group});expect(data.nodes.some(node=>node.kind==='group')).toBe(false);expect(data.nodes.filter(node=>node.id===frame.id||node.id===text.id).every(node=>!node.visible&&node.locked&&node.parentId===null)).toBe(true);
});
it('缺失、重复、跨页、循环、伪造来源和旧格式不能成为有效图层版面',()=>{
  const {layout,frame,text}=fixture(),base=publicationLayers(layout),group=crypto.randomUUID();for(const mode of ['missing','duplicate','foreign','source','cycle','page']as const){const data=structuredClone(base);if(mode==='missing')data.nodes.pop();if(mode==='duplicate')data.nodes.push(data.nodes[0]);if(mode==='foreign')data.nodes[0].id=crypto.randomUUID();if(mode==='source')(data.nodes[0] as unknown as Record<string,unknown>).assetId='foreign';if(mode==='cycle'||mode==='page')data.nodes.push({id:mode==='page'?layout.pages[0].id:group,kind:'group',parentId:mode==='cycle'?group:null,name:null,visible:true,locked:false});expect(()=>publicationLayerPreview(layout,data)).toThrow();}
  expect(()=>publicationChangeLayers(layout,base,{type:'group',id:group,ids:[frame.id,layout.pages[1].frames[0].id],name:'错误跨页'})).toThrow();const valid=publicationLayerPreview(layout,publicationChangeLayers(layout,base,{type:'group',id:group,ids:[frame.id,text.id],name:'封面'}));expect(()=>publicationLayers({...valid,schemaVersion:1})).toThrow();expect(()=>publicationLayers({...layout,schemaVersion:3})).toThrow();
});
it('连续照片只有一个图层，跨页前后保留配置；不能偷偷把一个分组拆成两个页面',()=>{
  const {layout,data}=continuousFixture();const layers=publicationLayers(layout),withLayers=publicationLayerPreview(layout,layers),linked=publicationLinkPreview(withLayers,data);expect(linked.schemaVersion).toBe(3);expect(publicationLayers(linked).nodes.filter(node=>node.id===data.frameId)).toHaveLength(1);
  const other=linked.pages[2].frames.find(frame=>frame.id!==data.frameId)!;expect(()=>publicationChangeLayers(linked,publicationLayers(linked),{type:'group',id:crypto.randomUUID(),ids:[data.frameId,other.id],name:'错误分组'})).toThrow();
});
