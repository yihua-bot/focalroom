import { useCallback } from 'react';
import { useRevisionContext } from '../../components/review/useRevisionContext';
import { HistoricalObservations } from '../../components/review/HistoricalObservations';
import type { CanvasRepository, EditorialMessage, EditorialRevisionContext as RevisionContext } from '../../domain/canvas';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';

export function EditorialRevisionContext({ repository, projectId, draftId }: { repository: CanvasRepository; projectId: string; draftId: string }) {
  const load = useCallback((project: string, draft: string, offset: number) => repository.editorialRevisionContext!(project, draft, offset), [repository]);
  const { location, page, busy, failed, navigate } = useRevisionContext<RevisionContext>(repository.editorialRevisionContext ? load : undefined, projectId, draftId);
  if (!repository.editorialRevisionContext || page === null) return null;
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
      <HistoricalObservations observations={page.observations} total={page.totalObservations} />
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
