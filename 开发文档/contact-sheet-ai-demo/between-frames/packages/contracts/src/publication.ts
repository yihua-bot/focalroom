import { z } from 'zod';
const id=z.uuid();
const projectId=z.string().min(1).max(160);
const finite=z.number().finite();
const revision=z.number().int().min(0).max(9_007_199_254_739_000);
const fit=z.enum(['fit','fill','stretch']);
const position={xMm:finite,yMm:finite,widthMm:finite.positive(),heightMm:finite.positive()};
const source=z.string().min(1).max(160).nullable();
const frame={frameId:id,...position,fitMode:fit,cropX:finite.min(0).max(1),cropY:finite.min(0).max(1),cropZoom:finite.min(1).max(20),assetId:source,renditionId:source};
const action=<T extends string,S extends z.ZodRawShape>(type:T,shape:S)=>z.strictObject({type:z.literal(type),data:z.strictObject(shape)});
export const publicationActionSchema=z.discriminatedUnion('type',[
  action('Create',{title:z.string().trim().min(1).max(200),pubType:z.enum(['photobook','exhibition','portfolio']).nullable().optional(),widthMm:finite.min(50).max(2000).nullable().optional(),heightMm:finite.min(50).max(2000).nullable().optional()}),
  action('Generate',{photosPerPage:z.union([z.literal(1),z.literal(2),z.literal(4)])}),
  action('UpdateFrame',frame),
  action('UpdatePage',{pageId:id,widthMm:finite.min(50).max(2000),heightMm:finite.min(50).max(2000),marginMm:finite.min(0).max(999),bleedMm:finite.min(0).max(25)}),
  action('CaptureVersion',{label:z.string().trim().min(1).max(120)}),
  action('RestoreVersion',{versionId:id}),
]);
export const publicationRequestSchema=z.strictObject({schemaVersion:z.literal(3),commandId:id,idempotencyKey:z.string().min(1).max(160),projectId,publicationId:id,baseRevision:revision,operations:z.array(z.strictObject({opId:id,action:publicationActionSchema})).min(1).max(100)}).superRefine((request,ctx)=>{
  if(new Set(request.operations.map(op=>op.opId)).size!==request.operations.length)ctx.addIssue({code:'custom',message:'Duplicate operation'});
  if(request.operations.length>1&&request.operations.some(op=>!['UpdateFrame','UpdatePage'].includes(op.action.type)))ctx.addIssue({code:'custom',message:'Version changes need a separate command'});
  if((request.operations[0].action.type==='Create')!==(request.baseRevision===0))ctx.addIssue({code:'custom',message:'Publication version mismatch'});
});
export const publicationRecordSchema=z.object({id,project_id:projectId,pub_type:z.enum(['photobook','exhibition','portfolio']),title:z.string(),state:z.enum(['draft','in_review','finalized','delivered']),created_at:z.string(),revision:revision.refine(n=>n>0)});
export const publicationReceiptSchema=z.object({commandId:id,projectId,publicationId:id,baseRevision:revision,revision:revision.refine(n=>n>0),changed:z.boolean(),results:z.array(z.unknown())});
export type PublicationAction=z.infer<typeof publicationActionSchema>;
export type PublicationCommandRequest=z.infer<typeof publicationRequestSchema>;
export type PublicationReceipt=z.infer<typeof publicationReceiptSchema>;
export type PublicationFrameUpdate=Extract<PublicationAction,{type:'UpdateFrame'}>['data'];
export function parsePublicationReceipt(request:PublicationCommandRequest,value:unknown):PublicationReceipt {
  const result=publicationReceiptSchema.parse(value);
  const invalid=()=>{throw new Error('尚未确认出版操作结果');};
  if(result.commandId!==request.commandId||result.projectId!==request.projectId||result.publicationId!==request.publicationId||result.baseRevision!==request.baseRevision||result.revision!==request.baseRevision+Number(result.changed)||result.results.length!==request.operations.length)invalid();
  request.operations.forEach((op,i)=>{
    if(op.action.type==='Create'){const publication=publicationRecordSchema.parse(result.results[i]);if(!result.changed||publication.id!==request.publicationId||publication.project_id!==request.projectId||publication.revision!==result.revision)invalid();}
    else if(op.action.type==='Generate'){if(!result.changed)invalid();z.number().int().positive().parse(result.results[i]);}
    else{z.null().parse(result.results[i]);if(['CaptureVersion','RestoreVersion'].includes(op.action.type)&&!result.changed)invalid();}
  });return result;
}
