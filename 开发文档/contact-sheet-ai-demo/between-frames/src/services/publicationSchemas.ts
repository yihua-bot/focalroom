import { z } from 'zod';
import { convertFileSrc } from '@tauri-apps/api/core';
const finite=z.number().finite(),hash=z.string().regex(/^[a-f0-9]{64}$/),identifier=z.string().min(1).max(160);
const position={xMm:finite,yMm:finite,widthMm:finite.positive(),heightMm:finite.positive()};
const sourceSchema=z.object({assetId:identifier,sourceSha256:hash,width:z.number().int().nonnegative(),height:z.number().int().nonnegative(),sourceProfileDigest:z.string().nullable(),renditionId:identifier.nullable(),variantId:identifier.nullable(),renditionSha256:hash.nullable(),renditionWidth:z.number().int().positive().nullable(),renditionHeight:z.number().int().positive().nullable(),bitDepth:z.number().int().positive().nullable(),iccDigest:z.string().nullable(),previewSha256:hash.nullable()});
export const publicationLayoutSchema=z.object({schemaVersion:z.literal(1),publication:z.object({id:identifier,projectId:identifier,title:z.string(),pubType:z.enum(['photobook','exhibition','portfolio']),specJson:z.string(),state:z.enum(['draft','in_review','finalized','delivered'])}),pages:z.array(z.object({id:identifier,pageNumber:z.number().int(),side:z.enum(['left','right','spread','cover','back']),widthMm:finite.min(50).max(2000),heightMm:finite.min(50).max(2000),marginMm:finite.nonnegative(),bleedMm:finite.min(0).max(25),layoutJson:z.string().nullable(),frames:z.array(z.object({id:identifier,...position,fitMode:z.enum(['fit','fill','stretch']),assetId:identifier.nullable(),renditionId:identifier.nullable(),cropX:finite.min(0).max(1),cropY:finite.min(0).max(1),cropZoom:finite.min(1).max(20),source:sourceSchema.nullable()})).max(4096),texts:z.array(z.object({id:identifier,...position,contentJson:z.string().max(32768),styleJson:z.string().max(8192).nullable()})).max(4096)})).max(512)});
export const publicationVersionSchema=z.object({id:identifier,publicationId:identifier,revision:z.number().int().positive(),label:z.string(),kind:z.enum(['working','saved']),snapshotSha256:hash,createdAt:z.string()});
export const publicationPreviewSchema=z.array(z.object({frameId:identifier,thumbnailPath:z.string().nullable()})).max(4096);
export function checkPublicationScope(layout:z.infer<typeof publicationLayoutSchema>,projectId:string,publicationId:string){
  const ids=new Set<string>(),numbers=new Set<number>();let count=0;
  if(layout.publication.projectId!==projectId||layout.publication.id!==publicationId)throw new Error('无法打开这份出版物。');
  for(const page of layout.pages){if(ids.has(page.id)||numbers.has(page.pageNumber))throw new Error('页面已变化，请重新打开。');ids.add(page.id);numbers.add(page.pageNumber);
    for(const frame of page.frames){if(ids.has(frame.id)||(frame.source?.assetId??null)!==frame.assetId||(frame.source?.renditionId??null)!==frame.renditionId)throw new Error('照片版本不匹配，请重新打开。');ids.add(frame.id);count++;}
    for(const text of page.texts){if(ids.has(text.id))throw new Error('页面内容已变化，请重新打开。');ids.add(text.id);count++;}
  }if(count>4096)throw new Error('版面内容过多，请拆分为多个分册。');
}
export function publicationPreviews(layout:z.infer<typeof publicationLayoutSchema>,previews:z.infer<typeof publicationPreviewSchema>){
  const frames=new Map(layout.pages.flatMap(p=>p.frames.map(f=>[f.id,f] as const))),seen=new Set<string>();
  if(frames.size!==previews.length)throw new Error('版面预览不完整，请重新打开。');
  return previews.map(preview=>{const frame=frames.get(preview.frameId);if(!frame||seen.has(preview.frameId))throw new Error('版面预览不匹配，请重新打开。');seen.add(preview.frameId);
    if(preview.thumbnailPath){const path=preview.thumbnailPath.split('\\').join('/'),hash=frame.source?.renditionSha256??frame.source?.sourceSha256;
      if(!hash||!frame.source?.previewSha256||(!path.startsWith('/')&&!/^[A-Za-z]:\//.test(path))||!path.endsWith('/'+hash+'-320.jpg'))throw new Error('图像版本预览不匹配，请重新打开。');
    }
    return {frameId:preview.frameId,thumbnailSrc:preview.thumbnailPath?convertFileSrc(preview.thumbnailPath):undefined};
  });
}
