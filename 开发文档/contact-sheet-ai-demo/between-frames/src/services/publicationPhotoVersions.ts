import { z } from 'zod';
import type { PublicationLayoutFrame,PublicationPhotoVersionCursor } from '../domain/publication';
import { sourceSchema,publicationThumbnail } from './publicationSchemas';
import { zh } from '../app/copy';
const identifier=z.string().min(1).max(160);
const cursorSchema=z.object({createdAt:z.string().min(1).max(80),id:z.uuid()});
const choiceSchema=z.object({renditionId:identifier.nullable(),variantName:z.string().nullable(),externalSoftware:z.string().nullable(),createdAt:z.string().nullable(),source:sourceSchema,thumbnailPath:z.string().nullable()});
const pageSchema=z.object({revision:z.number().int().positive(),frameId:identifier,assetId:identifier,original:choiceSchema,current:choiceSchema,choices:z.array(choiceSchema).max(50),next:cursorSchema.nullable()});
export function parsePublicationPhotoVersions(value:unknown,revision:number,frame:PublicationLayoutFrame,cursor?:PublicationPhotoVersionCursor){
  const result=pageSchema.parse(value),seen=new Set<string>();
  if(result.revision!==revision||result.frameId!==frame.id||result.assetId!==frame.assetId||result.original.renditionId!==null||result.current.renditionId!==frame.renditionId)throw new Error(zh.publicationUpdated);
  if(!frame.source||Object.entries(frame.source).some(([field,value])=>field!=='previewSha256'&&result.current.source[field as keyof typeof result.current.source]!==value))throw new Error(zh.publicationUpdated);
  const choices=result.choices;
  for(let i=0;i<choices.length;i++){
    const item=choices[i];if(!item.renditionId||!item.createdAt||seen.has(item.renditionId))throw new Error(zh.publicationPreviewFailed);seen.add(item.renditionId);
    const prior=i>0?{id:choices[i-1].renditionId!,createdAt:choices[i-1].createdAt!}:cursor;
    if(prior&&(item.createdAt>prior.createdAt||(item.createdAt===prior.createdAt&&item.renditionId>=prior.id)))throw new Error(zh.publicationUpdated);
  }
  const last=choices[choices.length-1];if(result.next&&(!last||choices.length!==50||last.renditionId!==result.next.id||last.createdAt!==result.next.createdAt))throw new Error(zh.publicationUpdated);
  const convert=(choice:z.infer<typeof choiceSchema>)=>{
    if(choice.source.assetId!==frame.assetId||choice.source.sourceSha256!==frame.source?.sourceSha256||choice.source.renditionId!==choice.renditionId||(choice.renditionId!==null&&!choice.source.renditionSha256))throw new Error(zh.publicationPreviewFailed);
    const {thumbnailPath,...rest}=choice;return {...rest,thumbnailSrc:publicationThumbnail(choice.source,thumbnailPath)};
  };
  return {...result,original:convert(result.original),current:convert(result.current),choices:choices.map(convert)};
}
