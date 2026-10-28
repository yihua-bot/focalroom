import type { Photo } from '../../domain/models';
import type { PublicationLayout,PublicationFrameUpdate,PublicationFrameAdd,PublicationFramePhoto } from '../../domain/publication';
import { publicationOrdinaryFrames } from '../../domain/publicationContinuous';
type Page=PublicationLayout['pages'][number];
/** 同一计划同时用于预览和确认后的命令，不改变源照片、裁切或选片顺序。 */
export function publicationPhotoLayout(page:Page,photos:Photo[],columns:number,newId:()=>string,protectedIds:ReadonlySet<string>=new Set()){
  const empty=page.frames.filter(f=>!f.assetId&&!protectedIds.has(f.id)).sort((a,b)=>a.yMm-b.yMm||a.xMm-b.xMm||a.id.localeCompare(b.id));
  if(empty.length){
    if(!photos.length||photos.length>empty.length||photos.length>100)throw new Error('请按空照片框数量选择照片');
    const assignments:PublicationFramePhoto[]=photos.map((photo,i)=>({frameId:empty[i].id,photoId:photo.id,renditionId:null}));
    return {updates:[] as PublicationFrameUpdate[],additions:[] as PublicationFrameAdd[],assignments,previewImages:assignments.map((frame,i)=>({frameId:frame.frameId,photoIndex:i})),preview:{...page,frames:page.frames.map(frame=>assignments.some(a=>a.frameId===frame.id)?{...frame,cropX:.5,cropY:.5,cropZoom:1}:frame)}};
  }
  const ordinary=publicationOrdinaryFrames(page).filter(frame=>!protectedIds.has(frame.id)),fixed=page.frames.filter(frame=>!ordinary.some(item=>item.id===frame.id));
  const count=ordinary.length+photos.length;
  if(!count||page.frames.length+photos.length>100||!Number.isInteger(columns)||columns<0||columns>4)throw new Error('版面照片数量或排列方式无效');
  const cols=Math.min(count,columns||Math.ceil(Math.sqrt(count))),rows=Math.ceil(count/cols);
  const width=page.widthMm-page.marginMm*2,height=page.heightMm-page.marginMm*2;
  const gap=Math.min(4,width/cols/4,height/rows/4);
  const w=(width-gap*(cols-1))/cols,h=(height-gap*(rows-1))/rows;
  if(w<=0||h<=0)throw new Error('页面没有足够空间，请调整边距');
  const positions=Array.from({length:count},(_,i)=>({xMm:page.marginMm+(i%cols)*(w+gap),yMm:page.marginMm+Math.floor(i/cols)*(h+gap),widthMm:w,heightMm:h}));
  const existing=[...ordinary].sort((a,b)=>a.yMm-b.yMm||a.xMm-b.xMm||a.id.localeCompare(b.id));
  const updates:PublicationFrameUpdate[]=existing.map((frame,i)=>({frameId:frame.id,...positions[i],fitMode:frame.fitMode,cropX:frame.cropX,cropY:frame.cropY,cropZoom:frame.cropZoom,assetId:frame.assetId,renditionId:frame.renditionId}));
  const additions:PublicationFrameAdd[]=photos.map((photo,i)=>({frameId:newId(),pageId:page.id,photoId:photo.id,renditionId:null,...positions[ordinary.length+i]}));
  const preview:Page={...page,frames:[...fixed,...existing.map((frame,i)=>({...frame,...positions[i]})),...additions.map(frame=>({id:frame.frameId,xMm:frame.xMm,yMm:frame.yMm,widthMm:frame.widthMm,heightMm:frame.heightMm,fitMode:'fit' as const,cropX:.5,cropY:.5,cropZoom:1,assetId:null,renditionId:null,source:null}))]};
  return {updates,additions,assignments:[] as PublicationFramePhoto[],previewImages:additions.map((frame,i)=>({frameId:frame.frameId,photoIndex:i})),preview};
}
