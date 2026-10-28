import { expect,it } from 'vitest';
import type { PublicationLayout } from '../../domain/publication';
import { publicationArrange,publicationArrangementPreview,publicationArrangementUpdates,publicationPageObjects,publicationSelectionBounds,publicationSelectionObjects,publicationTransformSelection } from './publicationArrangement';
import { publicationRectangleValid,publicationTransform,type PublicationCorner } from './publicationGeometry';
const frame={id:'photo',xMm:15,yMm:20,widthMm:40,heightMm:50,fitMode:'fill' as const,cropX:.3,cropY:.7,cropZoom:1.5,assetId:'asset',renditionId:'render',source:null};
const page:PublicationLayout['pages'][number]={id:'page',pageNumber:1,side:'right',widthMm:210,heightMm:297,marginMm:15,bleedMm:3,layoutJson:null,frames:[frame,{...frame,id:'other',xMm:135,yMm:90,widthMm:20}],texts:[{id:'text',xMm:80,yMm:140,widthMm:30,heightMm:20,contentJson:'{"kind":"text","text":"原文","extra":true}',styleJson:'{"fontSizePt":17,"custom":true}'}]};
const objects=publicationPageObjects(page);
it('选择必须完整、唯一并属于同页，不能丢掉缺失对象继续保存',()=>{
  expect(publicationSelectionObjects(page,objects)).toEqual(objects);expect(publicationSelectionObjects(page,[objects[0],objects[0]])).toBeUndefined();expect(publicationSelectionObjects(page,[objects[0],{kind:'text',id:'foreign'}])).toBeUndefined();expect(publicationSelectionObjects(page,Array.from({length:101},()=>objects[0]))).toBeUndefined();
});
it('整组移动保持相对位置，恢复起点不累计漂移；框缩放不改照片裁切或正文',()=>{
  const bounds=publicationSelectionBounds(objects)!,moved=publicationTransformSelection(page,objects,{...bounds,xMm:bounds.xMm+10,yMm:bounds.yMm+12})!;expect(moved.map(item=>[item.xMm,item.yMm])).toEqual(objects.map(item=>[item.xMm+10,item.yMm+12]));expect(publicationTransformSelection(page,objects,bounds)).toEqual(objects);
  const scaled=publicationTransformSelection(page,objects,{...bounds,widthMm:bounds.widthMm/2,heightMm:bounds.heightMm/2})!,preview=publicationArrangementPreview(page,scaled),changes=publicationArrangementUpdates(page,scaled)!;expect(preview.frames[0].cropZoom).toBe(1.5);expect(preview.frames[0].renditionId).toBe('render');expect(preview.texts[0].contentJson).toBe(page.texts[0].contentJson);expect(preview.texts[0].styleJson).toBe(page.texts[0].styleJson);expect(changes.texts[0]).not.toHaveProperty('contentJson');expect(changes.frames[0]).toMatchObject({assetId:'asset',renditionId:'render',cropX:.3,cropY:.7,cropZoom:1.5});expect(publicationArrangementUpdates(page,objects)).toEqual({frames:[],texts:[]});
});
it('六种对齐参照明确，保持尺寸；超出页面时整组拒绝而不裁小框',()=>{
  const left=publicationArrange(page,objects,'left','selection')!;expect(left.every(item=>item.xMm===15)).toBe(true);const right=publicationArrange(page,objects,'right','page')!;expect(right.every(item=>item.xMm+item.widthMm===195)).toBe(true);const center=publicationArrange(page,objects,'centerY','page')!;expect(center.every(item=>item.yMm+item.heightMm/2===148.5)).toBe(true);for(let i=0;i<objects.length;i++){expect(right[i].widthMm).toBe(objects[i].widthMm);expect(right[i].heightMm).toBe(objects[i].heightMm);}expect(publicationArrange(page,[{...objects[0],xMm:-3,widthMm:210},objects[1]],'left','page')).toBeUndefined();
});
it('分布按位置稳定排序并均分边缘间隔，范围不足不制造重叠',()=>{
  const shuffled=[objects[2],objects[0],objects[1]],spread=publicationArrange(page,shuffled,'distributeX','page')!;const sorted=[...spread].sort((a,b)=>a.xMm-b.xMm);expect(sorted[0].xMm).toBe(15);expect(sorted[2].xMm+sorted[2].widthMm).toBe(195);expect(sorted[1].xMm-sorted[0].xMm-sorted[0].widthMm).toBeCloseTo(sorted[2].xMm-sorted[1].xMm-sorted[1].widthMm);expect(spread.map(item=>item.id)).toEqual(shuffled.map(item=>item.id));expect(publicationArrange(page,objects.slice(0,2),'distributeX','page')).toBeUndefined();expect(publicationArrange(page,objects.map(item=>({...item,xMm:15,widthMm:100})),'distributeX','page')).toBeUndefined();
});
it('小数边界、比例和极端位移的整组缩放保持全部对象有效，不能部分通过',()=>{
  for(let i=0;i<100;i++){const bounds=publicationSelectionBounds(objects)!;for(const corner of ['nw','ne','se','sw'] as PublicationCorner[]){for(const aspect of [true,false]){const target=publicationTransform(page,bounds,(i-50)*17.31,(35-i)*9.37,corner,aspect)!;const result=publicationTransformSelection(page,objects,target);expect(result).toHaveLength(objects.length);expect(result!.every(item=>publicationRectangleValid(page,item))).toBe(true);}}}expect(publicationTransformSelection(page,objects,{xMm:-999,yMm:0,widthMm:100,heightMm:100})).toBeUndefined();
});
