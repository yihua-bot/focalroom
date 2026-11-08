import { z } from 'zod';
const count=z.number().int().nonnegative();
const read=z.object({
 memoryId:count,sourceProjectId:z.string().nullable(),sourceProjectTitle:z.string().nullable(),origin:z.string().nullable(),scope:z.string().nullable(),
 statusAtRead:z.string().nullable(),selectionReason:z.string().nullable(),
 confirmation:z.object({requestId:z.string(),createdAt:z.string(),action:z.enum(['ConfirmHypothesis','RememberPreference'])}).nullable(),
 memoryTextVersion:z.enum(['verified','changed','unrecorded','source_unavailable']),passage:z.string().nullable(),
 currentStatus:z.string().nullable(),disabled:z.boolean().nullable(),applicability:z.string().nullable(),
 evidence:z.object({recorded:count,available:count,excluded:count,missing:count}).nullable(),evidenceRefsTruncated:z.boolean(),
}).superRefine((value,ctx)=>{
 if(value.memoryTextVersion!=='verified' && (value.passage!==null || value.applicability!==null)) ctx.addIssue({code:'custom',message:'Historical memory text is not verified'});
 if(value.evidence && value.evidence.recorded!==value.evidence.available+value.evidence.excluded+value.evidence.missing) ctx.addIssue({code:'custom',message:'Evidence counts do not agree'});
 if(value.scope==='cross_project' && value.applicability!==null) ctx.addIssue({code:'custom',message:'Cross-project applicability text is outside this trace'});
 if(value.memoryTextVersion==='source_unavailable' && (value.currentStatus!==null || value.disabled!==null || value.sourceProjectTitle!==null || value.evidence!==null)) ctx.addIssue({code:'custom',message:'Unavailable source cannot supply current state'});
});
export const memoryTraceSchema=z.object({events:z.array(z.object({
 id:count,kind:z.enum(['context_attempt','query_result']),round:z.number().int().positive().nullable(),createdAt:z.string(),callId:z.string().nullable(),recorded:z.boolean(),reads:z.array(read).max(10),
})).max(40),hasEarlierEvents:z.boolean()});
