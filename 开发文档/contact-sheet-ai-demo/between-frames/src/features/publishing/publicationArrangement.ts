import type { PublicationLayout,PublicationFrameUpdate,PublicationTextGeometry } from '../../domain/publication';
import { publicationRectangleValid,type PublicationBounds,type PublicationObject,type PublicationRectangle } from './publicationGeometry';
import { publicationOrdinaryFrames } from '../../domain/publicationContinuous';
type Page=PublicationLayout['pages'][number];
export type PublicationObjectGeometry=PublicationObject&PublicationRectangle;
export type PublicationTransformObject=PublicationObject|{kind:'selection';id:string};
export type PublicationSelectionIntent={additive?:boolean;preserve?:boolean;member?:boolean};
export type PublicationArrangeAction='left'|'centerX'|'right'|'top'|'centerY'|'bottom'|'distributeX'|'distributeY';
export const publicationObjectKey=(object:PublicationObject)=>object.kind+':'+object.id;
export const publicationGeometryEqual=(a:PublicationRectangle,b:PublicationRectangle)=>Math.abs(a.xMm-b.xMm)<1e-8&&Math.abs(a.yMm-b.yMm)<1e-8&&Math.abs(a.widthMm-b.widthMm)<1e-8&&Math.abs(a.heightMm-b.heightMm)<1e-8;
export function publicationPageObjects(page:Page):PublicationObjectGeometry[]{return [...publicationOrdinaryFrames(page).map(frame=>({kind:'frame' as const,id:frame.id,xMm:frame.xMm,yMm:frame.yMm,widthMm:frame.widthMm,heightMm:frame.heightMm})),...page.texts.map(text=>({kind:'text' as const,id:text.id,xMm:text.xMm,yMm:text.yMm,widthMm:text.widthMm,heightMm:text.heightMm}))];}
export function publicationSelectionObjects(page:Page,selection:PublicationObject[]):PublicationObjectGeometry[]|undefined{
  if(!selection.length||selection.length>100||new Set(selection.map(publicationObjectKey)).size!==selection.length)return;
  const objects=new Map(publicationPageObjects(page).map(item=>[publicationObjectKey(item),item]));const result=selection.map(item=>objects.get(publicationObjectKey(item)));
  return result.every((item):item is PublicationObjectGeometry=>!!item&&publicationRectangleValid(page,item))?result:undefined;
}
export function publicationSelectionBounds(objects:PublicationObjectGeometry[]):PublicationRectangle|undefined{
  if(!objects.length||objects.length>100||new Set(objects.map(publicationObjectKey)).size!==objects.length)return;const x=Math.min(...objects.map(o=>o.xMm)),y=Math.min(...objects.map(o=>o.yMm));return {xMm:x,yMm:y,widthMm:Math.max(...objects.map(o=>o.xMm+o.widthMm))-x,heightMm:Math.max(...objects.map(o=>o.yMm+o.heightMm))-y};
}
function inside(page:PublicationBounds,box:PublicationObjectGeometry):PublicationObjectGeometry|undefined{
  const value={...box,xMm:Math.max(-page.bleedMm,box.xMm),yMm:Math.max(-page.bleedMm,box.yMm)};
  if(value.xMm+value.widthMm>page.widthMm+page.bleedMm)value.widthMm=page.widthMm+page.bleedMm-value.xMm-1e-9;
  if(value.yMm+value.heightMm>page.heightMm+page.bleedMm)value.heightMm=page.heightMm+page.bleedMm-value.yMm-1e-9;
  return publicationRectangleValid(page,value)?value:undefined;
}
export function publicationTransformSelection(page:PublicationBounds,objects:PublicationObjectGeometry[],target:PublicationRectangle):PublicationObjectGeometry[]|undefined{
  const bounds=publicationSelectionBounds(objects);if(!bounds||!publicationRectangleValid(page,bounds)||!publicationRectangleValid(page,target)||!objects.every(o=>publicationRectangleValid(page,o)))return;
  if(publicationGeometryEqual(bounds,target))return objects.map(item=>({...item}));
  const sx=target.widthMm/bounds.widthMm,sy=target.heightMm/bounds.heightMm;
  const result=objects.map(item=>inside(page,{...item,xMm:target.xMm+(item.xMm-bounds.xMm)*sx,yMm:target.yMm+(item.yMm-bounds.yMm)*sy,widthMm:item.widthMm*sx,heightMm:item.heightMm*sy}));
  return result.every((item):item is PublicationObjectGeometry=>!!item)?result:undefined;
}
function translatedInside(page:PublicationBounds,box:PublicationObjectGeometry):PublicationObjectGeometry|undefined{
  const min=-page.bleedMm,right=page.widthMm+page.bleedMm,bottom=page.heightMm+page.bleedMm;
  if(box.xMm<min-1e-8||box.yMm<min-1e-8||box.xMm+box.widthMm>right+1e-8||box.yMm+box.heightMm>bottom+1e-8)return;
  const value={...box,xMm:Math.max(min,Math.min(right-box.widthMm,box.xMm)),yMm:Math.max(min,Math.min(bottom-box.heightMm,box.yMm))};
  if(value.xMm+value.widthMm>right)value.xMm=Math.max(min,value.xMm-1e-9);if(value.yMm+value.heightMm>bottom)value.yMm=Math.max(min,value.yMm-1e-9);
  return publicationRectangleValid(page,value)?value:undefined;
}
export function publicationArrange(page:PublicationBounds&{marginMm:number},objects:PublicationObjectGeometry[],action:PublicationArrangeAction,reference:'selection'|'page'):PublicationObjectGeometry[]|undefined{
  if(objects.length<2||objects.length>100||!Number.isFinite(page.marginMm)||page.marginMm<0||!objects.every(o=>publicationRectangleValid(page,o)))return;
  const bounds=reference==='page'?{xMm:page.marginMm,yMm:page.marginMm,widthMm:page.widthMm-page.marginMm*2,heightMm:page.heightMm-page.marginMm*2}:publicationSelectionBounds(objects);
  if(!bounds||!publicationRectangleValid(page,bounds))return;
  if(action==='distributeX'||action==='distributeY'){
    if(objects.length<3)return;const horizontal=action==='distributeX',coordinate=horizontal?'xMm':'yMm',size=horizontal?'widthMm':'heightMm';
    const sorted=[...objects].sort((a,b)=>a[coordinate]-b[coordinate]||publicationObjectKey(a).localeCompare(publicationObjectKey(b)));
    const gap=(bounds[size]-objects.reduce((sum,item)=>sum+item[size],0))/(objects.length-1);if(gap < -1e-8)return;
    let at=bounds[coordinate];const changed=new Map<string,PublicationObjectGeometry>();for(const item of sorted){const next=translatedInside(page,{...item,[coordinate]:at});if(!next)return;changed.set(publicationObjectKey(item),next);at+=item[size]+Math.max(0,gap);}
    return objects.map(item=>changed.get(publicationObjectKey(item))!);
  }
  const result=objects.map(item=>{
    let x=item.xMm,y=item.yMm;
    if(action==='left')x=bounds.xMm;if(action==='centerX')x=bounds.xMm+(bounds.widthMm-item.widthMm)/2;if(action==='right')x=bounds.xMm+bounds.widthMm-item.widthMm;
    if(action==='top')y=bounds.yMm;if(action==='centerY')y=bounds.yMm+(bounds.heightMm-item.heightMm)/2;if(action==='bottom')y=bounds.yMm+bounds.heightMm-item.heightMm;
    // 对齐不改变大小；超出范围整组拒绝，不能靠裁小对象通过检查。
    return translatedInside(page,{...item,xMm:x,yMm:y});
  });return result.every((item):item is PublicationObjectGeometry=>!!item)?result:undefined;
}
export function publicationArrangementPreview(page:Page,draft:PublicationObjectGeometry[]|undefined):Page{
  if(!draft)return page;const changed=new Map(draft.map(item=>[publicationObjectKey(item),item]));const geometry=(item:PublicationObjectGeometry|undefined)=>item?{xMm:item.xMm,yMm:item.yMm,widthMm:item.widthMm,heightMm:item.heightMm}:{};
  return {...page,frames:page.frames.map(item=>({...item,...geometry(changed.get('frame:'+item.id))})),texts:page.texts.map(item=>({...item,...geometry(changed.get('text:'+item.id))}))};
}
export function publicationArrangementUpdates(page:Page,draft:PublicationObjectGeometry[]){
  const original=publicationSelectionObjects(page,draft);if(!original||!draft.every(o=>publicationRectangleValid(page,o)))return;
  const frames:PublicationFrameUpdate[]=[],texts:PublicationTextGeometry[]=[];
  for(let i=0;i<draft.length;i++){const item=draft[i];if(publicationGeometryEqual(item,original[i]))continue;
    if(item.kind==='frame'){const frame=page.frames.find(f=>f.id===item.id)!;frames.push({frameId:frame.id,xMm:item.xMm,yMm:item.yMm,widthMm:item.widthMm,heightMm:item.heightMm,fitMode:frame.fitMode,cropX:frame.cropX,cropY:frame.cropY,cropZoom:frame.cropZoom,assetId:frame.assetId,renditionId:frame.renditionId});}
    else texts.push({textId:item.id,pageId:page.id,xMm:item.xMm,yMm:item.yMm,widthMm:item.widthMm,heightMm:item.heightMm});
  }return {frames,texts};
}
