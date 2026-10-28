import { PUBLICATION_COVER_KEY,setPublicationCoverSchema,type SetPublicationCover } from '@betweenframes/contracts';
import type { PublicationLayout } from '../../domain/publication';
import { checkPublicationCover,publicationPageCover } from '../../domain/publicationCover';
import { publicationLayers,publicationLayerViews,publicationLayerPreview } from '../../domain/publicationLayers';
import { publicationPageGuides } from '../../domain/publicationGuides';
import { publicationChapters } from '../../domain/publicationChapters';
type Page=PublicationLayout['pages'][number];
export class PublicationCoverError extends Error{
  constructor(readonly reason:'scope'|'locked'|'settings'|'crossing'|'bounds'|'noop'){super('');}
}
const fail=(reason:PublicationCoverError['reason']):never=>{throw new PublicationCoverError(reason);};
function comparable(layout:PublicationLayout){
  const value={...layout,publication:{...layout.publication,specJson:JSON.parse(layout.publication.specJson)},pages:layout.pages.map(page=>({...page,layoutJson:page.layoutJson===null?null:JSON.parse(page.layoutJson)}))};
  return JSON.stringify(value,(_key,item)=>item&&typeof item==='object'&&!Array.isArray(item)?Object.fromEntries(Object.entries(item).sort(([a],[b])=>a.localeCompare(b))):item);
}
/** 封底—书脊—封面共用一张实际展开页；原照片、文字与来源身份保持。 */
export function publicationCoverPlan(layout:PublicationLayout,data:SetPublicationCover){
  if(!setPublicationCoverSchema.safeParse(data).success||layout.publication.pubType!=='photobook')return fail('scope');
  checkPublicationCover(layout);
  const front=layout.pages.find(page=>page.pageNumber===0),back=layout.pages.find(page=>page.pageNumber===-1);
  const allIds=layout.pages.flatMap(page=>[page.id,...page.frames.map(frame=>frame.id),...page.texts.map(text=>text.id)]);
  const old=front?publicationPageCover(front):undefined;
  if(!old&&data.contentPlacement!=='followPanels')return fail('scope');
  if(front&&front.id!==data.pageId||!front&&allIds.includes(data.pageId)||(back?.id??null)!==data.backPageId||front&&front.side!=='cover'||back&&back.side!=='back')return fail('scope');
  if(!old&&[front,back].some(page=>page?.layoutJson!==null&&page?.layoutJson!==undefined)&&!data.clearPageSettingsConfirmed)return fail('settings');
  const layers=publicationLayers(layout),views=publicationLayerViews(layers);
  const affected=[front,back].filter((page):page is Page=>!!page);
  if(affected.flatMap(page=>[...page.frames,...page.texts]).some(object=>views.find(node=>node.id===object.id)?.effectiveLocked))return fail('locked');
  const {panelWidthMm:width,spineWidthMm:spine}=data.settings,total=width*2+spine,offset=width+spine;
  const page:Page=front?structuredClone(front):{id:data.pageId,pageNumber:0,side:'cover',widthMm:total,heightMm:data.heightMm,marginMm:data.marginMm,bleedMm:data.bleedMm,layoutJson:null,frames:[],texts:[]};
  function delta(object:{xMm:number;widthMm:number}){
    if(!old)return offset;
    if(data.contentPlacement==='keepPosition'||old.panelWidthMm===width&&old.spineWidthMm===spine)return 0;
    const x=object.xMm,end=x+object.widthMm;
    if(x>=-front!.bleedMm&&end<=old.panelWidthMm)return 0;
    if(x>=old.panelWidthMm&&end<=old.panelWidthMm+old.spineWidthMm)return width-old.panelWidthMm;
    if(x>=old.panelWidthMm+old.spineWidthMm&&end<=front!.widthMm+front!.bleedMm)return offset-old.panelWidthMm-old.spineWidthMm;
    return fail('crossing');
  }
  for(const object of [...page.frames,...page.texts])object.xMm+=delta(object);
  if(back){page.frames.push(...structuredClone(back.frames));page.texts.push(...structuredClone(back.texts));}
  page.frames.sort((a,b)=>a.id.localeCompare(b.id));page.texts.sort((a,b)=>a.id.localeCompare(b.id));
  Object.assign(page,{widthMm:total,heightMm:data.heightMm,marginMm:data.marginMm,bleedMm:data.bleedMm});
  if(page.marginMm*2>=Math.min(total,page.heightMm)||[...page.frames,...page.texts].some(object=>object.xMm<-page.bleedMm||object.yMm<-page.bleedMm||object.xMm+object.widthMm>total+page.bleedMm||object.yMm+object.heightMm>page.heightMm+page.bleedMm))return fail('bounds');
  const metadata=old&&!data.clearPageSettingsConfirmed?JSON.parse(front!.layoutJson??'{}'):{};
  metadata[PUBLICATION_COVER_KEY]=data.settings;page.layoutJson=JSON.stringify(metadata);
  const preview=publicationLayerPreview({...layout,pages:[page,...layout.pages.filter(item=>item.id!==front?.id&&item.id!==back?.id)]},layers);
  if(preview.pages.length>512)return fail('bounds');
  publicationPageGuides(page);checkPublicationCover(preview);publicationChapters(preview);publicationLayers(preview);
  if(comparable(preview)===comparable(layout))return fail('noop');
  return {preview,page,back,initial:!old,data};
}
