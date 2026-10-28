import { expect,it } from 'vitest';
import { publicationSnap } from './publicationSnap';
import { publicationRectangleValid,publicationTransform,type PublicationCorner } from './publicationGeometry';
const bounds={widthMm:200,heightMm:300,bleedMm:3},before={xMm:15,yMm:20,widthMm:80,heightMm:100},context={before,aspect:false,unitsPerPixel:{x:.1,y:.1}};
it('吸附阈值按屏幕像素计算，最近边或中心对齐，超出阈值保持原位',()=>{
 const raw={...before,xMm:24.5,yMm:28};const result=publicationSnap(bounds,raw,{x:[25],y:[30]},context);expect(result.rectangle).toEqual({...raw,xMm:25});expect(result.lines).toEqual({x:[25],y:[]});expect(publicationSnap(bounds,raw,{x:[25],y:[30]},{...context,unitsPerPixel:{x:1,y:1}}).rectangle).toEqual({...raw,xMm:25,yMm:30});
});
it('四角吸附固定相对锚点，Shift等比时只接受真实匹配的辅助线',()=>{
 for(const corner of ['nw','ne','se','sw'] as PublicationCorner[]){const raw=publicationTransform(bounds,before,7,5,corner)!,west=corner.endsWith('w'),north=corner.startsWith('n'),target=(west?raw.xMm:raw.xMm+raw.widthMm)+.5;const result=publicationSnap(bounds,raw,{x:[target],y:[]},{...context,corner});expect(west?result.rectangle.xMm+result.rectangle.widthMm:result.rectangle.xMm).toBeCloseTo(west?before.xMm+before.widthMm:before.xMm);expect(north?result.rectangle.yMm+result.rectangle.heightMm:result.rectangle.yMm).toBeCloseTo(north?before.yMm+before.heightMm:before.yMm);expect(result.lines.x).toEqual([target]);}
 const raw=publicationTransform(bounds,before,8,10,'se',true)!,result=publicationSnap(bounds,raw,{x:[103.5],y:[]},{...context,corner:'se',aspect:true});expect(result.rectangle.widthMm/result.rectangle.heightMm).toBeCloseTo(.8);expect(result.rectangle.xMm+result.rectangle.widthMm).toBe(103.5);expect(result.lines.x).toEqual([103.5]);
});
it('不能为吸附越出页面或生成零宽度；连续跨页仍使用完整毫米坐标',()=>{
 const raw={xMm:163,yMm:20,widthMm:40,heightMm:100};expect(publicationSnap(bounds,raw,{x:[205],y:[]},{...context,before:raw,unitsPerPixel:{x:1,y:1}}).rectangle).toEqual(raw);
 const cross={xMm:20,yMm:20,widthMm:350,heightMm:240},moved={...cross,xMm:29.5},result=publicationSnap({...bounds,widthMm:400},moved,{x:[30],y:[]},{...context,before:cross});expect(result.rectangle).toEqual({...cross,xMm:30});expect(publicationRectangleValid({...bounds,widthMm:400},result.rectangle)).toBe(true);
});
it('大量页面与角点组合吸附保持范围、锚点和等比约束',()=>{
 for(let i=0;i<120;i++){const p={widthMm:100+i*.8,heightMm:150+i,bleedMm:3},box={xMm:10,yMm:15,widthMm:50,heightMm:70};for(const corner of ['nw','ne','se','sw']as PublicationCorner[])for(const aspect of [false,true]){const raw=publicationTransform(p,box,(i-60)*.2,(70-i)*.2,corner,aspect)!;const result=publicationSnap(p,raw,{x:[0,15,p.widthMm/2,p.widthMm],y:[0,20,p.heightMm/2,p.heightMm]},{before:box,corner,aspect,unitsPerPixel:{x:.25,y:.25}});expect(publicationRectangleValid(p,result.rectangle)).toBe(true);if(aspect)expect(result.rectangle.widthMm/result.rectangle.heightMm).toBeCloseTo(box.widthMm/box.heightMm);}}
});
