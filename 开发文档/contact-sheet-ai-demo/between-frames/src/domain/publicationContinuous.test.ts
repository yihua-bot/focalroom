import { expect,it } from 'vitest';
import { continuousFixture } from '../test/publicationContinuousFixture';
import { CONTINUOUS_FRAME_KEY,publicationContinuousFrames,publicationLinkPreview,publicationUnlinkPreview,publicationOrdinaryFrames,publicationVisibleFrames,publicationVisiblePhotoIds,publicationContinuousWindow } from './publicationContinuous';


it('跨页只有一份正式对象和来源，右页仅引用同一对象，原布局和旧设置保持',()=>{
  const {layout,data}=continuousFixture(),before=structuredClone(layout),preview=publicationLinkPreview(layout,data),items=publicationContinuousFrames(preview);expect(items).toHaveLength(1);expect(preview.schemaVersion).toBe(2);expect(preview.pages[3].frames).toEqual([]);expect(preview.pages.flatMap(page=>page.frames).filter(frame=>frame.id===data.frameId)).toHaveLength(1);expect(items[0].frame.source).toEqual(layout.pages[3].frames[0].source);expect(publicationVisibleFrames(preview.pages[3],items)[0]).toBe(items[0].frame);expect(publicationOrdinaryFrames(preview.pages[2])).toEqual(layout.pages[2].frames);expect(JSON.parse(preview.pages[2].layoutJson!).legacy).toEqual({keep:[1,2]});expect(layout).toEqual(before);expect(preview.pages[1]).toEqual(layout.pages[1]);
});
it('转成单页保留正式身份与图像版本，显式采用新几何，不遗留右页投影或关联',()=>{
  const {layout,data}=continuousFixture(),linked=publicationLinkPreview(layout,data),source=publicationContinuousFrames(linked)[0].frame;const preview=publicationUnlinkPreview(linked,{frameId:data.frameId,targetPageId:data.rightPageId,geometry:{xMm:20,yMm:25,widthMm:140,heightMm:190,fitMode:'fit',cropX:.5,cropY:.5,cropZoom:1}});expect(preview.schemaVersion).toBe(1);expect(publicationContinuousFrames(preview)).toEqual([]);expect(preview.pages[3].frames[0]).toMatchObject({id:source.id,source:source.source,assetId:source.assetId,renditionId:source.renditionId,xMm:20,widthMm:140});expect(preview.pages[2].frames).toEqual(layout.pages[2].frames);expect(JSON.parse(preview.pages[2].layoutJson!).legacy).toEqual({keep:[1,2]});expect(JSON.parse(preview.pages[2].layoutJson!)[CONTINUOUS_FRAME_KEY]).toBeUndefined();
});
it('只有相邻的实体左右页可配对，丢失对象、失配来源、重复照片与旧格式不能冒充连续布局',()=>{
  const {layout,data}=continuousFixture();expect(()=>publicationLinkPreview(layout,{...data,leftPageId:layout.pages[1].id})).toThrow();expect(()=>publicationLinkPreview(layout,{...data,rightPageId:layout.pages[0].id})).toThrow();expect(()=>publicationLinkPreview(layout,{...data,geometry:{...data.geometry,widthMm:100}})).toThrow();
  const linked=publicationLinkPreview(layout,data);for(const mode of ['old','missing','source','duplicate','size','extra']as const){const bad=structuredClone(linked);const frame=bad.pages[2].frames.find(frame=>frame.id===data.frameId)!;if(mode==='old')bad.schemaVersion=1;if(mode==='missing')bad.pages[2].frames=bad.pages[2].frames.filter(frame=>frame.id!==data.frameId);if(mode==='source')frame.source!.renditionId='other';if(mode==='duplicate')bad.pages[3].frames.push({...frame,id:crypto.randomUUID(),xMm:15,widthMm:160});if(mode==='size')bad.pages[3].widthMm=200;if(mode==='extra'){const meta=JSON.parse(bad.pages[2].layoutJson!);meta[CONTINUOUS_FRAME_KEY][0].untrusted='ignored';bad.pages[2].layoutJson=JSON.stringify(meta);}expect(()=>publicationContinuousFrames(bad)).toThrow();}
});
it('投影的照片来源按真实photoId解析，不以资产编号充当照片编号；旧设置仍可只读',()=>{
  const {layout,data}=continuousFixture(),linked=publicationLinkPreview(layout,data),items=publicationContinuousFrames(linked);expect(publicationVisiblePhotoIds(linked.pages[3],items,[{frameId:data.frameId,photoId:'actual-photo-id'}])).toEqual(['actual-photo-id']);expect(publicationVisiblePhotoIds(linked.pages[3],items,[{frameId:data.frameId}])).toEqual([]);layout.pages[2].layoutJson=JSON.stringify({legacy:'x'.repeat(70*1024)});expect(publicationContinuousFrames(layout)).toEqual([]);expect(()=>publicationLinkPreview(layout,data)).toThrow();
});

it('左右视窗在装订线采用相同照片坐标，局部框不会越过纸张或复制裁切参数',()=>{
  const {layout,data}=continuousFixture(),linked=publicationLinkPreview(layout,data),item=publicationContinuousFrames(linked)[0],left=publicationContinuousWindow(item.left,item.frame,0)!,right=publicationContinuousWindow(item.right,item.frame,item.left.widthMm)!;
  expect(left.rectangle).toEqual({xMm:15,yMm:20,widthMm:175,heightMm:210});expect(right.rectangle).toEqual({xMm:0,yMm:20,widthMm:175,heightMm:210});expect(left.image).toEqual({left:0,top:0,width:200,height:100});expect(right.image).toEqual({left:-100,top:0,width:200,height:100});
  const coordinate=(window:typeof left,point:number)=>((point-window.rectangle.xMm)/window.rectangle.widthMm*100-window.image.left)/window.image.width;
  expect(coordinate(left,190)).toBeCloseTo(coordinate(right,0),12);expect(item.frame.cropX).toBe(.2);expect(item.frame.cropZoom).toBe(1.6);
});
