import { expect,it } from 'vitest';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { publicationLayers,publicationChangeLayers,publicationLayerPreview } from '../../domain/publicationLayers';
import { publicationGroupRemoval } from './publicationRemoval';
function fixture(){
  let {layout}=continuousFixture();const page=layout.pages[0],text={id:crypto.randomUUID(),xMm:20,yMm:20,widthMm:80,heightMm:20,contentJson:'{"kind":"text","text":"保留正文"}',styleJson:null};page.texts.push(text);
  const groupId=crypto.randomUUID(),outerId=crypto.randomUUID();let tree=publicationChangeLayers(layout,publicationLayers(layout),{type:'group',ids:[page.frames[0].id,text.id],id:groupId,name:'内组'});
  const peer={...text,id:crypto.randomUUID()};page.texts.push(peer);
  tree.nodes.push({id:peer.id,kind:'text',parentId:null,name:null,visible:true,locked:false});
  tree=publicationChangeLayers(layout,tree,{type:'group',ids:[groupId,peer.id],id:outerId,name:'外组'});layout=publicationLayerPreview(layout,tree);
  return {layout,groupId,outerId,text,peer};
}
it('组内照片与文字先做不可变对比，移除嵌套组保留兄弟对象和祖先，移除外组清除空组',()=>{
  const {layout,groupId,outerId,peer}=fixture(),before=structuredClone(layout),plan=publicationGroupRemoval(layout,groupId);
  expect(plan.objects.map(object=>object.kind).sort()).toEqual(['frame','text']);expect(plan.preview.pages[0].frames).toEqual([]);expect(plan.preview.pages[0].texts).toEqual([peer]);
  expect(publicationLayers(plan.preview).nodes.find(node=>node.id===outerId)).toBeDefined();expect(publicationLayers(plan.preview).nodes.some(node=>node.id===groupId)).toBe(false);expect(plan.preview.pages.slice(1)).toEqual(layout.pages.slice(1));expect(layout).toEqual(before);
  const outer=publicationGroupRemoval(layout,outerId);expect(outer.objects).toHaveLength(3);expect(publicationLayers(outer.preview).nodes.some(node=>node.kind==='group')).toBe(false);expect(layout).toEqual(before);
});
it('隐藏组仍可明确移除；继承锁定、非组、无效引用和超过100项不产生计划',()=>{
  const {layout,groupId,outerId}=fixture(),tree=publicationLayers(layout),hidden=publicationLayerPreview(layout,publicationChangeLayers(layout,tree,{type:'visible',id:outerId}));expect(publicationGroupRemoval(hidden,groupId).objects).toHaveLength(2);
  const locked=publicationLayerPreview(layout,publicationChangeLayers(layout,tree,{type:'locked',id:outerId}));expect(()=>publicationGroupRemoval(locked,groupId)).toThrow();expect(()=>publicationGroupRemoval(layout,layout.pages[0].frames[0].id)).toThrow();expect(()=>publicationGroupRemoval(layout,crypto.randomUUID())).toThrow();
  const many=structuredClone(layout),nodes=publicationLayers(many);for(let i=0;i<100;i++){const text={...many.pages[0].texts[0],id:crypto.randomUUID()};many.pages[0].texts.push(text);nodes.nodes.push({id:text.id,kind:'text',parentId:groupId,name:null,visible:true,locked:false});}expect(()=>publicationGroupRemoval(publicationLayerPreview(many,nodes),groupId)).toThrow();
});
