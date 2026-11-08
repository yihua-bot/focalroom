import { zh } from '../../app/copy';
import { knowledgeMatchLines } from './knowledgeMatch';
import type { PhotoJudgment } from '../../domain/photoJudgment';

export function PhotoJudgments({items, photoName}: {items: PhotoJudgment[]; photoName: (id:string)=>string}) {
 if (!items.length) return null;
 return <details className="chat-inspection"><summary>{zh.chatJudgmentTitle}</summary>
  <p className="chat-footnote">{zh.chatJudgmentInference}</p>
  {items.map(item=><article key={item.id}>
   <strong>{photoName(item.photoId)}</strong>
   <p className="chat-footnote">{item.imageStatus==='verified' ? zh.chatJudgmentPixelsMatch : zh.chatJudgmentPixelsChanged}</p>
   <p>{item.judgment}</p>
   <strong>{zh.chatJudgmentFacts}</strong><ul>{item.facts.map(fact=><li key={fact.index}>{fact.text}</li>)}</ul>
   <strong>{zh.chatJudgmentReasoning}</strong><p>{item.reasoning}</p>
   <strong>{zh.chatJudgmentMethods}</strong>
   {!item.methods.length && <p>{zh.chatJudgmentNoMethod}</p>}
   {item.methods.map((method,index)=>{
    const key=`chatReview_${method.reviewStatus}`;
    const status=method.kind==='original_method' ? zh.chatJudgmentOriginalMethod : !method.reviewStatus ? zh.chatJudgmentUnreviewed : zh[key]===key ? method.reviewStatus : zh[key];
    return <div key={index}><p>{method.title} · {status}{method.disputed ? ` · ${zh.chatReferenceDisputed}` : ''}</p>
     <blockquote>{method.passage}{method.excerpt && <small> · {zh.chatJudgmentExcerpt}</small>}</blockquote>
     <small>{method.source ?? zh.chatJudgmentNoSource} · {method.citationId}{method.version != null ? ` · ${method.version}` : ''}</small>
     {knowledgeMatchLines(method.matchReason).map((line,lineIndex)=><p key={lineIndex} className="chat-footnote">{line}</p>)}
     {method.matchReason && <p className="chat-footnote">{zh.chatMatchNotEvidence}</p>}
    </div>;
   })}
   <strong>{zh.chatJudgmentUncertainty}</strong><p>{item.observationUncertainty}</p><p>{item.uncertainty}</p>
  </article>)}
 </details>;
}
