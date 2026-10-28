import { z } from 'zod';
import type { PublicationLayout,PublicationLayoutFrame,PublicationSpreadFrame,PublicationUnspreadFrame,PublicationPreview } from './publication';

export const CONTINUOUS_FRAME_KEY='focalroomContinuousFramesV1';
type Page=PublicationLayout['pages'][number];
const linksSchema=z.array(z.strictObject({frameId:z.uuid(),rightPageId:z.uuid()})).max(100);
export interface ContinuousFrame {left:Page;right:Page;frame:PublicationLayoutFrame}
export class ContinuousLayoutError extends Error {readonly code='CONTINUOUS_LAYOUT_INVALID';constructor(){super('');}}
const invalid=():never=>{throw new ContinuousLayoutError();};
function metadata(page:Page):Record<string,unknown>{
  if(!page.layoutJson)return {};let value:unknown;try{value=JSON.parse(page.layoutJson);}catch{invalid();}
  if(!value||typeof value!=='object'||Array.isArray(value))return invalid();return value as Record<string,unknown>;
}
export function publicationContinuousLinks(page:Page){
  const value=metadata(page);if(value[CONTINUOUS_FRAME_KEY]===undefined)return [];
  if(new TextEncoder().encode(page.layoutJson!).length>64*1024)return invalid();
  const result=linksSchema.safeParse(value[CONTINUOUS_FRAME_KEY]);if(!result.success||new Set(result.data.map(link=>link.frameId)).size!==result.data.length)return invalid();return result.data;
}
function withLinks(page:Page,links:ReturnType<typeof publicationContinuousLinks>):Page{
  if(JSON.stringify(publicationContinuousLinks(page))===JSON.stringify(links))return page;
  const value=metadata(page);if(links.length)value[CONTINUOUS_FRAME_KEY]=links;else delete value[CONTINUOUS_FRAME_KEY];
  const layoutJson=Object.keys(value).length?JSON.stringify(value):null;if(layoutJson&&new TextEncoder().encode(layoutJson).length>64*1024)return invalid();return {...page,layoutJson};
}
export function publicationOrdinaryFrames(page:Page){const ids=new Set(publicationContinuousLinks(page).map(link=>link.frameId));return page.frames.filter(frame=>!ids.has(frame.id));}
export function publicationContinuousPair(pages:Page[],leftId:string,rightId:string):[Page,Page]{
  const index=pages.findIndex(page=>page.id===leftId),left=pages[index],right=pages[index+1];
  if(!left||!right||right.id!==rightId||left.side!=='left'||right.side!=='right'||left.pageNumber<2||left.pageNumber%2!==0||right.pageNumber!==left.pageNumber+1||left.widthMm!==right.widthMm||left.heightMm!==right.heightMm||left.bleedMm!==right.bleedMm)return invalid();return [left,right];
}
export function publicationContinuousBounds(left:Page,right:Page){return {widthMm:left.widthMm+right.widthMm,heightMm:left.heightMm,bleedMm:left.bleedMm};}
function validRectangle(bounds:{widthMm:number;heightMm:number;bleedMm:number},frame:{xMm:number;yMm:number;widthMm:number;heightMm:number}){return [frame.xMm,frame.yMm,frame.widthMm,frame.heightMm].every(Number.isFinite)&&frame.widthMm>0&&frame.heightMm>0&&frame.xMm>=-bounds.bleedMm&&frame.yMm>=-bounds.bleedMm&&frame.xMm+frame.widthMm<=bounds.widthMm+bounds.bleedMm&&frame.yMm+frame.heightMm<=bounds.heightMm+bounds.bleedMm;}
/** 物理配对与唯一正式对象共用一份解析，供读取校验、排序和投影使用。 */
export function publicationContinuousPairs(pages:Page[]):ContinuousFrame[]{
  const ids=new Set<string>(),result:ContinuousFrame[]=[];
  for(const owner of pages){for(const link of publicationContinuousLinks(owner)){
    const [left,right]=publicationContinuousPair(pages,owner.id,link.rightPageId),frame=left.frames.find(frame=>frame.id===link.frameId);
    if(!frame||ids.has(frame.id)||!frame.assetId||!frame.source||frame.source.assetId!==frame.assetId||frame.source.renditionId!==frame.renditionId||!validRectangle(publicationContinuousBounds(left,right),frame)||frame.xMm>=left.widthMm||frame.xMm+frame.widthMm<=left.widthMm)return invalid();
    ids.add(frame.id);result.push({left,right,frame});
  }}
  for(const item of result){for(const page of [item.left,item.right]){const visible=[...page.frames,...result.filter(other=>other.right.id===page.id).map(other=>other.frame)];if(visible.some(frame=>frame.id!==item.frame.id&&frame.assetId===item.frame.assetId&&frame.renditionId===item.frame.renditionId))return invalid();}}
  return result.sort((a,b)=>a.frame.id.localeCompare(b.frame.id));
}
export function publicationContinuousFrames(layout:PublicationLayout):ContinuousFrame[]{
  const result=publicationContinuousPairs(layout.pages);if(result.length&&(![2,3].includes(layout.schemaVersion)||layout.publication.pubType!=='photobook'))return invalid();return result;
}
export function publicationVisibleFrames(page:Page,continuous:ContinuousFrame[]):PublicationLayoutFrame[]{return [...page.frames,...continuous.filter(item=>item.right.id===page.id).map(item=>item.frame)];}
export function publicationVisiblePhotoIds(page:Page,continuous:ContinuousFrame[],previews:PublicationPreview[]){const ids=new Set(publicationVisibleFrames(page,continuous).map(frame=>frame.id));return previews.filter(preview=>ids.has(preview.frameId)).flatMap(preview=>preview.photoId?[preview.photoId]:[]);}
/** 每侧只有可见视窗；视窗中的图像仍按整张跨页框的尺寸与原点渲染。 */
export function publicationContinuousWindow(page:Page,frame:PublicationLayoutFrame,offsetMm:number){
  const xMm=Math.max(0,frame.xMm-offsetMm),yMm=Math.max(0,frame.yMm),right=Math.min(page.widthMm,frame.xMm+frame.widthMm-offsetMm),bottom=Math.min(page.heightMm,frame.yMm+frame.heightMm);
  const widthMm=right-xMm,heightMm=bottom-yMm;if(widthMm<=0||heightMm<=0)return undefined;
  return {rectangle:{xMm,yMm,widthMm,heightMm},image:{left:(frame.xMm-offsetMm-xMm)/widthMm*100,top:(frame.yMm-yMm)/heightMm*100,width:frame.widthMm/widthMm*100,height:frame.heightMm/heightMm*100}};
}

