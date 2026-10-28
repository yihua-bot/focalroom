import { expect,it,vi } from 'vitest';
vi.mock('@tauri-apps/api/core',()=>({convertFileSrc:(path:string)=>'asset:'+path}));
import { parsePublicationPhotoVersions } from './publicationPhotoVersions';
import type { PublicationLayoutFrame } from '../domain/publication';
const source={assetId:'asset',sourceSha256:'a'.repeat(64),width:4000,height:3000,sourceProfileDigest:null,renditionId:null,variantId:null,renditionSha256:null,renditionWidth:null,renditionHeight:null,bitDepth:null,iccDigest:null,previewSha256:'c'.repeat(64)};
const frame:PublicationLayoutFrame={id:'frame',xMm:15,yMm:15,widthMm:100,heightMm:150,fitMode:'fill',cropX:.5,cropY:.5,cropZoom:1,assetId:'asset',renditionId:null,source};
const original={renditionId:null,variantName:null,externalSoftware:null,createdAt:null,source,thumbnailPath:'/cache/'+source.sourceSha256+'-320.jpg'};
const choice={...original,renditionId:crypto.randomUUID(),variantName:'冷调',createdAt:'2026-10-25',source:{...source,renditionId:null as string|null,variantId:'v',renditionSha256:'b'.repeat(64),renditionWidth:2000,renditionHeight:1500},thumbnailPath:'/cache/'+'b'.repeat(64)+'-320.jpg'};choice.source.renditionId=choice.renditionId;
const page={revision:2,frameId:'frame',assetId:'asset',original,current:original,choices:[choice],next:null};
it('目录绑定版面、照片和指定版本，拒绝伪造路径、来源变化与重复版本',()=>{
  expect(parsePublicationPhotoVersions(page,2,frame).choices[0].thumbnailSrc).toBe('asset:'+choice.thumbnailPath);
  for(const invalid of [{...page,revision:1},{...page,assetId:'other'},{...page,frameId:'other'},{...page,current:{...original,source:{...source,width:999}}},{...page,choices:[choice,choice]},{...page,choices:[{...choice,thumbnailPath:original.thumbnailPath}]},{...page,choices:[{...choice,source:{...choice.source,assetId:'other'}}]}])expect(()=>parsePublicationPhotoVersions(invalid,2,frame)).toThrow();
});
it('分页不得倒退到新结果或伪造下一页，缺失预览不回退原片',()=>{
  expect(parsePublicationPhotoVersions({...page,choices:[{...choice,thumbnailPath:null}]},2,frame).choices[0].thumbnailSrc).toBeUndefined();
  expect(()=>parsePublicationPhotoVersions(page,2,frame,{createdAt:'2026-10-24',id:crypto.randomUUID()})).toThrow();expect(()=>parsePublicationPhotoVersions({...page,next:{createdAt:choice.createdAt,id:choice.renditionId}},2,frame)).toThrow();
});
