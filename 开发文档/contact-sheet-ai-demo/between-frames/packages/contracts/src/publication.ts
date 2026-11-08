import { z } from 'zod';
const id=z.uuid();
const projectId=z.string().min(1).max(160);
const finite=z.number().finite();
const revision=z.number().int().min(0).max(9_007_199_254_739_000);
const fit=z.enum(['fit','fill','stretch']);
const position={xMm:finite,yMm:finite,widthMm:finite.positive(),heightMm:finite.positive()};
const source=z.string().min(1).max(160).nullable();
const caption=z.enum(['left','center','right']);
// 审校状态只在这一处定义：记录、命令、界面徽标都读它，避免各写一份枚举。
const state=z.enum(['draft','in_review','finalized','delivered']);
const text={textId:id,...position,text:z.string().min(1).max(300),sizePt:finite.min(6).max(72),align:caption};
const frame={frameId:id,...position,fitMode:fit,cropX:finite.min(0).max(1),cropY:finite.min(0).max(1),cropZoom:finite.min(1).max(20),assetId:source,renditionId:source};
const action=<T extends string,S extends z.ZodRawShape>(type:T,shape:S)=>z.strictObject({type:z.literal(type),data:z.strictObject(shape)});
export const publicationActionSchema=z.discriminatedUnion('type',[
  action('Create',{title:z.string().trim().min(1).max(200),pubType:z.enum(['photobook','exhibition','portfolio']).nullable().optional(),widthMm:finite.min(50).max(2000).nullable().optional(),heightMm:finite.min(50).max(2000).nullable().optional()}),
  action('Generate',{photosPerPage:z.union([z.literal(1),z.literal(2),z.literal(4)])}),
  action('UpdateFrame',frame),
  action('UpdatePage',{pageId:id,widthMm:finite.min(50).max(2000),heightMm:finite.min(50).max(2000),marginMm:finite.min(0).max(999),bleedMm:finite.min(0).max(25)}),
  // 审校状态单独成条：它不是版面改动，不能和改图框、改纸张混在一条命令里半途生效。
  action('UpdateState',{state}),
  // 展览规格单独成条：改的是"这面墙怎么看"，不是版面，不能和改图框拼在一条命令里半途生效。
  // 两个数各自"要么在物理范围内、要么明确留空"，留空就是不记录。
  action('UpdateSpec',{viewingDistanceMm:finite.min(300).max(20000).nullable(),hangCenterMm:finite.min(300).max(3000).nullable()}),
  action('AddText',{...text,pageId:id}),
  action('UpdateText',text),
  action('DeleteText',{textId:id}),
  action('CaptureVersion',{label:z.string().trim().min(1).max(120)}),
  action('RestoreVersion',{versionId:id}),
]);
export const publicationRequestSchema=z.strictObject({schemaVersion:z.literal(3),commandId:id,idempotencyKey:z.string().min(1).max(160),projectId,publicationId:id,baseRevision:revision,operations:z.array(z.strictObject({opId:id,action:publicationActionSchema})).min(1).max(100)}).superRefine((request,ctx)=>{
  if(new Set(request.operations.map(op=>op.opId)).size!==request.operations.length)ctx.addIssue({code:'custom',message:'Duplicate operation'});
  if(request.operations.length>1&&request.operations.some(op=>!['UpdateFrame','UpdatePage','AddText','UpdateText','DeleteText'].includes(op.action.type)))ctx.addIssue({code:'custom',message:'Version changes need a separate command'});
  if((request.operations[0].action.type==='Create')!==(request.baseRevision===0))ctx.addIssue({code:'custom',message:'Publication version mismatch'});
});
export const publicationRecordSchema=z.object({id,project_id:projectId,pub_type:z.enum(['photobook','exhibition','portfolio']),title:z.string(),state,created_at:z.string(),revision:revision.refine(n=>n>0)});
export const publicationReceiptSchema=z.object({commandId:id,projectId,publicationId:id,baseRevision:revision,revision:revision.refine(n=>n>0),changed:z.boolean(),results:z.array(z.unknown())});
export type PublicationAction=z.infer<typeof publicationActionSchema>;
export type PublicationCommandRequest=z.infer<typeof publicationRequestSchema>;
export type PublicationReceipt=z.infer<typeof publicationReceiptSchema>;
export type PublicationFrameUpdate=Extract<PublicationAction,{type:'UpdateFrame'}>['data'];
export type PublicationPageUpdate=Extract<PublicationAction,{type:'UpdatePage'}>['data'];
export type PublicationState=z.infer<typeof state>;
export type PublicationStateUpdate=Extract<PublicationAction,{type:'UpdateState'}>['data'];
export type PublicationSpecUpdate=Extract<PublicationAction,{type:'UpdateSpec'}>['data'];
export type PublicationTextUpdate=Extract<PublicationAction,{type:'UpdateText'}>['data'];
export type PublicationTextNew=Extract<PublicationAction,{type:'AddText'}>['data'];
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