export function publicationLinkPreview(layout:PublicationLayout,data:PublicationSpreadFrame):PublicationLayout{
  const [left,right]=publicationContinuousPair(layout.pages,data.leftPageId,data.rightPageId),owner=layout.pages.find(page=>page.frames.some(frame=>frame.id===data.frameId)),source=owner?.frames.find(frame=>frame.id===data.frameId);
  if(!owner||!source||owner.id!==left.id&&owner.id!==right.id||!source.assetId)return invalid();
  const frame={...source,...data.geometry};
  const pages=layout.pages.map(page=>{const links=publicationContinuousLinks(page).filter(link=>link.frameId!==frame.id);if(page.id===left.id)links.push({frameId:frame.id,rightPageId:right.id});const result=withLinks(page,links),frames=page.frames.filter(item=>item.id!==frame.id);if(page.id===left.id)frames.push(frame);return {...result,frames:page.id===left.id?frames.sort((a,b)=>a.id.localeCompare(b.id)):frames};});
  const preview:PublicationLayout={...layout,schemaVersion:layout.schemaVersion===3?3:2,pages};publicationContinuousFrames(preview);return preview;
}
export function publicationUnlinkPreview(layout:PublicationLayout,data:PublicationUnspreadFrame):PublicationLayout{
  const item=publicationContinuousFrames(layout).find(item=>item.frame.id===data.frameId);if(!item||data.targetPageId!==item.left.id&&data.targetPageId!==item.right.id)return invalid();const target=layout.pages.find(page=>page.id===data.targetPageId)!,frame={...item.frame,...data.geometry};if(!validRectangle(target,frame))return invalid();
  const pages=layout.pages.map(page=>{const links=publicationContinuousLinks(page);const next=links.some(link=>link.frameId===frame.id)?withLinks(page,links.filter(link=>link.frameId!==frame.id)):page;const frames=page.frames.filter(other=>other.id!==frame.id);if(page.id===target.id)frames.push(frame);return {...next,frames:frames.sort((a,b)=>a.id.localeCompare(b.id))};});
  const preview:PublicationLayout={...layout,schemaVersion:layout.schemaVersion===3?3:pages.some(page=>publicationContinuousLinks(page).length)?2:1,pages};publicationContinuousFrames(preview);return preview;
}
