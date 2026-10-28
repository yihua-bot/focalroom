import type { PublicationLayout,PublicationObjectReference } from '../../domain/publication';
import { publicationLayers,publicationLayerViews,publicationLayerMembers,publicationLayerObjects,publicationLayerPreview,PublicationLayersError } from '../../domain/publicationLayers';

// 先冻结本次组成员和移除后的版面；保存仍走原有的一次批量命令。
export function publicationGroupRemoval(layout:PublicationLayout,groupId:string){
  const data=publicationLayers(layout),views=publicationLayerViews(data),group=views.find(node=>node.id===groupId);
  if(!group||group.kind!=='group')throw new PublicationLayersError();
  const members=publicationLayerMembers(data,groupId,views);
  if(!members.length||members.length>100||group.effectiveLocked||members.some(member=>views.find(node=>node.id===member.id)?.effectiveLocked))throw new PublicationLayersError();
  const references=publicationLayerObjects(layout),pageId=references.get(members[0].id)?.pageId;
  if(!pageId||members.some(member=>references.get(member.id)?.pageId!==pageId))throw new PublicationLayersError();
  const ids=new Set(members.map(node=>node.id)),objects=members.map(node=>({kind:node.kind as 'frame'|'text',id:node.id})) satisfies PublicationObjectReference[];
  const nodes=data.nodes.filter(node=>node.kind==='group'?publicationLayerMembers(data,node.id,views).some(member=>!ids.has(member.id)):!ids.has(node.id));
  const preview=publicationLayerPreview({...layout,pages:layout.pages.map(page=>page.id===pageId?{...page,frames:page.frames.filter(frame=>!ids.has(frame.id)),texts:page.texts.filter(text=>!ids.has(text.id))}:page)},{nodes});
  return {groupId,name:group.name,pageId,objects,preview};
}
export type PublicationGroupRemoval=ReturnType<typeof publicationGroupRemoval>;
