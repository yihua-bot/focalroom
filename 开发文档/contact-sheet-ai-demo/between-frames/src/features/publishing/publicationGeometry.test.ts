import { expect,it } from 'vitest';
import { publicationRectangleValid,publicationTransform,type PublicationCorner } from './publicationGeometry';
const page={widthMm:200,heightMm:300,bleedMm:3},box={xMm:15,yMm:20,widthMm:80,heightMm:100};
it('移动使用页面毫米并限制在出血范围，尺寸保持',()=>{
  expect(publicationTransform(page,box,10.123,4.567)).toEqual({...box,xMm:25.12,yMm:24.57});expect(publicationTransform(page,box,-999,999)).toEqual({...box,xMm:-3,yMm:203});
});
it('四个角固定对角锚点，限制最小尺寸与边界',()=>{
  expect(publicationTransform(page,box,10,20,'nw')).toEqual({xMm:25,yMm:40,widthMm:70,heightMm:80});expect(publicationTransform(page,box,10,20,'se')).toEqual({...box,widthMm:90,heightMm:120});
  const tiny=publicationTransform(page,box,-999,-999,'se')!;expect(tiny.widthMm).toBe(1);expect(tiny.heightMm).toBe(1);expect(publicationRectangleValid(page,tiny)).toBe(true);
});
it('按比例缩放仍符合边界，不把非有限或越界起点变成有效操作',()=>{
  const resized=publicationTransform(page,box,20,10,'se',true)!;expect(resized.widthMm/resized.heightMm).toBeCloseTo(.8);expect(publicationRectangleValid(page,resized)).toBe(true);
  expect(publicationTransform(page,box,NaN,0)).toBeUndefined();expect(publicationTransform(page,{...box,xMm:-4},0,0)).toBeUndefined();expect(publicationTransform(page,{...box,widthMm:0},0,0)).toBeUndefined();
});
it('不同页面尺寸和大幅拖动下保持有限、有界和对角锚点',()=>{
  for(let i=0;i<250;i++){const p={widthMm:50+i*.731,heightMm:65+i*.43,bleedMm:3.1},b={xMm:5,yMm:7,widthMm:p.widthMm/3,heightMm:p.heightMm/4};for(const corner of ['nw','ne','se','sw'] as PublicationCorner[]){for(const aspect of [false,true]){const value=publicationTransform(p,b,(i-125)*13.37,(100-i)*9.71,corner,aspect);expect(value).toBeDefined();expect(publicationRectangleValid(p,value!)).toBe(true);if(aspect)expect(value!.widthMm/value!.heightMm).toBeCloseTo(b.widthMm/b.heightMm);}}}
});
