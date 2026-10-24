import { useEffect, useRef, useState } from 'react';
import type { CanvasRepository, EditorialDiscussion as Discussion, EditorialMessage } from '../../domain/canvas';
import { Button } from '../../components/shared/Button';
import { userFacingError } from '../../services/userFacingError';
import { zh } from '../../app/copy';

export function EditorialDiscussion({ repository, projectId, draftId, revision, changeId, changeTitles, readOnly, onRevise }: {
  repository: CanvasRepository; projectId: string; draftId: string; revision: number; changeId?: string; changeTitles: Record<string,string>; readOnly: boolean; onRevise?: () => void;
}) {
  const [page, setPage] = useState<Discussion>();
  const [messages, setMessages] = useState<EditorialMessage[]>([]);
  const [earliest, setEarliest] = useState(0);
  const [scope, setScope] = useState<'change'|'proposal'>('change');
  const target = scope === 'change' ? changeId ?? null : null;
  const [drafts, setDrafts] = useState<Record<string,string>>({});
  const [replying, setReplying] = useState<string>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const lock = useRef(false), alive = useRef(true), request = useRef(0);
  const pending = repository.pendingCommands?.(projectId).some(c => c.boardId === draftId);
  const key = target ?? 'proposal';
  const refresh = async () => {
    if (!repository.editorialDiscussion) return;
    const number = ++request.current;
    const first = await repository.editorialDiscussion(projectId,draftId,revision);
    const offset = Math.max(0, first.totalMessages - 30);
    const last = offset ? await repository.editorialDiscussion(projectId,draftId,revision,offset,first.cursor) : first;
    if (alive.current && number === request.current) { setPage(last); setMessages(last.messages); setEarliest(offset); }
  };
  useEffect(() => {
    alive.current = true;
    let active = true;
    repository.editorialDiscussion?.(projectId,draftId,revision).then(async first => {
      const offset = Math.max(0,first.totalMessages-30);
      const last = offset ? await repository.editorialDiscussion!(projectId,draftId,revision,offset,first.cursor) : first;
      if (active && request.current === 0) { setPage(last); setMessages(last.messages); setEarliest(offset); }
    }).catch(error => { if (active) setError(userFacingError(error,zh.editorialDiscussionFailed)); });
    return () => { active = false; alive.current = false; };
  }, [repository,projectId,draftId,revision]);
  const act = async (action: () => Promise<void>, after?: () => void) => {
    if (lock.current || readOnly || pending) return;
    lock.current = true; setBusy(true); setError('');
    try { await action(); if (alive.current) { after?.(); await refresh(); } }
    catch (error) { if (alive.current) setError(userFacingError(error,zh.editorialDiscussionFailed)); }
    finally { lock.current = false; if (alive.current) setBusy(false); }
  };
  const threads = new Map<string,EditorialMessage[]>();
  for (const message of messages) {
    if (scope === 'change' && message.changeId !== target && message.changeId !== null) continue;
    threads.set(message.threadId,[...(threads.get(message.threadId) ?? []),message]);
  }
  if (!repository.editorialDiscussion) return null;
  return <section className="editorial-discussion" aria-label={zh.editorialDiscussion}>
    <h4>{zh.editorialDiscussion}</h4>
    <select aria-label={zh.editorialCommentScope} value={scope} onChange={event => setScope(event.target.value as 'change'|'proposal')}><option value="change">{zh.editorialCurrentSuggestion}</option><option value="proposal">{zh.editorialWholeProposal}</option></select>
    {earliest > 0 && <Button size="small" disabled={busy} onClick={() => {
      if (!page || lock.current) return;
      lock.current = true; setBusy(true);
      const offset = Math.max(0,earliest-30), cursor = page.cursor;
      void repository.editorialDiscussion!(projectId,draftId,revision,offset,cursor).then(value => {
        if (!alive.current) return;
        setMessages(old => [...value.messages,...old.filter(m => !value.messages.some(earlier => earlier.id === m.id))]); setEarliest(offset);
      }).catch(error => { if (alive.current) setError(userFacingError(error,zh.editorialDiscussionFailed)); }).finally(() => { lock.current=false; if (alive.current) setBusy(false); });
    }}>{zh.editorialEarlierComments}</Button>}
    {[...threads].map(([thread,items]) => {
      const root = items[0], replyKey = `reply:${thread}`, body = drafts[replyKey] ?? '';
      return <article key={thread} className="editorial-comment-thread">
        <header><strong>{root.changeId ? changeTitles[root.changeId] ?? zh.editorialCurrentSuggestion : zh.editorialWholeProposal}</strong>{root.resolved && <small>{zh.editorialThreadResolved}</small>}</header>
        <p>{root.rootBody}</p>
        {items.filter(m => m.isReply).map(message => <blockquote key={message.id}><p>{message.body}</p><time>{new Date(message.createdAt).toLocaleString()}</time></blockquote>)}
        <div className="editorial-comment-actions"><Button size="small" disabled={busy || readOnly || pending || !repository.resolveEditorialThread} onClick={() => void act(() => repository.resolveEditorialThread!(projectId,draftId,revision,thread,!root.resolved))}>{root.resolved ? zh.editorialReopenThread : zh.editorialResolveThread}</Button>
          {!root.resolved && <Button size="small" disabled={busy || readOnly || pending} onClick={() => setReplying(replying === thread ? undefined : thread)}>{zh.editorialReply}</Button>}
        </div>
        {replying === thread && !root.resolved && <form onSubmit={event => { event.preventDefault(); if (body.trim() && repository.addEditorialComment) void act(() => repository.addEditorialComment!(projectId,draftId,revision,{changeId:root.changeId,threadId:thread,body:body.trim()}),() => { setDrafts(values => ({...values,[replyKey]:''})); setReplying(undefined); }); }}>
          <textarea aria-label={zh.editorialReplyBody} maxLength={2000} rows={2} disabled={busy || readOnly || pending} value={body} onChange={event => setDrafts(values => ({...values,[replyKey]:event.target.value}))} />
          <Button type="submit" size="small" disabled={busy || readOnly || pending || !body.trim()}>{zh.editorialSendComment}</Button>
        </form>}
      </article>;
    })}
    {!threads.size && page && <p className="muted">{zh.editorialNoComments}</p>}
    <form onSubmit={event => { event.preventDefault(); const body = (drafts[key] ?? '').trim(); if (body && repository.addEditorialComment) void act(() => repository.addEditorialComment!(projectId,draftId,revision,{changeId:target,threadId:null,body}),() => setDrafts(values => ({...values,[key]:''}))); }}>
      <label>{target ? changeTitles[target] ?? zh.editorialCurrentSuggestion : zh.editorialWholeProposal}<textarea aria-label={zh.editorialCommentBody} placeholder={zh.editorialCommentPlaceholder} maxLength={2000} rows={3} disabled={busy || readOnly || pending} value={drafts[key] ?? ''} onChange={event => setDrafts(values => ({...values,[key]:event.target.value}))} /></label>
      <Button type="submit" size="small" disabled={busy || readOnly || pending || !drafts[key]?.trim() || !repository.addEditorialComment}>{zh.editorialSendComment}</Button>
    </form>
    {onRevise && <Button size="small" disabled={busy || pending || readOnly || !page?.totalMessages} onClick={onRevise}>{zh.editorialReviseFromComments}</Button>}
    {error && <p role="alert">{error}</p>}
  </section>;
}
