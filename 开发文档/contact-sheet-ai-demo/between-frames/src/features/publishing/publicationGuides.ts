import type { PublicationRectangle } from './publicationGeometry';
import { publicationPageGuides,type GuidePage } from '../../domain/publicationGuides';
export { publicationPageGuides,publicationGuidesPreview,publicationValidateGuides,PublicationGuidesError } from '../../domain/publicationGuides';
export type { GuidePage,PublicationPageGuides } from '../../domain/publicationGuides';
export interface PublicationGuideView {rulers:boolean;grid:boolean;margins:boolean;guides:boolean;snap:boolean}
export const publicationDefaultGuideView:PublicationGuideView={rulers:false,grid:false,margins:false,guides:true,snap:false};

function gridAxis(length:number,margin:number,count:number,gap:number){const cell=(length-2*margin-(count-1)*gap)/count,lines=[margin,length-margin];for(let i=1;i<count;i++){lines.push(margin+i*cell+(i-1)*gap,margin+i*(cell+gap));}return [...new Set(lines)].sort((a,b)=>a-b);}
export function publicationGuideLines(page:GuidePage,view:Pick<PublicationGuideView,'grid'|'margins'|'guides'>){
  const settings=publicationPageGuides(page),grid={x:view.grid?gridAxis(page.widthMm,page.marginMm,settings.columns,settings.columnGapMm):[],y:view.grid?gridAxis(page.heightMm,page.marginMm,settings.rows,settings.rowGapMm):[]};
  return {grid,margins:{x:view.margins?[page.marginMm,page.widthMm-page.marginMm]:[],y:view.margins?[page.marginMm,page.heightMm-page.marginMm]:[]},custom:{x:view.guides?settings.verticalGuidesMm:[],y:view.guides?settings.horizontalGuidesMm:[]}};
}
export function publicationSnapTargets(page:GuidePage,view:PublicationGuideView,objects:PublicationRectangle[]=[]){
  const lines=publicationGuideLines(page,view);
  return {x:[...lines.custom.x,...lines.margins.x,...lines.grid.x,0,page.widthMm/2,page.widthMm,...objects.flatMap(box=>[box.xMm,box.xMm+box.widthMm/2,box.xMm+box.widthMm])],y:[...lines.custom.y,...lines.margins.y,...lines.grid.y,0,page.heightMm/2,page.heightMm,...objects.flatMap(box=>[box.yMm,box.yMm+box.heightMm/2,box.yMm+box.heightMm])]};
}
/** 标签间距按实际呈现宽度自适应；数值仍为版面毫米，不声称屏幕已做物理校准。 */
export function publicationRulerTicks(length:number,pixels:number){
  if(!Number.isFinite(length)||!Number.isFinite(pixels)||length<=0||pixels<=0)return [];
  const wanted=length/Math.max(1,pixels/48),power=10**Math.floor(Math.log10(wanted)),step=([1,2,5,10].find(value=>value*power>=wanted)??10)*power,minor=step/5;
  const result:Array<{mm:number;major:boolean}>=[];for(let i=0;i*minor<=length+1e-8&&i<1000;i++)result.push({mm:Number((i*minor).toPrecision(12)),major:i%5===0});return result;
}
