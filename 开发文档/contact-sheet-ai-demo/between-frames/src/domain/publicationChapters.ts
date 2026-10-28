import { publicationChaptersSchema,type PublicationChapter } from '@betweenframes/contracts';
import type { PublicationLayout } from './publication';
import { publicationLayers } from './publicationLayers';
import { publicationContinuousFrames } from './publicationContinuous';
export const PUBLICATION_CHAPTERS_KEY='focalroomChaptersV1';
export class PublicationChaptersError extends Error {readonly code='PUBLICATION_CHAPTERS_INVALID';constructor(){super('');}}
const invalid=():never=>{throw new PublicationChaptersError();};
export function publicationChapters(layout:PublicationLayout):PublicationChapter[]{
  let spec:Record<string,unknown>;try{spec=JSON.parse(layout.publication.specJson);}catch{return invalid();}
  if(!spec||typeof spec!=='object'||Array.isArray(spec))return invalid();
  if(!(PUBLICATION_CHAPTERS_KEY in spec))return [];
  if(layout.schemaVersion!==3)return invalid();
  const parsed=publicationChaptersSchema.safeParse(spec[PUBLICATION_CHAPTERS_KEY]);
  if(!parsed.success||new TextEncoder().encode(JSON.stringify(parsed.data)).length>32768)return invalid();
  const tree=publicationLayers(layout),objects=[...layout.pages.flatMap(p=>[p.id,...p.frames.map(f=>f.id),...p.texts.map(t=>t.id)]),...tree.nodes.filter(n=>n.kind==='group').map(n=>n.id)],ids=new Set(objects);
  if(ids.size!==objects.length)return invalid();
  const right=new Set(publicationContinuousFrames(layout).map(item=>item.right.id));
  for(const chapter of parsed.data){
    if(ids.has(chapter.id)||!layout.pages.some(p=>p.id===chapter.startPageId&&p.pageNumber>0)||right.has(chapter.startPageId))return invalid();
    ids.add(chapter.id);
  }
  return parsed.data.sort((a,b)=>layout.pages.findIndex(p=>p.id===a.startPageId)-layout.pages.findIndex(p=>p.id===b.startPageId));
}
export function publicationChapterRanges(layout:PublicationLayout){
  const chapters=publicationChapters(layout);
  return chapters.map((chapter,i)=>{
    const start=layout.pages.findIndex(p=>p.id===chapter.startPageId),end=i+1<chapters.length?layout.pages.findIndex(p=>p.id===chapters[i+1].startPageId):layout.pages.length;
    return {...chapter,pages:layout.pages.slice(start,end).filter(p=>p.pageNumber>0)};
  });
}
export function publicationChapterChanges(layout:PublicationLayout,pages:PublicationLayout['pages']){
  const ownership=(value:PublicationLayout)=>new Map(publicationChapterRanges(value).flatMap(chapter=>chapter.pages.map(p=>[p.id,chapter] as const)));
  const before=ownership(layout),after=ownership({...layout,pages});
  return pages.filter(p=>p.pageNumber>0&&before.get(p.id)?.id!==after.get(p.id)?.id).map(p=>({pageId:p.id,pageNumber:p.pageNumber,before:before.get(p.id),after:after.get(p.id)}));
}
