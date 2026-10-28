import { publicationStoredTextDirection } from '../domain/publicationTypography';
import { checkPublicationCover } from '../domain/publicationCover';
import { z } from 'zod';
import { convertFileSrc } from '@tauri-apps/api/core';
import { zh } from '../app/copy';
import { publicationMasterSchema,publicationMasterLinkSchema } from '@betweenframes/contracts';
import { publicationPageGuides } from '../domain/publicationGuides';
import { publicationChapters } from '../domain/publicationChapters';
import { publicationLayers } from '../domain/publicationLayers';
import { publicationContinuousFrames } from '../domain/publicationContinuous';
const finite=z.number().finite(),hash=z.string().regex(/^[a-f0-9]{64}$/),identifier=z.string().min(1).max(160);
const position={xMm:finite,yMm:finite,widthMm:finite.positive(),heightMm:finite.positive()};
export const sourceSchema=z.object({assetId:identifier,sourceSha256:hash,width:z.number().int().nonnegative(),height:z.number().int().nonnegative(),sourceProfileDigest:z.string().nullable(),renditionId:identifier.nullable(),variantId:identifier.nullable(),renditionSha256:hash.nullable(),renditionWidth:z.number().int().positive().nullable(),renditionHeight:z.number().int().positive().nullable(),bitDepth:z.number().int().positive().nullable(),iccDigest:z.string().nullable(),previewSha256:hash.nullable()});
export const publicationLayoutSchema=z.object({schemaVersion:z.union([z.literal(1),z.literal(2),z.literal(3)]),publication:z.object({id:identifier,projectId:identifier,title:z.string(),pubType:z.enum(['photobook','exhibition','portfolio']),specJson:z.string().max(1024*1024),state:z.enum(['draft','in_review','finalized','delivered'])}),pages:z.array(z.object({id:identifier,pageNumber:z.number().int(),side:z.enum(['left','right','spread','cover','back']),widthMm:finite.min(50).max(2000),heightMm:finite.min(50).max(2000),marginMm:finite.nonnegative(),bleedMm:finite.min(0).max(25),layoutJson:z.string().nullable(),frames:z.array(z.object({id:identifier,...position,fitMode:z.enum(['fit','fill','stretch']),assetId:identifier.nullable(),renditionId:identifier.nullable(),cropX:finite.min(0).max(1),cropY:finite.min(0).max(1),cropZoom:finite.min(1).max(20),source:sourceSchema.nullable()})).max(4096),texts:z.array(z.object({id:identifier,...position,contentJson:z.string().max(32768),styleJson:z.string().max(8192).nullable()})).max(4096)})).max(512)});
export const publicationVersionSchema=z.object({id:identifier,publicationId:identifier,revision:z.number().int().positive(),label:z.string(),kind:z.enum(['working','saved']),snapshotSha256:hash,createdAt:z.string()});
export const publicationPreviewSchema=z.array(z.object({frameId:identifier,thumbnailPath:z.string().nullable(),photoId:identifier.nullable().optional()})).max(4096);
export const publicationMasterStatesSchema=z.array(z.object({pageId:identifier,masterId:identifier,appliedRevision:z.number().int().positive(),currentRevision:z.number().int().positive().nullable(),locallyChanged:z.boolean()})).max(512);
export function checkPublicationScope(layout:z.infer<typeof publicationLayoutSchema>,projectId:string,publicationId:string){
  const ids=new Set<string>(),numbers=new Set<number>();let count=0;
  if(layout.publication.projectId!==projectId||layout.publication.id!==publicationId)throw new Error(zh.publicationScopeMismatch);
  const spec=JSON.parse(layout.publication.specJson);z.array(publicationMasterSchema).max(16).parse(spec.focalroomPageMastersV1??[]);
  for(const page of layout.pages){publicationPageGuides(page);if(ids.has(page.id)||numbers.has(page.pageNumber))throw new Error(zh.publicationUpdated);ids.add(page.id);numbers.add(page.pageNumber);
    if(page.layoutJson){const link=JSON.parse(page.layoutJson)?.focalroomPageMasterV1;if(link)publicationMasterLinkSchema.parse(link);}
    for(const frame of page.frames){if(ids.has(frame.id)||(frame.source?.assetId??null)!==frame.assetId||(frame.source?.renditionId??null)!==frame.renditionId)throw new Error(zh.publicationPreviewFailed);ids.add(frame.id);count++;}
    for(const text of page.texts){publicationStoredTextDirection(text.styleJson);if(ids.has(text.id))throw new Error(zh.publicationUpdated);ids.add(text.id);count++;}
  }if(count>4096)throw new Error(zh.publicationLoadFailed);publicationContinuousFrames(layout);publicationLayers(layout);publicationChapters(layout);checkPublicationCover(layout);
}
export function publicationPreviews(layout:z.infer<typeof publicationLayoutSchema>,previews:z.infer<typeof publicationPreviewSchema>){
  const frames=new Map(layout.pages.flatMap(p=>p.frames.map(f=>[f.id,f] as const))),seen=new Set<string>();
  if(frames.size!==previews.length)throw new Error(zh.publicationPreviewFailed);
  return previews.map(preview=>{const frame=frames.get(preview.frameId);if(!frame||seen.has(preview.frameId))throw new Error(zh.publicationPreviewFailed);seen.add(preview.frameId);
    return {frameId:preview.frameId,thumbnailSrc:publicationThumbnail(frame.source,preview.thumbnailPath),photoId:preview.photoId??undefined};
  });
}
export function publicationThumbnail(source:z.infer<typeof sourceSchema>|null,thumbnailPath:string|null){
  if(!thumbnailPath)return undefined;const path=thumbnailPath.split('\\').join('/'),hash=source?.renditionSha256??source?.sourceSha256;
  if(!hash||!source?.previewSha256||(!path.startsWith('/')&&!/^[A-Za-z]:\//.test(path))||!path.endsWith('/'+hash+'-320.jpg'))throw new Error(zh.publicationPreviewFailed);
  return convertFileSrc(thumbnailPath);
}
