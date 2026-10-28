/** 页面坐标始终为毫米；拖动不改变裁切、内容或照片身份。 */
export interface PublicationRectangle {xMm:number;yMm:number;widthMm:number;heightMm:number}
export interface PublicationBounds {widthMm:number;heightMm:number;bleedMm:number}
export type PublicationCorner='nw'|'ne'|'se'|'sw';
export type PublicationObject={kind:'frame'|'text';id:string};
const clamp=(value:number,min:number,max:number)=>Math.min(max,Math.max(min,value));
const round=(value:number)=>Math.round(value*100)/100;
export function publicationRectangleValid(page:PublicationBounds,box:PublicationRectangle){return [page.widthMm,page.heightMm,page.bleedMm,box.xMm,box.yMm,box.widthMm,box.heightMm].every(Number.isFinite)&&page.widthMm>0&&page.heightMm>0&&page.bleedMm>=0&&box.widthMm>0&&box.heightMm>0&&box.xMm>=-page.bleedMm&&box.yMm>=-page.bleedMm&&box.xMm+box.widthMm<=page.widthMm+page.bleedMm&&box.yMm+box.heightMm<=page.heightMm+page.bleedMm;}
export function publicationTransform(page:PublicationBounds,box:PublicationRectangle,dx:number,dy:number,corner?:PublicationCorner,aspect=false):PublicationRectangle|undefined {
  if(!publicationRectangleValid(page,box)||!Number.isFinite(dx)||!Number.isFinite(dy))return;
  const left=-page.bleedMm,top=left,right=page.widthMm+page.bleedMm,bottom=page.heightMm+page.bleedMm;
  if(!corner)return {widthMm:box.widthMm,heightMm:box.heightMm,xMm:clamp(round(box.xMm+dx),left,right-box.widthMm),yMm:clamp(round(box.yMm+dy),top,bottom-box.heightMm)};
  const west=corner.endsWith('w'),north=corner.startsWith('n'),anchorX=west?box.xMm+box.widthMm:box.xMm,anchorY=north?box.yMm+box.heightMm:box.yMm;
  const maxWidth=west?anchorX-left:right-anchorX,maxHeight=north?anchorY-top:bottom-anchorY,min=Math.min(1,box.widthMm,box.heightMm);
  let width=box.widthMm+(west?-dx:dx),height=box.heightMm+(north?-dy:dy);
  if(aspect){const scale=clamp(Math.abs(width/box.widthMm-1)>=Math.abs(height/box.heightMm-1)?width/box.widthMm:height/box.heightMm,Math.max(min/box.widthMm,min/box.heightMm),Math.min(maxWidth/box.widthMm,maxHeight/box.heightMm));width=box.widthMm*scale;height=box.heightMm*scale;}
  else {width=clamp(round(width),min,maxWidth);height=clamp(round(height),min,maxHeight);}
  const result={xMm:west?anchorX-width:anchorX,yMm:north?anchorY-height:anchorY,widthMm:width,heightMm:height};
  // 极端小数边界可能有浮点尾差；向内收紧，不放大页面范围。
  result.xMm=Math.max(left,result.xMm);result.yMm=Math.max(top,result.yMm);
  if(result.xMm+result.widthMm>right)result.widthMm=Math.max(min,right-result.xMm-1e-9);
  if(result.yMm+result.heightMm>bottom)result.heightMm=Math.max(min,bottom-result.yMm-1e-9);
  return publicationRectangleValid(page,result)?result:undefined;
}
