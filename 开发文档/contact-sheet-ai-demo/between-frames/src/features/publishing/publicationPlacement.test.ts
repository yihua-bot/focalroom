import { expect,it } from 'vitest';
import type { Photo } from '../../domain/models';
import type { PublicationLayout } from '../../domain/publication';
import { publicationPlacement } from './publicationPlacement';
import { publicationRectangleValid } from './publicationGeometry';
const page:PublicationLayout['pages'][number]={id:'page',pageNumber:1,side:'right',widthMm:210,heightMm:297,marginMm:15,bleedMm:3,layoutJson:'{"master":"keep"}',frames:[{id:'existing',assetId:'asset',renditionId:'render',xMm:15,yMm:15,widthMm:90,heightMm:70,cropX:.2,cropY:.7,cropZoom:1.8,fitMode:'fill',source:null}],texts:[{id:'text',xMm:20,yMm:200,widthMm:80,heightMm:25,contentJson:'{"text":"原文"}',styleJson:'{"custom":true}'}]};
const photo:Photo={id:'photo',index:1,name:'街道',src:'photo.jpg',rating:0,status:'keep',note:'',width:6000,height:4000};
it('新增框使用明确照片和原片版本，不重排任何已有内容，极端落点夹在边距内',()=>{
  const before=structuredClone(page),result=publicationPlacement(page,photo,{xMm:-100,yMm:500},()=> 'new')!;expect(result.additions).toHaveLength(1);expect(result.assignments).toEqual([]);expect(result.additions[0]).toMatchObject({frameId:'new',pageId:'page',photoId:'photo',renditionId:null,xMm:15});expect(result.preview.frames[0]).toEqual(page.frames[0]);expect(result.preview.texts).toEqual(page.texts);expect(result.preview.layoutJson).toBe(page.layoutJson);expect(result.preview.frames[1].widthMm/result.preview.frames[1].heightMm).toBeCloseTo(1.5);expect(result.preview.frames[1].yMm+result.preview.frames[1].heightMm).toBeLessThanOrEqual(282);expect(page).toEqual(before);
});
it('填入指定空框保持原几何与其他框的版本和裁切，已填/失效目标不被静默覆盖',()=>{
  const empty={...page.frames[0],id:'blank',assetId:null,renditionId:null},source={...page,frames:[page.frames[0],empty]};const result=publicationPlacement(source,photo,{xMm:30,yMm:30},()=> 'unused','blank')!;expect(result.additions).toEqual([]);expect(result.assignments).toEqual([{frameId:'blank',photoId:'photo',renditionId:null}]);expect(result.preview.frames[0]).toEqual(page.frames[0]);expect(result.preview.frames[1]).toEqual({...empty,cropX:.5,cropY:.5,cropZoom:1});expect(publicationPlacement(source,photo,{xMm:30,yMm:30},()=> 'new','existing')).toBeUndefined();expect(publicationPlacement(source,photo,{xMm:30,yMm:30},()=> 'new','foreign')).toBeUndefined();
});
it('无空间、非有限坐标、重复框身份与满页拒绝；不同开本及照片比例仍全部有界',()=>{
  expect(publicationPlacement(page,photo,{xMm:NaN,yMm:1},()=> 'new')).toBeUndefined();expect(publicationPlacement(page,photo,{xMm:1,yMm:1},()=> 'existing')).toBeUndefined();expect(publicationPlacement({...page,marginMm:150},photo,{xMm:1,yMm:1},()=> 'new')).toBeUndefined();expect(publicationPlacement({...page,frames:Array.from({length:100},(_,index)=>({...page.frames[0],id:String(index)}))},photo,{xMm:1,yMm:1},()=> 'new')).toBeUndefined();for(let i=1;i<=100;i++){const source={...page,widthMm:40+i*3.17,heightMm:60+i*2.31,marginMm:5};for(const ratio of [.01,.6,1.5,100]){const result=publicationPlacement(source,{...photo,width:ratio*1000,height:1000},{xMm:i*13-300,yMm:500-i*8},()=> 'new')!;expect(result.preview.frames).toHaveLength(2);expect(publicationRectangleValid(source,result.preview.frames[1])).toBe(true);}}
});
