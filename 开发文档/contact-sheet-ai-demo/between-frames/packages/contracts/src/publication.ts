import { setPublicationCoverSchema } from './publication-cover.ts';
import { z } from 'zod';
import { publicationLayersSchema } from './publication-layers.ts';
import { setPublicationPageGuidesSchema } from './publication-guides.ts';
import { savePublicationMasterSchema,applyPublicationMasterSchema } from './publication-master.ts';
import { publicationChapterSchema,publicationChaptersSchema } from './publication-chapters.ts';
const id=z.uuid();
const projectId=z.string().min(1).max(160);
const finite=z.number().finite();
const revision=z.number().int().min(0).max(9_007_199_254_739_000);
const fit=z.enum(['fit','fill','stretch']);
const position={xMm:finite,yMm:finite,widthMm:finite.positive(),heightMm:finite.positive()};
const source=z.string().min(1).max(160).nullable();
const frame={frameId:id,...position,fitMode:fit,cropX:finite.min(0).max(1),cropY:finite.min(0).max(1),cropZoom:finite.min(1).max(20),assetId:source,renditionId:source};
export const publicationFrameGeometrySchema=z.strictObject({...position,fitMode:fit,cropX:finite.min(0).max(1),cropY:finite.min(0).max(1),cropZoom:finite.min(1).max(20)});
const page={widthMm:finite.min(50).max(2000),heightMm:finite.min(50).max(2000),marginMm:finite.min(0).max(999),bleedMm:finite.min(0).max(25)};
export const publicationTextDirectionSchema=z.enum(['horizontal','vertical-rl','vertical-lr','rotate-90','rotate-270']);
export type PublicationTextDirection=z.infer<typeof publicationTextDirectionSchema>;
const text={textDirection:publicationTextDirectionSchema.optional(),textId:id,pageId:id,...position,kind:z.enum(['text','page_number']),text:z.string().max(10000),fontFamily:z.enum(['serif','sans-serif','monospace']),fontSizePt:finite.min(4).max(300),lineHeight:finite.min(1).max(3),align:z.enum(['left','center','right']),bold:z.boolean(),color:z.string().regex(/^#[\da-fA-F]{6}$/)};
const action=<T extends string,S extends z.ZodRawShape>(type:T,shape:S)=>z.strictObject({type:z.literal(type),data:z.strictObject(shape)});
export const publicationActionSchema=z.discriminatedUnion('type',[
  action('Create',{title:z.string().trim().min(1).max(200),pubType:z.enum(['photobook','exhibition','portfolio']).nullable().optional(),widthMm:finite.min(50).max(2000).nullable().optional(),heightMm:finite.min(50).max(2000).nullable().optional()}),
  action('Generate',{photosPerPage:z.union([z.literal(1),z.literal(2),z.literal(4)]),replaceExisting:z.boolean().optional()}),
  action('UpdateFrame',frame),
  z.strictObject({type:z.literal('SetPageGuides'),data:setPublicationPageGuidesSchema}),
  z.strictObject({type:z.literal('SetPublicationLayers'),data:publicationLayersSchema}),
  z.strictObject({type:z.literal('SetCover'),data:setPublicationCoverSchema}),
  action('SetChapters',{chapters:publicationChaptersSchema,pageIds:z.array(id).min(1).max(512).refine(ids=>new Set(ids).size===ids.length)}),
  action('CreateChapter',{chapter:publicationChapterSchema,pages:z.array(z.strictObject({pageId:id,afterPageId:id.nullable(),...page})).min(1).max(2),title:z.strictObject(text),startOnRight:z.boolean()}),
  action('SetSpreadFrame',{frameId:id,leftPageId:id,rightPageId:id,geometry:publicationFrameGeometrySchema}),
  action('UnsetSpreadFrame',{frameId:id,targetPageId:id,geometry:publicationFrameGeometrySchema}),
  action('AddFrame',{frameId:id,pageId:id,photoId:source.unwrap(),renditionId:source,...position}),
  action('SetFramePhoto',{frameId:id,photoId:source.unwrap(),renditionId:source}),
  action('RemoveFrame',{frameId:id}),
  action('UpdatePage',{pageId:id,...page}),
  action('AddPage',{pageId:id,afterPageId:id.nullable(),...page}),
  action('ReorderPages',{pageIds:z.array(id).min(1).max(512).refine(ids=>new Set(ids).size===ids.length),chapterChangesConfirmed:z.boolean().optional()}),
  action('RemovePage',{pageId:id}),
  action('RemovePages',{pageIds:z.array(id).min(1).max(512).refine(ids=>new Set(ids).size===ids.length),retainBlankPageId:id.nullable(),chapterId:id.nullable()}),
  action('AddText',text),
  action('UpdateText',text),
  action('UpdateTextGeometry',{textId:id,pageId:id,...position}),
  action('RemoveText',{textId:id}),
  z.strictObject({type:z.literal('SaveMaster'),data:savePublicationMasterSchema}),
  z.strictObject({type:z.literal('ApplyMaster'),data:applyPublicationMasterSchema}),
  action('CaptureVersion',{label:z.string().trim().min(1).max(120)}),
  action('RestoreVersion',{versionId:id}),
]);
export const publicationRequestSchema=z.strictObject({schemaVersion:z.literal(3),commandId:id,idempotencyKey:z.string().min(1).max(160),projectId,publicationId:id,baseRevision:revision,operations:z.array(z.strictObject({opId:id,action:publicationActionSchema})).min(1).max(100)}).superRefine((request,ctx)=>{
  if(new Set(request.operations.map(op=>op.opId)).size!==request.operations.length)ctx.addIssue({code:'custom',message:'Duplicate operation'});
  if(request.operations.length>1&&request.operations.some(op=>!['UpdateFrame','AddFrame','SetFramePhoto','RemoveFrame','UpdatePage','AddPage','AddText','UpdateText','UpdateTextGeometry','RemoveText'].includes(op.action.type)))ctx.addIssue({code:'custom',message:'Version changes need a separate command'});
  if((request.operations[0].action.type==='Create')!==(request.baseRevision===0))ctx.addIssue({code:'custom',message:'Publication version mismatch'});
});
export const publicationRecordSchema=z.object({id,project_id:projectId,pub_type:z.enum(['photobook','exhibition','portfolio']),title:z.string(),state:z.enum(['draft','in_review','finalized','delivered']),created_at:z.string(),revision:revision.refine(n=>n>0)});
export const publicationReceiptSchema=z.object({commandId:id,projectId,publicationId:id,baseRevision:revision,revision:revision.refine(n=>n>0),changed:z.boolean(),results:z.array(z.unknown())});
export type PublicationAction=z.infer<typeof publicationActionSchema>;
export type PublicationCommandRequest=z.infer<typeof publicationRequestSchema>;
export type PublicationReceipt=z.infer<typeof publicationReceiptSchema>;
export type PublicationFrameUpdate=Extract<PublicationAction,{type:'UpdateFrame'}>['data'];
export type PublicationPageUpdate=Extract<PublicationAction,{type:'UpdatePage'}>['data'];
export type PublicationPageAdd=Extract<PublicationAction,{type:'AddPage'}>['data'];
export type PublicationTextUpdate=Extract<PublicationAction,{type:'UpdateText'}>['data'];
export type PublicationTextGeometry=Extract<PublicationAction,{type:'UpdateTextGeometry'}>['data'];
export type PublicationFrameAdd=Extract<PublicationAction,{type:'AddFrame'}>['data'];
export type PublicationFramePhoto=Extract<PublicationAction,{type:'SetFramePhoto'}>['data'];
export type PublicationFrameGeometry=z.infer<typeof publicationFrameGeometrySchema>;
export type PublicationSpreadFrame=Extract<PublicationAction,{type:'SetSpreadFrame'}>['data'];
export type PublicationUnspreadFrame=Extract<PublicationAction,{type:'UnsetSpreadFrame'}>['data'];
export function parsePublicationReceipt(request:PublicationCommandRequest,value:unknown):PublicationReceipt {
  const result=publicationReceiptSchema.parse(value);
  const invalid=()=>{throw new Error('尚未确认出版操作结果');};
  if(result.commandId!==request.commandId||result.projectId!==request.projectId||result.publicationId!==request.publicationId||result.baseRevision!==request.baseRevision||result.revision!==request.baseRevision+Number(result.changed)||result.results.length!==request.operations.length)invalid();
  request.operations.forEach((op,i)=>{
    if(op.action.type==='Create'){const publication=publicationRecordSchema.parse(result.results[i]);if(!result.changed||publication.id!==request.publicationId||publication.project_id!==request.projectId||publication.revision!==result.revision)invalid();}
    else if(op.action.type==='Generate'){if(!result.changed)invalid();z.number().int().positive().parse(result.results[i]);}
    else{z.null().parse(result.results[i]);if(['CaptureVersion','RestoreVersion','RemovePages','CreateChapter','SetCover'].includes(op.action.type)&&!result.changed)invalid();}
  });return result;
}

export type SetPublicationChapters=Extract<PublicationAction,{type:'SetChapters'}>['data'];
export type CreatePublicationChapter=Extract<PublicationAction,{type:'CreateChapter'}>['data'];

export type RemovePublicationPages=Extract<PublicationAction,{type:'RemovePages'}>['data'];
