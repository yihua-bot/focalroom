import { publicationActionSchema,type RemovePublicationPages } from '@betweenframes/contracts';
import type { PublicationLayout } from '../../domain/publication';
import { publicationChapterRanges,publicationChapters,PUBLICATION_CHAPTERS_KEY } from '../../domain/publicationChapters';
import { PUBLICATION_LAYERS_KEY,publicationLayers,publicationLayerViews,publicationLayerMembers,publicationLayerPreview } from '../../domain/publicationLayers';
import { publicationContinuousFrames } from '../../domain/publicationContinuous';
import { publicationPageOrderPreview } from './publicationPageOrder';
export class PublicationPageRemovalError extends Error{
  readonly code='PUBLICATION_PAGE_REMOVAL_INVALID';
  constructor(readonly reason:'scope'|'boundary'|'chapter'|'locked'|'parity'|'noop'){super('');}
}
const fail=(reason:PublicationPageRemovalError['reason']):never=>{throw new PublicationPageRemovalError(reason);};
/** 冻结页面范围与完整移除预览，保留空白页必须是明确选择，不自动改动后续配对。 */
export function publicationPageRemoval(layout:PublicationLayout,data:RemovePublicationPages){
  if(!publicationActionSchema.safeParse({type:'RemovePages',data}).success)return fail('scope');
  const ids=new Set(data.pageIds),pages=layout.pages.filter(p=>ids.has(p.id)),first=layout.pages.findIndex(p=>p.id===data.pageIds[0]);
  if(pages.length!==ids.size||pages.some(p=>p.pageNumber<=0)||pages.some((p,i)=>p.id!==data.pageIds[i])||layout.pages.slice(first,first+pages.length).some((p,i)=>p.id!==data.pageIds[i])||data.retainBlankPageId!==null&&data.retainBlankPageId!==data.pageIds[0])return fail('scope');
  const chapters=publicationChapters(layout),chapter=data.chapterId===null?undefined:publicationChapterRanges(layout).find(c=>c.id===data.chapterId);
  if(data.chapterId!==null&&(!chapter||chapter.pages.length!==data.pageIds.length||chapter.pages.some((p,i)=>p.id!==data.pageIds[i])))return fail('chapter');
  for(const pair of publicationContinuousFrames(layout))if(ids.has(pair.left.id)!==ids.has(pair.right.id))return fail('boundary');
  if(chapters.some(c=>ids.has(c.startPageId)&&c.id!==data.chapterId&&c.startPageId!==data.retainBlankPageId))return fail('chapter');
  const layers=publicationLayers(layout),views=publicationLayerViews(layers),objects=pages.flatMap(p=>[...p.frames.map(f=>f.id),...p.texts.map(t=>t.id)]);
  if(objects.some(id=>views.find(n=>n.id===id)?.effectiveLocked))return fail('locked');
  if(data.chapterId===null&&pages.length===1&&data.retainBlankPageId===pages[0].id&&!pages[0].frames.length&&!pages[0].texts.length&&pages[0].layoutJson===null)return fail('noop');
  let next:PublicationLayout={...layout,pages:layout.pages.filter(p=>!ids.has(p.id)||p.id===data.retainBlankPageId).map(p=>p.id===data.retainBlankPageId?{...p,frames:[],texts:[],layoutJson:null}:p)};
  if(chapter)next={...next,publication:{...next.publication,specJson:JSON.stringify({...JSON.parse(next.publication.specJson),[PUBLICATION_CHAPTERS_KEY]:chapters.filter(c=>c.id!==chapter.id)})}};
  const ordered=publicationPageOrderPreview(next.pages,next.pages.map(p=>p.id));if(!ordered)return fail('parity');next={...next,pages:ordered};
  if(Object.prototype.hasOwnProperty.call(JSON.parse(layout.publication.specJson),PUBLICATION_LAYERS_KEY)){
    const removed=new Set(objects),nodes=layers.nodes.filter(n=>n.kind==='group'?publicationLayerMembers(layers,n.id,views).some(member=>!removed.has(member.id)):!removed.has(n.id));
    next=publicationLayerPreview(next,{nodes});
  }else{
    next={...next,schemaVersion:publicationContinuousFrames({...next,schemaVersion:2}).length?2:1};
  }
  publicationChapters(next);publicationLayers(next);
  if(JSON.stringify(next)===JSON.stringify(layout))return fail('noop');
  const removedPageIds=data.pageIds.filter(id=>id!==data.retainBlankPageId),focusId=data.retainBlankPageId??next.pages[Math.min(first,next.pages.length-1)]?.id;
  return {data,chapter,pages,preview:next,removedPageIds,focusId,frameCount:pages.reduce((n,p)=>n+p.frames.length,0),textCount:pages.reduce((n,p)=>n+p.texts.length,0)};
}
export type PublicationPageRemovalPlan=ReturnType<typeof publicationPageRemoval>;
export function publicationChapterRemovalIds(layout:PublicationLayout,chapterId:string):string[]{
  const chapter=publicationChapterRanges(layout).find(c=>c.id===chapterId);if(!chapter)return fail('chapter');return chapter.pages.map(p=>p.id);
}
