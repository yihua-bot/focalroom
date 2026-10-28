import { expect,it } from 'vitest';
import { insertionY } from './canvasLayout';
import type { CanvasPhoto } from '../../domain/canvas';
const node=(id:string,x:number,y:number,height:number):CanvasPhoto=>({id,boardId:'board',nodeType:'photo',x,y,width:220,height,rotation:0,zOrder:1,assetId:id,renditionRef:null});
it('新内容避开相邻障碍，不改变旧内容，也不受隐藏内容和连线影响',()=>{
  const nodes=[node('a',50,50,160),node('b',50,300,160),node('far',600,100,500),{...node('hidden',50,1000,500),visible:false},{...node('line',50,0,2000),nodeType:'arrow'}];
  const before=structuredClone(nodes);
  expect(insertionY(nodes,50,50,220,160)).toBe(484);
  expect(insertionY(nodes,50,700,220,160)).toBe(700);
  expect(nodes).toEqual(before);
});
