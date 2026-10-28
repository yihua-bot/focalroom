import { publicationLayersSchema,PUBLICATION_LAYERS_KEY,type PublicationLayerNode,type PublicationLayers } from '@betweenframes/contracts';
import type { PublicationLayout } from './publication';
import { publicationContinuousFrames } from './publicationContinuous';
export { PUBLICATION_LAYERS_KEY };
export type { PublicationLayerNode,PublicationLayers };
export class PublicationLayersError extends Error {readonly code='PUBLICATION_LAYERS_INVALID';constructor(){super('');}}
const invalid=():never=>{throw new PublicationLayersError();};
const defaultNode=(id:string,kind:PublicationLayerNode['kind']):PublicationLayerNode=>({id,kind,parentId:null,name:null,visible:true,locked:false});
export function publicationLayerObjects(layout:PublicationLayout){return new Map<string,{kind:'frame'|'text';pageId:string}>(layout.pages.flatMap(page=>[...page.frames.map(frame=>[frame.id,{kind:'frame' as const,pageId:page.id}] as const),...page.texts.map(text=>[text.id,{kind:'text' as const,pageId:page.id}] as const)]));}
export function publicationLayers(layout:PublicationLayout):PublicationLayers{
  if(layout.publication.specJson.length>1024*1024)return invalid();
  let spec:Record<string,unknown>;try{spec=JSON.parse(layout.publication.specJson);}catch{return invalid();}
  if(!spec||typeof spec!=='object'||Array.isArray(spec))return invalid();
  const stored=spec[PUBLICATION_LAYERS_KEY],bytes=new TextEncoder().encode(layout.publication.specJson).length;
  if(bytes>(stored===undefined?32768:1024*1024))return invalid();
  if(stored===undefined){
    if(layout.schemaVersion===3)return invalid();
    const continuous=new Set(publicationContinuousFrames(layout).map(value=>value.frame.id));
    return {nodes:[...layout.pages.flatMap(page=>page.frames.filter(frame=>continuous.has(frame.id)).map(frame=>defaultNode(frame.id,'frame'))),...layout.pages.flatMap(page=>page.frames.filter(frame=>!continuous.has(frame.id)).map(frame=>defaultNode(frame.id,'frame'))),...layout.pages.flatMap(page=>page.texts.map(text=>defaultNode(text.id,'text')))]};
  }
  if(layout.schemaVersion!==3)return invalid();const parsed=publicationLayersSchema.safeParse(stored);if(!parsed.success)return invalid();publicationValidateLayers(layout,parsed.data);return parsed.data;
}
export function publicationValidateLayers(layout:PublicationLayout,data:PublicationLayers){
  const nodes=data.nodes,byId=new Map(nodes.map(node=>[node.id,node])),objects=publicationLayerObjects(layout),continuous=new Set(publicationContinuousFrames(layout).map(value=>value.frame.id)),groups=new Map<string,Set<string>>();
  if(byId.size!==nodes.length||nodes.length>4608||nodes.filter(node=>node.kind==='group').length>512)return invalid();
  const pages=new Set(layout.pages.map(page=>page.id));let leaves=0;
  for(const node of nodes){
    if(node.kind==='group'){if(objects.has(node.id)||pages.has(node.id))return invalid();}
    else {if(objects.get(node.id)?.kind!==node.kind)return invalid();leaves++;}
    let parent=node.parentId;const seen=new Set([node.id]);while(parent){
      if(seen.has(parent)||seen.size>=9)return invalid();seen.add(parent);const group=byId.get(parent);if(!group||group.kind!=='group')return invalid();
      if(node.kind!=='group'){if(continuous.has(node.id))return invalid();const groupPages=groups.get(group.id)??new Set<string>();groupPages.add(objects.get(node.id)!.pageId);groups.set(group.id,groupPages);}parent=group.parentId;
    }
  }
  if(leaves!==objects.size||nodes.some(node=>node.kind==='group'&&groups.get(node.id)?.size!==1))return invalid();
}
export interface PublicationLayerView extends PublicationLayerNode {effectiveVisible:boolean;effectiveLocked:boolean;rank:number;depth:number;rootGroup?:string}
export function publicationLayerViews(layers:PublicationLayers):PublicationLayerView[]{
  const children=new Map<string|null,PublicationLayerNode[]>();for(const node of layers.nodes){const list=children.get(node.parentId)??[];list.push(node);children.set(node.parentId,list);}
  const result:PublicationLayerView[]=[];let rank=0;const visit=(parent:string|null,visible:boolean,locked:boolean,depth:number,root?:string)=>{if(depth>8)return invalid();for(const node of children.get(parent)??[]){const effectiveVisible=visible&&node.visible,effectiveLocked=locked||node.locked,rootGroup=root??(node.kind==='group'?node.id:undefined);result.push({...node,effectiveVisible,effectiveLocked,depth,rank:rank++,rootGroup});if(node.kind==='group')visit(node.id,effectiveVisible,effectiveLocked,depth+1,rootGroup);}};visit(null,true,false,0);if(result.length!==layers.nodes.length)return invalid();return result;
}
export function publicationLayerMembers(layers:PublicationLayers,id:string,views=publicationLayerViews(layers)):PublicationLayerNode[]{
  const group=views.find(node=>node.id===id);if(!group)return [];if(group.kind!=='group')return [group];const index=views.indexOf(group);const result:PublicationLayerNode[]=[];for(const node of views.slice(index+1)){if(node.depth<=group.depth)break;if(node.kind!=='group')result.push(node);}return result;
}
export function publicationLayerPreview(layout:PublicationLayout,layers:PublicationLayers):PublicationLayout{
  const checked=publicationLayersSchema.parse(layers);publicationValidateLayers(layout,checked);const spec=JSON.parse(layout.publication.specJson);spec[PUBLICATION_LAYERS_KEY]=checked;const specJson=JSON.stringify(spec);if(new TextEncoder().encode(specJson).length>1024*1024)return invalid();return {...layout,schemaVersion:3,publication:{...layout.publication,specJson}};
}
export type LayerAction={type:'visible'|'locked';id:string}|{type:'name';id:string;name:string}|{type:'front'|'back'|'forward'|'backward';id:string;sceneIds:string[]}|{type:'group';ids:string[];id:string;name:string}|{type:'ungroup';id:string};
export function publicationChangeLayers(layout:PublicationLayout,layers:PublicationLayers,action:LayerAction):PublicationLayers{
  let nodes=layers.nodes.map(node=>({...node}));const node=nodes.find(node=>node.id===action.id);if(action.type==='group'){
    const selection=new Set(action.ids),selected=nodes.filter(node=>selection.has(node.id));if(selected.length<2||selected.length!==selection.size||selected.some(value=>value.parentId!==selected[0].parentId)||nodes.some(value=>value.id===action.id))return invalid();
    const at=Math.max(...selected.map(item=>nodes.indexOf(item))),group={...defaultNode(action.id,'group'),parentId:selected[0].parentId,name:action.name};nodes.splice(at+1,0,group);for(const child of selected)child.parentId=group.id;
  }else{
    if(!node)return invalid();
    if(action.type==='visible')node.visible=!node.visible;
    else if(action.type==='locked')node.locked=!node.locked;
    else if(action.type==='name')node.name=action.name.trim();
    else if(action.type==='ungroup'){
      if(node.kind!=='group')return invalid();for(const child of nodes){if(child.parentId===node.id){child.parentId=node.parentId;child.visible=child.visible&&node.visible;child.locked=child.locked||node.locked;}}
      // 同父节点的相对位置在原分组所在的位置展开。
      const children=nodes.filter(value=>value.parentId===node.parentId&&layers.nodes.find(old=>old.id===value.id)?.parentId===node.id);
      const others=nodes.filter(value=>value.id!==node.id&&!children.some(child=>child.id===value.id)),at=nodes.filter(value=>value.id!==node.id&&!children.some(child=>child.id===value.id)).filter(value=>nodes.indexOf(value)<nodes.indexOf(node)).length;others.splice(at,0,...children);nodes=others;
    }else if('sceneIds' in action){
      const scene=new Set(action.sceneIds),views=publicationLayerViews(layers),touches=(id:string)=>publicationLayerMembers(layers,id,views).some(member=>scene.has(member.id));
      const siblings=nodes.filter(value=>value.parentId===node.parentId&&(value.id===node.id||touches(value.id))),from=siblings.indexOf(node);let to=action.type==='front'?siblings.length-1:action.type==='back'?0:from+(action.type==='forward'?1:-1);to=Math.max(0,Math.min(siblings.length-1,to));if(from!==to){const target=siblings[to],old=nodes.indexOf(node),dest=nodes.indexOf(target);nodes.splice(old,1);nodes.splice(dest,0,node);}
    }
  }
  const result={nodes};publicationLayerPreview(layout,result);return result;
}
