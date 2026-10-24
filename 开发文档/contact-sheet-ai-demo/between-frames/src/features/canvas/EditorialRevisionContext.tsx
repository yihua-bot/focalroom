import { useEffect, useState } from 'react';
import type { CanvasRepository, EditorialMessage, EditorialRevisionContext as RevisionContext } from '../../domain/canvas';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';

export function EditorialRevisionContext({ repository, projectId, draftId }: { repository: CanvasRepository; projectId: string; draftId: string }) {
  const [location, setLocation] = useState({ draftId, offset: 0, trail: [] as string[], generation: 0 });
  const [page, setPage] = useState<RevisionContext | null>();
  const [busy, setBusy] = useState(true), [failed, setFailed] = useState(false);
  useEffect(() => {
    let active = true;
    if (!repository.editorialRevisionContext) return;
    repository.editorialRevisionContext(projectId, location.draftId, location.offset).then(value => {
      if (active) setPage(value);
    }).catch(() => { if (active) setFailed(true); }).finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, [repository, projectId, location.draftId, location.offset, location.generation]);
  if (!repository.editorialRevisionContext || page === null) return null;
  const navigate = (draft: string, offset: number, trail = location.trail) => {
    setBusy(true); setFailed(false); setPage(undefined); setLocation(previous => ({ draftId: draft, offset, trail, generation: previous.generation + 1 }));
  };
  const threads = new Map<string, EditorialMessage[]>();
  for (const message of page?.discussion.messages ?? []) threads.set(message.threadId, [...(threads.get(message.threadId) ?? []), message]);
  return <details className="editorial-revision-context">
    <summary>{zh.editorialRevisionContext}</summary>
    {busy && <p role="status">{zh['common.loading']}</p>}
    {failed && <p role="alert">{zh.editorialRevisionUnavailable}</p>}
    {page && <>
      <h4>{page.parentTitle || zh.editorialWholeProposal} · {zh.editorialRevisionNumber} {page.parent.revision}</h4>
      <p className="muted">{zh.editorialRevisionFrozen}</p>
      <h5>{zh.editorialRevisionDecisions} · {page.totalDecisions}</h5>
      <ol start={location.offset + 1}>{page.decisions.map(decision => <li key={decision.changeId}>
        <strong>{zh[`editorialKind_${decision.kind}`]} · {zh[`editorialDecision_${decision.decision}`]}</strong>
        {!!Object.keys(decision.fieldDecisions).length && <p>{Object.entries(decision.fieldDecisions).map(([field, state]) => `${zh[`editorialField_${field}`]}: ${zh[`editorialDecision_${state}`]}`).join(' · ')}</p>}
      </li>)}</ol>
      <h5>{zh.editorialDiscussion} · {page.discussion.totalMessages}</h5>
      {[...threads].map(([id, messages]) => <article className="editorial-comment-thread" key={id}>
        <p>{messages[0].rootBody}</p>
        {messages[0].resolved && <small>{zh.editorialThreadResolved}</small>}
        {messages.filter(message => message.isReply).map(message => <blockquote key={message.id}><p>{message.body}</p></blockquote>)}
      </article>)}
      <h5>{zh.editorialRevisionObservations} · {page.totalObservations}</h5>
      <p className="muted">{zh.editorialRevisionObservationHint}</p>
      {page.observations.map(photo => <article className="editorial-comment-thread" key={`${photo.taskId}:${photo.callId}:${photo.photoId}`}>
        {!photo.currentHashMatches && <p role="status">{zh.editorialRevisionPhotoChanged}</p>}
        <ul>{photo.visibleFacts.map((fact, index) => <li key={index}>{fact}</li>)}</ul>
        <p>{photo.interpretation}</p><p className="muted">{photo.uncertainty}</p>
      </article>)}
      <div className="editorial-comment-actions">
        {location.offset > 0 && <Button size="small" disabled={busy} onClick={() => navigate(location.draftId, Math.max(0, location.offset - 30))}>{zh.editorialRevisionPreviousPage}</Button>}
        {page.nextOffset !== null && <Button size="small" disabled={busy} onClick={() => navigate(location.draftId, page.nextOffset!)}>{zh.editorialRevisionNextPage}</Button>}
        {page.hasEarlier && !location.trail.includes(page.parent.id) && <Button size="small" disabled={busy} onClick={() => navigate(page.parent.id, 0, [...location.trail, location.draftId])}>{zh.editorialRevisionEarlier}</Button>}
        {!!location.trail.length && <Button size="small" disabled={busy} onClick={() => navigate(location.trail[location.trail.length - 1], 0, location.trail.slice(0, -1))}>{zh.editorialRevisionBack}</Button>}
      </div>
    </>}
    {failed && <Button size="small" onClick={() => navigate(location.draftId, location.offset)}>{zh['common.retry']}</Button>}
  </details>;
}
