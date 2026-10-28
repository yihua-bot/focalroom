import { expect,it } from 'vitest';
import type { Photo } from '../../domain/models';
import type { PublicationLayout } from '../../domain/publication';
import { publicationPhotoLayout } from './publicationPhotoLayout';
const page:PublicationLayout['pages'][number]={id:'page',pageNumber:1,side:'right',widthMm:190,heightMm:257,marginMm:15,bleedMm:3,layoutJson:null,texts:[{id:'text',xMm:10,yMm:220,widthMm:80,heightMm:10,contentJson:'{"text":"不移动文字"}',styleJson:null}],frames:[{id:'old',xMm:15,yMm:15,widthMm:160,heightMm:227,fitMode:'fill',cropX:.2,cropY:.7,cropZoom:1.6,assetId:'asset',renditionId:'render',source:null}]};
const photos:Photo[]=['A','B'].map((id,index)=>({id,index,name:id,src:id+'.jpg',status:'undecided',rating:0,note:''}));
it('预览与保存使用同一几何，保留已有照片版本和裁切，不移动文字或原图',()=>{
  const before=structuredClone(page),photoBefore=structuredClone(photos);let i=0;const result=publicationPhotoLayout(page,photos,0,()=>`new-${i++}`);
  expect(result.updates[0]).toMatchObject({frameId:'old',assetId:'asset',renditionId:'render',fitMode:'fill',cropX:.2,cropY:.7,cropZoom:1.6});
  expect(result.additions.map(f=>f.photoId)).toEqual(['A','B']);expect(result.additions.every(f=>f.renditionId===null)).toBe(true);
  expect(result.preview.texts).toEqual(page.texts);for(const frame of result.preview.frames){expect(frame.xMm).toBeGreaterThanOrEqual(15);expect(frame.yMm).toBeGreaterThanOrEqual(15);expect(frame.xMm+frame.widthMm).toBeLessThanOrEqual(175);expect(frame.yMm+frame.heightMm).toBeLessThanOrEqual(242);}
  expect(result.preview.frames[1]).toMatchObject({id:result.additions[0].frameId,xMm:result.additions[0].xMm,yMm:result.additions[0].yMm,widthMm:result.additions[0].widthMm,heightMm:result.additions[0].heightMm});expect(page).toEqual(before);expect(photos).toEqual(photoBefore);
});
it('拒绝超过单次命令容量的版面，不生成无法提交的半份计划',()=>{
  const many={...page,frames:Array.from({length:100},(_,i)=>({...page.frames[0],id:String(i)}))};expect(()=>publicationPhotoLayout(many,photos,2,()=>crypto.randomUUID())).toThrow();
});
it('重排保留已有照片的阅读顺序，不使用内部编号顺序',()=>{
  const unordered={...page,frames:[{...page.frames[0],id:'a',xMm:100,yMm:100},{...page.frames[0],id:'z',xMm:15,yMm:15},{...page.frames[0],id:'b',xMm:100,yMm:15}]};
  const result=publicationPhotoLayout(unordered,[photos[0]],2,()=>crypto.randomUUID());expect(result.updates.map(f=>f.frameId)).toEqual(['z','b','a']);expect(unordered.frames.map(f=>f.id)).toEqual(['a','z','b']);
});
it('空母版照片框按原位置填入，既有照片、文字和布局不重新排列',()=>{
  const blank={...page.frames[0],id:'blank',assetId:null,renditionId:null,source:null,xMm:100,yMm:100};const target={...page,frames:[page.frames[0],blank]};
  const result=publicationPhotoLayout(target,[photos[0]],0,()=>{throw new Error('不应创建新的照片框');});expect(result.updates).toEqual([]);expect(result.additions).toEqual([]);expect(result.assignments).toEqual([{frameId:'blank',photoId:'A',renditionId:null}]);expect(result.preview.frames[0]).toEqual(page.frames[0]);expect(result.preview.frames[1].xMm).toBe(100);expect(result.preview.texts).toEqual(page.texts);
});

it('新增照片排版保留分组、隐藏或锁定照片框的位置和裁切，不把受保护空框填掉',()=>{
 const protectedEmpty={...page.frames[0],id:'protected-empty',assetId:null,renditionId:null,source:null,xMm:80};
 const source={...page,frames:[page.frames[0],protectedEmpty]};const before=structuredClone(source),result=publicationPhotoLayout(source,photos,2,()=>crypto.randomUUID(),new Set(['old','protected-empty']));
 expect(result.updates).toEqual([]);expect(result.assignments).toEqual([]);expect(result.additions).toHaveLength(2);expect(result.preview.frames.slice(0,2)).toEqual(source.frames);expect(source).toEqual(before);
});
