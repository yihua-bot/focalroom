import { useEffect, useRef, useState } from 'react';
import type { PublicationRepository, PublicationReview } from '../../domain/publication';
import { Button } from '../../components/shared/Button';
import { userFacingError } from '../../services/userFacingError';
import { zh } from '../../app/copy';

// 意见锚在"这一本 + 提它时那一版"上，界面只读后端那一份：本地攒一份草稿式意见列表会让"处理过了"变成各说各话。
export function PublicationReviewPanel({ repository, projectId, publicationId, readOnly }: {
  repository: PublicationRepository; projectId: string; publicationId: string; readOnly: boolean;
}) {
  const [page, setPage] = useState<PublicationReview>();
  const [body, setBody] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const lock = useRef(false), alive = useRef(true), token = useRef(0);
  const canWrite = !readOnly && Boolean(repository.addReview && repository.resolveReview);
  const show = (value: PublicationReview) => { if (alive.current) setPage(value); };
  const load = async () => {
    if (!repository.review) return;
    const current = ++token.current;
    try { const value = await repository.review(projectId, publicationId); if (alive.current && current === token.current) show(value); }
    catch (error) { if (alive.current && current === token.current) setError(userFacingError(error, zh.publicationReviewFailed)); }
  };
  useEffect(() => {
    alive.current = true; const current = ++token.current;
    repository.review?.(projectId, publicationId)
      .then(value => { if (alive.current && token.current === current) setPage(value); })
      .catch(error => { if (alive.current && token.current === current) setError(userFacingError(error, zh.publicationReviewFailed)); });
    return () => { alive.current = false; };
  }, [repository, projectId, publicationId]);
  const act = async (operation: () => Promise<PublicationReview>) => {
    if (lock.current || !canWrite) return;
    lock.current = true; setBusy(true); setError('');
    try { const value = await operation(); if (alive.current) { setPage(value); setBody(''); } }
    catch (error) { if (alive.current) setError(userFacingError(error, zh.publicationReviewFailed)); }
    finally { lock.current = false; if (alive.current) setBusy(false); }
  };
  if (!repository.review) return null;
  const open = page?.open ?? 0;
  return <section className="publication-studio-review" aria-label={zh.publicationReview}>
    <h4>{zh.publicationReview}</h4>
    <p className="publication-studio-review-hint">{zh.publicationReviewHint}</p>
    {!!page && <p role="status" className={open ? 'publication-studio-review-open' : 'publication-studio-review-clear'}>
      {open ? zh.publicationReviewOpen.replace('{count}', String(open)) : zh.publicationReviewSettled}
    </p>}
    {!page?.threads.length && <p className="muted">{zh.publicationReviewEmpty}</p>}
    {page?.threads.map(thread => <article key={thread.id} className="publication-studio-review-thread">
      <header>
        <strong>{zh.publicationReviewOnVersion.replace('{revision}', String(thread.revision))}</strong>
        {thread.state === 'resolved' && <small>{zh.publicationReviewResolved}</small>}
      </header>
      {thread.comments.map(comment => <blockquote key={comment.id}>
        <p>{comment.body}</p>
        <footer><cite>{comment.author}</cite><time>{new Date(comment.createdAt).toLocaleString()}</time></footer>
      </blockquote>)}
      <div className="publication-studio-actions"><Button size="small" disabled={busy || !canWrite} onClick={() => void act(() => repository.resolveReview!(projectId, publicationId, thread.id, thread.state !== 'resolved'))}>
        {thread.state === 'resolved' ? zh.publicationReviewReopen : zh.publicationReviewResolve}
      </Button></div>
    </article>)}
    {canWrite && <form onSubmit={event => {
      event.preventDefault();
      const text = body.trim();
      if (!text) return;
      void act(() => repository.addReview!(projectId, publicationId, text));
    }}>
      <label>{zh.publicationReviewBody}<textarea aria-label={zh.publicationReviewBody} rows={3} maxLength={5000} disabled={busy} value={body} onChange={event => setBody(event.target.value)} /></label>
      <Button type="submit" size="small" disabled={busy || !body.trim()}>{zh.publicationReviewSend}</Button>
    </form>}
    {error && <p role="alert">{error}</p>}
    <Button size="small" disabled={busy} onClick={() => void load()}>{zh.publicationReviewReload}</Button>
  </section>;
}
