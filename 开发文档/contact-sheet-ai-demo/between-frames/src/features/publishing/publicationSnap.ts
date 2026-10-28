import { publicationRectangleValid,type PublicationBounds,type PublicationCorner,type PublicationRectangle } from './publicationGeometry';
export interface PublicationSnapLines {x:number[];y:number[]}
export interface PublicationSnapContext {before:PublicationRectangle;corner?:PublicationCorner;aspect:boolean;unitsPerPixel:{x:number;y:number}}
/** 候选顺序同距离时优先自定义参考线；只改拖动预览，不接管毫米键盘微调。 */
export function publicationSnap(bounds:PublicationBounds,box:PublicationRectangle,targets:PublicationSnapLines,context:PublicationSnapContext){
  const tolerance={x:6*context.unitsPerPixel.x,y:6*context.unitsPerPixel.y};
  const closest=(axis:'x'|'y',points:number[])=>{let best:{delta:number;line:number}|undefined;for(const line of targets[axis])for(const point of points){const delta=line-point;if(Number.isFinite(delta)&&Math.abs(delta)<=tolerance[axis]&&(!best||Math.abs(delta)<Math.abs(best.delta)-1e-8))best={delta,line};}return best;};
  const {corner,before}=context,west=corner?.endsWith('w'),north=corner?.startsWith('n');
  const x=closest('x',corner?[west?box.xMm:box.xMm+box.widthMm]:[box.xMm,box.xMm+box.widthMm/2,box.xMm+box.widthMm]),y=closest('y',corner?[north?box.yMm:box.yMm+box.heightMm]:[box.yMm,box.yMm+box.heightMm/2,box.yMm+box.heightMm]);
  let result={...box};
  if(corner&&context.aspect){
    const candidates=[...(x?[{axis:'x' as const,delta:x.delta,distance:Math.abs(x.delta/context.unitsPerPixel.x)}]:[]),...(y?[{axis:'y' as const,delta:y.delta,distance:Math.abs(y.delta/context.unitsPerPixel.y)}]:[])].sort((a,b)=>a.distance-b.distance);
    for(const candidate of candidates){const scale=candidate.axis==='x'?(box.widthMm+(west?-candidate.delta:candidate.delta))/before.widthMm:(box.heightMm+(north?-candidate.delta:candidate.delta))/before.heightMm,widthMm=before.widthMm*scale,heightMm=before.heightMm*scale;
      const value={widthMm,heightMm,xMm:west?before.xMm+before.widthMm-widthMm:before.xMm,yMm:north?before.yMm+before.heightMm-heightMm:before.yMm};if(widthMm>=Math.min(1,before.widthMm)&&heightMm>=Math.min(1,before.heightMm)&&publicationRectangleValid(bounds,value)){result=value;break;}}
  }else{
    if(x){const value=corner?{...result,xMm:west?result.xMm+x.delta:result.xMm,widthMm:result.widthMm+(west?-x.delta:x.delta)}:{...result,xMm:result.xMm+x.delta};if(value.widthMm>=Math.min(1,before.widthMm)&&publicationRectangleValid(bounds,value))result=value;}
    if(y){const value=corner?{...result,yMm:north?result.yMm+y.delta:result.yMm,heightMm:result.heightMm+(north?-y.delta:y.delta)}:{...result,yMm:result.yMm+y.delta};if(value.heightMm>=Math.min(1,before.heightMm)&&publicationRectangleValid(bounds,value))result=value;}
  }
  const points={x:corner?[west?result.xMm:result.xMm+result.widthMm]:[result.xMm,result.xMm+result.widthMm/2,result.xMm+result.widthMm],y:corner?[north?result.yMm:result.yMm+result.heightMm]:[result.yMm,result.yMm+result.heightMm/2,result.yMm+result.heightMm]};
  return {rectangle:result,lines:{x:[...new Set(targets.x.filter(line=>points.x.some(point=>Math.abs(point-line)<1e-7)))],y:[...new Set(targets.y.filter(line=>points.y.some(point=>Math.abs(point-line)<1e-7)))]}};
}
