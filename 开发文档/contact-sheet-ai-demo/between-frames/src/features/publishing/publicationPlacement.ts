import type { Photo } from '../../domain/models';
import type { PublicationFrameAdd,PublicationFramePhoto,PublicationLayout } from '../../domain/publication';
import { publicationRectangleValid } from './publicationGeometry';
type Page=PublicationLayout['pages'][number];
export type PublicationPlacement={pageId:string;photoId:string;frameId:string;preview:Page;additions:PublicationFrameAdd[];assignments:PublicationFramePhoto[]};
/** 放置只增加一个框或填明确空框；不会重排已有照片或文字。 */
export function publicationPlacement(page:Page,photo:Photo,point:{xMm:number;yMm:number},newId:()=>string,emptyFrameId?:string):PublicationPlacement|undefined{
  if(!Number.isFinite(point.xMm)||!Number.isFinite(point.yMm)||!Number.isFinite(page.marginMm)||page.marginMm<0)return;
  if(emptyFrameId){
    const frame=page.frames.find(value=>value.id===emptyFrameId);if(!frame||frame.assetId||!publicationRectangleValid(page,frame))return;
    return {pageId:page.id,photoId:photo.id,frameId:frame.id,additions:[],assignments:[{frameId:frame.id,photoId:photo.id,renditionId:null}],preview:{...page,frames:page.frames.map(value=>value.id===frame.id?{...value,cropX:.5,cropY:.5,cropZoom:1}:value)}};
  }
  const innerWidth=page.widthMm-page.marginMm*2,innerHeight=page.heightMm-page.marginMm*2;
  if(innerWidth<=0||innerHeight<=0||page.frames.length>=100)return;
  const ratio=photo.width&&photo.height&&photo.width>0&&photo.height>0&&Number.isFinite(photo.width/photo.height)?photo.width/photo.height:4/3;
  const widthMm=Math.min(innerWidth*.4,innerHeight*.4*ratio),heightMm=widthMm/ratio;
  const xMm=Math.max(page.marginMm,Math.min(page.widthMm-page.marginMm-widthMm,point.xMm-widthMm/2)),yMm=Math.max(page.marginMm,Math.min(page.heightMm-page.marginMm-heightMm,point.yMm-heightMm/2));
  const rectangle={xMm,yMm,widthMm,heightMm};if(!publicationRectangleValid(page,rectangle))return;
  const frameId=newId();if(!frameId||page.frames.some(frame=>frame.id===frameId)||page.texts.some(text=>text.id===frameId))return;
  const addition:PublicationFrameAdd={frameId,pageId:page.id,photoId:photo.id,renditionId:null,...rectangle};
  return {pageId:page.id,photoId:photo.id,frameId,additions:[addition],assignments:[],preview:{...page,frames:[...page.frames,{id:frameId,...rectangle,fitMode:'fit',cropX:.5,cropY:.5,cropZoom:1,assetId:null,renditionId:null,source:null}]}};
}
