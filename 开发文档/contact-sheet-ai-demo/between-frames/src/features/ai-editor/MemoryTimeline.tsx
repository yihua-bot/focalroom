import { zh } from '../../app/copy';
import { useState } from 'react';
import type { MemoryTrace, MemoryTraceRead } from '../../domain/memoryTrace';

function label(prefix:string,value:string|null) {
 if(!value) return zh.memoryTraceUnknown;
 const key=`${prefix}${value}`; return zh[key]===key ? value : zh[key];
}
function time(value:string) { const date=new Date(value); return Number.isNaN(date.valueOf()) ? value : date.toLocaleString(); }
function Read({read}:{read:MemoryTraceRead}) {
 return <article>
  {read.passage ? <blockquote>{read.passage}</blockquote> : <p>{read.memoryTextVersion==='changed' ? zh.chatMemoryTextChanged : read.memoryTextVersion==='unrecorded' ? zh.chatMemoryTextUnrecorded : zh.chatMemoryUnreadable}</p>}
  <p>{label('chatMemoryOrigin_',read.origin)} · {read.scope==='cross_project' ? zh.chatMemoryFromProject.replace('{project}',read.sourceProjectTitle ?? read.sourceProjectId ?? zh.memoryTraceUnknown) : read.scope==='project_local' ? zh.chatMemoryLocalProject : zh.memoryTraceUnknown}</p>
  <p>{zh.memoryTraceReason} {label('memoryReason_',read.selectionReason)}</p>
  <p>{zh.memoryTraceReadStatus} {label('chatMemoryState_',read.statusAtRead)}</p>
  <p>{zh.memoryTraceCurrentStatus} {label('chatMemoryState_',read.currentStatus)}{read.disabled===true ? ` · ${zh.chatMemoryDisabled}` : ''}</p>
  {read.confirmation ? <details><summary>{zh.memoryTraceConfirmation}</summary><p>{read.confirmation.action==='RememberPreference' ? zh.memoryTraceSaved : zh.memoryTraceConfirmed} · {time(read.confirmation.createdAt)}</p><small>{read.confirmation.requestId}</small></details> : <p>{zh.memoryTraceNoConfirmation}</p>}
  {read.applicability && <p>{zh.memoryTraceApplicability} {read.applicability}</p>}
  {read.evidence ? <p>{zh.memoryTraceEvidence.replace('{available}',String(read.evidence.available)).replace('{excluded}',String(read.evidence.excluded)).replace('{missing}',String(read.evidence.missing))}</p> : <p>{zh.memoryTraceNoEvidence}</p>}
  {read.evidenceRefsTruncated && <p>{zh.memoryTracePartialEvidence}</p>}
 </article>;
}
export function MemoryTimeline({trace}:{trace:MemoryTrace|undefined}) {
 const [expanded,setExpanded]=useState(false);
 if(!trace?.events.length) return null;
 return <details className="chat-inspection" onToggle={event=>setExpanded(event.currentTarget.open)}><summary>{zh.memoryTraceTitle}</summary>
  {expanded && <>
  <p className="chat-footnote">{zh.memoryTraceNotice}</p>
  {trace.hasEarlierEvents && <p>{zh.memoryTraceEarlier}</p>}
  {trace.events.map(event=><section key={event.id}>
   <strong>{event.kind==='context_attempt' ? zh.memoryTraceAttempt : zh.memoryTraceQuery}{event.round ? ` · ${zh.memoryTraceRound.replace('{round}',String(event.round))}` : ` · ${zh.memoryTraceRoundUnknown}`}</strong>
   <p className="chat-footnote">{time(event.createdAt)}</p>
   {!event.recorded ? <p>{zh.memoryTraceNotRecorded}</p> : !event.reads.length ? <p>{zh.memoryTraceEmpty}</p> : event.reads.map(read=><Read key={read.memoryId} read={read} />)}
  </section>)}
  </>}
 </details>;
}
