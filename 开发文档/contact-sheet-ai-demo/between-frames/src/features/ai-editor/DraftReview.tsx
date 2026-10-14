import { useState } from 'react';
import type { DesktopProjectRepository, Project } from '../../domain/models';
import type { SelectionDraft } from '../../domain/agent';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';

export function DraftReview({ draft, project, repository, onClose, onUpdated }: { draft: SelectionDraft; project: Project; repository: DesktopProjectRepository; onClose: () => void; onUpdated: (project: Project) => void }) {
 const [selected, setSelected] = useState<string[]>(draft.items.map(item => item.photo_id));
 const [busy, setBusy] = useState(false);
 const [error, setError] = useState('');
 const [confirmFull, setConfirmFull] = useState(false);
 const proposed = draft.items.filter(item => item.proposed_action === 'keep').map(item => item.photo_id);
 const added = proposed.filter(id => !project.sequence.includes(id)).length;
 const removed = project.sequence.filter(id => !proposed.includes(id)).length;
 const moved = proposed.filter((id, pos) => project.sequence.includes(id) && project.sequence.indexOf(id) !== pos).length;
 const stale = draft.stale || draft.baseRevision !== project.revision;
 const run = async (task: () => Promise<Project>, close = false) => {
  if (busy) return; setBusy(true); setError('');
  try { const updated = await task(); onUpdated(updated); if (close) onClose(); }
  catch (error) { setError(error instanceof Error ? error.message : String(error)); }
  finally { setBusy(false); }
 };
 return <Modal title={draft.title} description="本地 Mock 草案。先比较，再明确采纳；所有正式改变作为一组编辑，可撤销。" onClose={onClose} wide>
  <p>{draft.summary}</p>
  <div className="draft-diff" role="status"><strong>{stale ? '基线已过期 · 采纳已禁用' : '基线一致'}</strong><p>基线 {draft.baseRevision} → 当前 {project.revision}；全量替换将加入 {added} 张、移出 {removed} 张、移动 {moved} 张。</p>
   <p>局部采纳只改变所选条目；其他人工状态与未选照片的相对顺序保留。未保存备注不受草案影响。</p></div>
  {stale && draft.state === 'pending' && <Button disabled={busy} onClick={() => { void run(() => repository.rebaseDraft(project.id, draft)); }}>基于当前版本重新比较</Button>}
  <div className="draft-select-tools"><Button size="small" disabled={busy} onClick={() => setSelected(draft.items.map(item => item.photo_id))}>全选草案项</Button><Button size="small" disabled={busy} onClick={() => setSelected([])}>清除选择</Button><span>已选 {selected.length} / {draft.items.length}</span></div>
  <ol className="draft-items">{draft.items.map((item, index) => {
   const photo = project.photos.find(photo => photo.id === item.photo_id);
   return <li key={item.photo_id}><label><input type="checkbox" aria-label={'采纳 ' + (photo?.name ?? item.photo_id)} checked={selected.includes(item.photo_id)} disabled={busy || draft.state !== 'pending'} onChange={event => setSelected(previous => event.target.checked ? [...previous, item.photo_id] : previous.filter(id => id !== item.photo_id))} />
    {photo && <img src={photo.src} alt={photo.name} />}<span><strong>{index + 1}. {photo?.name ?? '照片不可用'}</strong><small>{item.proposed_action === 'keep' ? '建议保留' : item.proposed_action === 'reject' ? '建议淘汰' : '建议待定'} · 当前 {photo?.status === 'keep' ? '已保留' : photo?.status === 'reject' ? '已淘汰' : '待定'}</small><small>{item.rationale}</small></span></label>
    <div><Button size="small" disabled={busy || draft.state !== 'pending' || index === 0} aria-label={'草案上移 ' + photo?.name} onClick={() => { const ids = draft.items.map(i => i.photo_id); [ids[index - 1], ids[index]] = [ids[index], ids[index - 1]]; void run(() => repository.updateDraft(project.id, draft, ids)); }}>↑</Button>
    <Button size="small" disabled={busy || draft.state !== 'pending' || index === draft.items.length - 1} aria-label={'草案下移 ' + photo?.name} onClick={() => { const ids = draft.items.map(i => i.photo_id); [ids[index], ids[index + 1]] = [ids[index + 1], ids[index]]; void run(() => repository.updateDraft(project.id, draft, ids)); }}>↓</Button></div>
   </li>;
  })}</ol>
  {error && <p role="alert">{error}</p>}
  {draft.state !== 'pending' ? <p>此草案已{draft.state === 'applied' ? '完成采纳' : '丢弃'}；正式操作仍可在工作台撤销。</p> : <div className="dialog-actions">
   <Button disabled={busy} onClick={() => { void run(() => repository.discardDraft(project.id, draft), true); }}>丢弃草案</Button>
   <Button disabled={busy || stale || selected.length === 0} variant="primary" onClick={() => { void run(() => repository.applyDraft(project.id, draft, selected, 'partial'), true); }}>采纳所选 {selected.length} 项</Button>
   <Button disabled={busy || stale} onClick={() => setConfirmFull(true)}>替换正式序列</Button>
  </div>}
  {confirmFull && <div className="draft-full-confirm"><p>确认用全部 {draft.items.length} 项替换正式序列？未包含的原保留照片将恢复待定，评分和备注保留。</p><Button disabled={busy} onClick={() => setConfirmFull(false)}>返回检查</Button><Button disabled={busy || stale} variant="primary" onClick={() => { void run(() => repository.applyDraft(project.id, draft, draft.items.map(item => item.photo_id), 'full'), true); }}>确认全量替换</Button></div>}
 </Modal>;
}
