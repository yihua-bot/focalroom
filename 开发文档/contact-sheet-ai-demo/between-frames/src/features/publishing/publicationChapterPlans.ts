import { publicationTextStyle } from './publicationText';
import type { CreatePublicationChapter,SetPublicationChapters,PublicationChapter } from '@betweenframes/contracts';
import type { PublicationLayout } from '../../domain/publication';
import { PUBLICATION_CHAPTERS_KEY,PublicationChaptersError,publicationChapters,publicationChapterRanges } from '../../domain/publicationChapters';
import { publicationLayers,publicationLayerPreview } from '../../domain/publicationLayers';
import { publicationPageOrderPreview } from './publicationPageOrder';
const invalid=():never=>{throw new PublicationChaptersError();};
function put(layout:PublicationLayout,chapters:PublicationChapter[]):PublicationLayout {
  const next={...layout,publication:{...layout.publication,specJson:JSON.stringify({...JSON.parse(layout.publication.specJson),[PUBLICATION_CHAPTERS_KEY]:chapters})}};
  publicationChapters(next);publicationLayers(next);return next;
}
export function publicationChaptersPreview(layout:PublicationLayout,data:SetPublicationChapters):PublicationLayout {
  const pages=publicationPageOrderPreview(layout.pages,data.pageIds);if(!pages)return invalid();
  return put(publicationLayerPreview({...layout,pages},publicationLayers(layout)),data.chapters);
}
export function publicationMoveChapter(layout:PublicationLayout,id:string,direction:-1|1):string[]|undefined{
  const chapters=publicationChapterRanges(layout),index=chapters.findIndex(c=>c.id===id),target=index+direction;
  if(index<0||target<0||target>=chapters.length)return;
  const ordered=chapters.slice();[ordered[index],ordered[target]]=[ordered[target],ordered[index]];
  return publicationReorderChapters(layout,ordered.map(c=>c.id));
}
export function publicationReorderChapters(layout:PublicationLayout,chapterIds:string[]):string[]|undefined{
  const chapters=publicationChapterRanges(layout);
  if(!chapters.length||chapterIds.length!==chapters.length||new Set(chapterIds).size!==chapterIds.length||chapterIds.some(id=>!chapters.some(c=>c.id===id)))return;
  const ordered=chapterIds.map(id=>chapters.find(c=>c.id===id)!);
  const first=layout.pages.findIndex(p=>p.id===chapters[0].startPageId),ids=[...layout.pages.slice(0,first).map(p=>p.id),...ordered.flatMap(c=>c.pages.map(p=>p.id)),...layout.pages.filter(p=>p.pageNumber===-1).map(p=>p.id)];
  try{publicationChaptersPreview(layout,{chapters:chapters.map(({id,title,startPageId})=>({id,title,startPageId})),pageIds:ids});return ids;}catch{return;}
}
export function publicationChapterOpeningPreview(layout:PublicationLayout,data:CreatePublicationChapter):PublicationLayout{
  if(!layout.pages.length||layout.pages.length+data.pages.length>512||data.pages.length<1||data.pages.length>2||data.title.kind!=='text'||data.title.text!==data.chapter.title||data.title.pageId!==data.chapter.startPageId||!data.pages.some(p=>p.pageId===data.chapter.startPageId)||data.pages.some(p=>p.pageId===data.title.textId)||data.pages[1]&&data.pages[1].afterPageId!==data.pages[0].pageId)return invalid();
  let pages=layout.pages.slice();
  for(const add of data.pages){
    const at=add.afterPageId?pages.findIndex(p=>p.id===add.afterPageId)+1:pages.findIndex(p=>p.pageNumber===-1)>=0?pages.findIndex(p=>p.pageNumber===-1):pages.length;
    if(add.afterPageId&&at===0||pages.some(p=>p.id===add.pageId)||add.afterPageId&&pages[at-1]?.side==='back')return invalid();
    pages.splice(at,0,{id:add.pageId,pageNumber:1,side:'right',widthMm:add.widthMm,heightMm:add.heightMm,marginMm:add.marginMm,bleedMm:add.bleedMm,layoutJson:null,frames:[],texts:[]});
  }
  const ordered=publicationPageOrderPreview(pages,pages.map(p=>p.id));if(!ordered)return invalid();pages=ordered;
  const opening=pages.find(p=>p.id===data.chapter.startPageId);if(!opening||data.startOnRight&&opening.side!=='right')return invalid();
  const title=data.title,text={id:title.textId,xMm:title.xMm,yMm:title.yMm,widthMm:title.widthMm,heightMm:title.heightMm,contentJson:JSON.stringify({kind:'text',text:title.text}),styleJson:JSON.stringify(publicationTextStyle(title))};
  const tree=publicationLayers(layout);
  const next=publicationLayerPreview({...layout,pages:pages.map(p=>p.id===opening.id?{...p,texts:[text]}:p)}, {nodes:[...tree.nodes,{id:text.id,kind:'text',parentId:null,name:null,visible:true,locked:false}]});
  return put(next,[...publicationChapters(layout),data.chapter]);
}
export function publicationChapterOpening(layout:PublicationLayout,settings:{title:string;afterPageId:string|null;count:1|2;startOnRight:boolean;chapterId:string;pageId:string;textId:string;companionId:string}):CreatePublicationChapter {
  const after=settings.afterPageId?layout.pages.findIndex(p=>p.id===settings.afterPageId):-1;
  const at=settings.afterPageId?after+1:layout.pages.findIndex(p=>p.pageNumber===-1)>=0?layout.pages.findIndex(p=>p.pageNumber===-1):layout.pages.length;
  if(settings.afterPageId&&after<0)return invalid();
  const base=layout.pages.find(p=>p.id===settings.afterPageId)??layout.pages[at-1]??layout.pages[0];if(!base)return invalid();
  const number=layout.pages.slice(0,at).filter(p=>p.pageNumber>0).length+1;
  const titleSecond=settings.startOnRight&&number%2===0;
  if(titleSecond&&settings.count===1)return invalid();
  const first=titleSecond?settings.companionId:settings.pageId,second=titleSecond?settings.pageId:settings.companionId;
  const size={widthMm:base.widthMm,heightMm:base.heightMm,marginMm:base.marginMm,bleedMm:base.bleedMm};
  const title=settings.title.trim(),inside=base.heightMm-2*base.marginMm;
  const result:CreatePublicationChapter={chapter:{id:settings.chapterId,title,startPageId:settings.pageId},pages:[{pageId:first,afterPageId:settings.afterPageId,...size},...(settings.count===2?[{pageId:second,afterPageId:first,...size}]:[])],startOnRight:settings.startOnRight,title:{textId:settings.textId,pageId:settings.pageId,xMm:base.marginMm,yMm:base.marginMm+inside*.3,widthMm:base.widthMm-2*base.marginMm,heightMm:inside*.35,kind:'text',text:title,fontFamily:'serif',fontSizePt:28,lineHeight:1.4,align:'left',bold:false,color:'#222222'}};
  publicationChapterOpeningPreview(layout,result);return result;
}
