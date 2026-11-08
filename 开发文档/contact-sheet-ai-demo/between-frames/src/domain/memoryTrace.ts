export interface MemoryTraceRead {
 memoryId:number; sourceProjectId:string|null; sourceProjectTitle:string|null; origin:string|null; scope:string|null;
 statusAtRead:string|null; selectionReason:string|null;
 confirmation:{requestId:string;createdAt:string;action:'ConfirmHypothesis'|'RememberPreference'}|null;
 memoryTextVersion:'verified'|'changed'|'unrecorded'|'source_unavailable'; passage:string|null;
 currentStatus:string|null; disabled:boolean|null; applicability:string|null;
 evidence:{recorded:number;available:number;excluded:number;missing:number}|null; evidenceRefsTruncated:boolean;
}
export interface MemoryTrace {
 events:{id:number;kind:'context_attempt'|'query_result';round:number|null;createdAt:string;callId:string|null;recorded:boolean;reads:MemoryTraceRead[]}[];
 hasEarlierEvents:boolean;
}
