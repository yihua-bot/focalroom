import { useCallback } from 'react';
import type { AgentRepository } from '../../domain/agent';
import type { SelectionRevisionContext as RevisionContext } from '../../domain/revisionContext';
import { useRevisionContext } from '../../components/review/useRevisionContext';
import { HistoricalObservations } from '../../components/review/HistoricalObservations';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';

export function SelectionRevisionContext({ repository, projectId, draftId }: { repository: AgentRepository; projectId: string; draftId: string }) {
  const load = useCallback((project: string, draft: string, offset: number) => repository.selectionRevisionContext!(project,draft,offset),[repository]);
  const { location,page,busy,failed,navigate } = useRevisionContext<RevisionContext>(repository.selectionRevisionContext ? load : undefined,projectId,draftId);
  if (!repository.selectionRevisionContext || page === null) return null;
  return <details className="selection-revision-context revision-context">
    <summary>{zh.editorialRevisionContext}</summary>
    {busy && <p role="status">{zh['common.loading']}</p>}
    {failed && <p role="alert">{zh.editorialRevisionUnavailable}</p>}
    {page && <>
      <h4>{page.parentTitle} · {zh.editorialRevisionNumber} {page.parent.revision}</h4>
      <p className="muted">{zh.editorialRevisionFrozen}</p><p>{page.parentSummary}</p>
      <p>{zh.selectionContextState}: {zh[`selectionState_${page.parentState}`]}</p>
      <h5>{zh.selectionContextItems} · {page.totalItems}</h5><p className="muted">{zh.selectionContextProposalHint}</p>
      <ol className="revision-selection-items" start={location.offset + 1}>{page.items.map(item => <li key={item.photoId}>
        {item.currentHashMatches && item.thumbnailPath && <img className="revision-item-thumbnail" src={item.thumbnailPath} alt={item.photoName} loading="lazy" />}
        <div><strong>{item.position+1}. {item.photoName}</strong><p>{zh[`selectionProposed_${item.action}`]}</p><p>{item.rationale}</p>
        {!item.currentHashMatches && <p role="status">{zh.editorialRevisionPhotoChanged}</p>}</div>
      </li>)}</ol>
      <h5>{zh.selectionContextReviews} · {page.totalReviews}</h5>
      {page.reviews.map(review => <article className="revision-observation" key={review.commandId}>
        <strong>{zh[`selectionReview_${review.kind === 'apply' ? `apply_${review.mode}` : review.kind}`]}</strong>
        <p><time>{new Date(review.createdAt).toLocaleString()}</time></p>
        {review.undoneAtDelegation && <p>{zh.selectionContextUndone}</p>}
        {review.partial && <p className="muted">{zh.selectionContextPartial}</p>}
        {!!review.photoNames.length && <details><summary>{zh.selectionContextReviewedPhotos} · {review.photoNames.length}</summary><ul>{review.photoNames.map((name,index) => <li key={review.photoIds[index]}>{name}</li>)}</ul></details>}
      </article>)}
      <h5>{zh.selectionContextFeedback} · {page.totalFeedback}</h5>
      {page.feedback.map(message => <blockquote className="revision-feedback" key={message.messageId}><strong>{message.role === 'user' ? zh.selectionContextUser : zh.selectionContextQuestion}</strong><p>{message.content}</p></blockquote>)}
      <HistoricalObservations observations={page.observations} total={page.totalObservations} />
      <div className="revision-navigation">
        {location.offset > 0 && <Button size="small" disabled={busy} onClick={() => navigate(location.draftId,Math.max(0,location.offset-30))}>{zh.editorialRevisionPreviousPage}</Button>}
        {page.nextOffset !== null && <Button size="small" disabled={busy} onClick={() => navigate(location.draftId,page.nextOffset!)}>{zh.editorialRevisionNextPage}</Button>}
        {page.hasEarlier && !location.trail.includes(page.parent.id) && <Button size="small" disabled={busy} onClick={() => navigate(page.parent.id,0,[...location.trail,location.draftId])}>{zh.editorialRevisionEarlier}</Button>}
      </div>
    </>}
    {!!location.trail.length && <Button size="small" onClick={() => navigate(location.trail[location.trail.length-1],0,location.trail.slice(0,-1))}>{zh.editorialRevisionBack}</Button>}
    {failed && <Button size="small" onClick={() => navigate(location.draftId,location.offset)}>{zh['common.retry']}</Button>}
  </details>;
}
