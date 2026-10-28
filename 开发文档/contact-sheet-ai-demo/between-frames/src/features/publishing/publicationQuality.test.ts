import { expect,it } from 'vitest';
import type { PublicationLayoutFrame } from '../../domain/publication';
import { publicationEffectivePpi } from './publicationQuality';
const frame:PublicationLayoutFrame={id:'f',xMm:0,yMm:0,widthMm:200,heightMm:200,fitMode:'fit',cropX:.5,cropY:.5,cropZoom:1,assetId:'a',renditionId:null,source:{assetId:'a',sourceSha256:'hash',width:2000,height:1000,sourceProfileDigest:null,renditionId:null,variantId:null,renditionSha256:null,renditionWidth:null,renditionHeight:null,bitDepth:null,iccDigest:null,previewSha256:null}};
it('按源像素、实际适配尺寸和放大计算，不使用照片框像素或缩略图尺寸',()=>{
  expect(publicationEffectivePpi(frame)).toBeCloseTo(254);expect(publicationEffectivePpi({...frame,fitMode:'fill'})).toBeCloseTo(127);expect(publicationEffectivePpi({...frame,fitMode:'fill',cropZoom:2})).toBeCloseTo(63.5);
});
it('指定图像版本尺寸缺失不回退为原图，已知版本则使用版本尺寸',()=>{
  expect(publicationEffectivePpi({...frame,renditionId:'r',source:{...frame.source!,renditionId:'r'}})).toBeUndefined();expect(publicationEffectivePpi({...frame,renditionId:'r',source:{...frame.source!,renditionId:'r',renditionWidth:1000,renditionHeight:500}})).toBeCloseTo(127);
});
