import { useRef, useState } from 'react';
import type { DesktopProjectRepository, Project } from '../../domain/models';
import type { DraftEvidence, PhotoInspection, SelectionDraft } from '../../domain/agent';
import type { PhotoJudgment } from '../../domain/photoJudgment';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { SelectionRevisionContext } from './SelectionRevisionContext';
import { zh } from '../../app/copy';
import { userFacingError } from '../../services/userFacingError';

/** 这一张为什么还不敢定：判断层的不确定与画面观察层的不确定是两件事，分开记、分开说。 */
export interface DraftItemDoubt { photoId: string; judgment?: string; observation?: string }

/**
 * 草案条目本身只带取舍理由，不带"AI 自己有多拿不准"。那句准头藏在同一轮任务的回执里：
 * 判断记录说得出两层的不确定，就以它为准；只有观察记录的照片退回观察层，缺判断就只说观察，
 * 不拿观察层的句子冒充判断层说过话。
 */
export function collectDraftDoubts(source?: { inspectedPhotos?: PhotoInspection[]; photoJudgments?: PhotoJudgment[] }): DraftItemDoubt[] {
 const doubts = new Map<string, DraftItemDoubt>();
 const note = (photoId: string, field: 'judgment' | 'observation', text: string) => {
  if (!photoId || !text) return;
  doubts.set(photoId, { ...doubts.get(photoId) ?? { photoId }, [field]: text });
 };
 const said = (text: string | undefined) => (text ?? '').trim();
 for (const item of source?.photoJudgments ?? []) {
  note(item.photoId, 'judgment', said(item.uncertainty));
  note(item.photoId, 'observation', said(item.observationUncertainty));
 }
 for (const item of source?.inspectedPhotos ?? []) {
  if (said(item.uncertainty) && doubts.get(item.photo_id)?.observation === undefined) note(item.photo_id, 'observation', said(item.uncertainty));
 }
 return [...doubts.values()];
}

export function DraftReview({ draft, project, repository, evidence, doubts = [], onClose, onUpdated }: { draft: SelectionDraft; project: Project; repository: DesktopProjectRepository; evidence?: DraftEvidence; doubts?: DraftItemDoubt[]; onClose: () => void; onUpdated: (project: Project) => void }) {
 const [selected, setSelected] = useState<string[]>(draft.items.map(item => item.photo_id));
 const [busy, setBusy] = useState(false);
 const [error, setError] = useState('');
 const [confirmFull, setConfirmFull] = useState(false);
 const [rechecked, setRechecked] = useState<{ kind: 'done' | 'unchanged'; omitted: number } | null>(null);
 const lock = useRef(false);
 const proposed = draft.items.filter(item => item.proposed_action === 'keep').map(item => item.photo_id);
 const added = proposed.filter(id => !project.sequence.includes(id)).length;
 const removed = project.sequence.filter(id => !proposed.includes(id)).length;
 const moved = proposed.filter((id, pos) => project.sequence.includes(id) && project.sequence.indexOf(id) !== pos).length;
 const stale = draft.stale || draft.baseRevision !== project.revision;
 const run = async (task: () => Promise<Project>, close = false) => {
  if (lock.current) return; lock.current=true; setBusy(true); setError('');
  try { const updated = await task(); onUpdated(updated); if (close) onClose(); }
  catch (error) { setError(userFacingError(error)); }
  finally { lock.current=false; setBusy(false); }
 };
 // 重算只让本地把画面事实再比一遍：不重问模型、不上传照片，也不动方案版本号。
 const recheck = async () => {
  if (lock.current || !repository.agent.recheckEvidence) return;
  // 面板每秒都在重读投影，所以比对要用点击这一刻屏幕上的数，不能等回执回来再取。
  const before = evidence;
  lock.current=true; setBusy(true); setError(''); setRechecked(null);
  try {
   const receipt = await repository.agent.recheckEvidence(project.id, draft.id);
   const same = !!before && receipt.observed === before.observed && receipt.unobserved === before.unobserved
    && receipt.omitted === before.omitted && receipt.draftRevision === before.draftRevision;
   setRechecked({ kind: same ? 'unchanged' : 'done', omitted: receipt.omitted });
  }
  catch (error) { setError(userFacingError(error)); }
  finally { lock.current=false; setBusy(false); }
 };
 const recheckNote = rechecked?.kind === 'unchanged' ? zh.draftEvidenceRecheckUnchanged.replace('{count}', String(rechecked.omitted)) : null;
 // 本地永远翻不过来的有三类：当初没记像素指纹（重算读的还是同一行，指纹仍是空的，也不许拿新算的指纹冒充旧的）、
 // 原图不在这台机器上、原图已经换过（后两类修复一律拒绝写）。其余（字节对不上、预览读不出来但原图没变）
 // 修好预览后有可能翻过来，按钮得留着；原图状态是"没查"时同样不敢断言白点。
 // 只有逐张原因一张不缺地看得到时才敢下这个结论——降级那份没有逐张，超出上限的会被截掉。
 const omittedTiers = evidence?.perPhoto.filter(entry => !entry.used) ?? [];
 const allUnrecorded = omittedTiers.every(entry => entry.pixelVersion === 'unrecorded');
 const recheckCannotHelp = !!evidence && !evidence.basisSummary && evidence.omitted > 0
  && evidence.draftRevision === draft.revision && omittedTiers.length === evidence.omitted
  && omittedTiers.every(entry => entry.pixelVersion === 'unrecorded' || entry.originalStatus === 'missing' || entry.originalStatus === 'changed');
 return <Modal title={draft.title} onClose={onClose} wide className="draft-review-dialog">
  <p>{draft.summary}</p>
  {evidence && <div className="draft-evidence">
   <p role="note">{evidence.observed ? zh.draftEvidenceTitle.replace('{observed}', String(evidence.observed)).replace('{unobserved}', String(evidence.unobserved)) : zh.draftEvidenceNone}</p>
   <p className="micro">{zh.draftEvidenceTiers.replace('{page}', String(evidence.pageTier)).replace('{detail}', String(evidence.detailTier)).replace('{version}', String(evidence.otherVersion)).replace('{tool}', String(evidence.otherTool))}</p>
   {evidence.draftRevision !== null && <p className="micro">{evidence.draftRevision === draft.revision ? zh.draftEvidenceRevision.replace('{revision}', String(evidence.draftRevision)) : zh.draftEvidenceRevisionStale.replace('{then}', String(evidence.draftRevision)).replace('{now}', String(draft.revision))}</p>}
   {!!evidence.crossContractPairs && <p className="micro">{zh.draftEvidenceCross.replace('{count}', String(evidence.crossContractPairs))}</p>}
   {!!evidence.omitted && <p className="micro">{zh.draftEvidenceOmitted.replace('{count}', String(evidence.omitted))}</p>}
   {/* 降级那份的计数是就地取的，不是猜的；但逐张档位真没读回来，就得照着说，别留白让人以为每张都核过。 */}
   {evidence.basisSummary && <p className="micro">{zh.draftEvidenceBasisSummary}</p>}
   {evidence.observed > evidence.scanned && <p className="micro">{zh.draftEvidenceScanned}</p>}
   {/* 只在口径确实可能对不上的时候开口：版本号对不上，或有照片的画面记录被排除（修完预览就属于这种）。
       口径本来就是新的时候不摆按钮，免得看着像还有一件事没做。 */}
   {repository.agent.recheckEvidence && (evidence.draftRevision !== draft.revision || evidence.omitted > 0) && (recheckCannotHelp
    ? <p role="note">{(allUnrecorded ? zh.draftEvidenceRecheckCannotHelp : zh.draftEvidenceRecheckCannotHelpOriginal).replace('{count}', String(evidence.omitted))}</p>
    : <div className="micro draft-evidence-recheck">
     <p>{zh.draftEvidenceRecheckHint}</p>
     <Button size="small" disabled={busy} onClick={() => { void recheck(); }}>{zh.draftEvidenceRecheck}</Button>
     {rechecked && <span role="status">{recheckNote ?? zh.draftEvidenceRecheckDone}</span>}
    </div>)}
  </div>}
  {/* 这一格说的是"没有能核对上的画面依据"：既包括从来没算过的方案（画布里生成的那份序列草案最常见），也包括算过但那条核验单整条读不回来的（太大，或被同任务里更新的一批挤出读取窗口）——后者界面分辨不了，所以话只能说到这里，不许猜成"没算过"：
      重算的入口本来就挂在"已经有一份口径"里面，等于没人能开这个口。这一步仍旧纯本地，不重问模型、不上传照片。 */}
  {!evidence && repository.agent.recheckEvidence && <div className="micro draft-evidence-missing">
   <p role="note">{zh.draftEvidenceMissing}</p>
   <p>{zh.draftEvidenceRecheckHint}</p>
   <Button size="small" disabled={busy} onClick={() => { void recheck(); }}>{zh.draftEvidenceCompute}</Button>
   {rechecked && <span role="status">{recheckNote ?? zh.draftEvidenceComputed}</span>}
  </div>}
  <div className="draft-diff" role="status">{stale && <strong>{zh.selectionStale}</strong>}<p>{zh.selectionDifference.replace('{added}',String(added)).replace('{removed}',String(removed)).replace('{moved}',String(moved))}</p></div>
  {stale && draft.state === 'pending' && <Button disabled={busy} onClick={() => { void run(() => repository.rebaseDraft(project.id, draft)); }}>{zh.selectionRecompare}</Button>}
  {draft.state === 'pending' && <div className="draft-select-tools"><Button size="small" disabled={busy} onClick={() => setSelected(draft.items.map(item => item.photo_id))}>{zh.selectionSelectAll}</Button><Button size="small" disabled={busy} onClick={() => setSelected([])}>{zh.selectionClear}</Button><span>{zh.selectionSelected} {selected.length} / {draft.items.length}</span></div>}
  <ol className="draft-items">{draft.items.map((item, index) => {
   const photo = project.photos.find(photo => photo.id === item.photo_id);
   // 逐张口径只在这张确实被记过时说话：没记过就什么都不加，不拿"没有记录"当作"没有观察"。
   const tier = evidence?.perPhoto.find(entry => entry.photoId === item.photo_id);
   const tierNote = !tier ? '' : tier.used ? tier.contract ? zh[`draftTier_${tier.contract}`] : '' : tier.originalStatus === 'missing' || tier.originalStatus === 'changed' ? zh[`draftTierOmitted_original_${tier.originalStatus}`] : tier.pixelVersion ? zh[`draftTierOmitted_${tier.pixelVersion}`] : '';
   // 准头也是从回执里读的：这一张没留下过任何"说不准"就一行都不写，不替模型补一句客气的犹豫。
   const doubt = doubts.find(entry => entry.photoId === item.photo_id);
   return <li key={item.photo_id}><label>{draft.state==='pending' && <input type="checkbox" aria-label={zh.selectionAcceptPhoto+' '+(photo?.name ?? zh.selectionPhotoUnavailable)} checked={selected.includes(item.photo_id)} disabled={busy || draft.state !== 'pending'} onChange={event => setSelected(previous => event.target.checked ? [...previous, item.photo_id] : previous.filter(id => id !== item.photo_id))} />}
    {photo && <img src={photo.src} alt={photo.name} />}<span><strong>{index + 1}. {photo?.name ?? zh.selectionPhotoUnavailable}</strong><small>{zh[`selectionProposed_${item.proposed_action}`]} · {zh.selectionCurrent} {zh[`selectionStatus_${photo?.status ?? 'undecided'}`]}</small><small>{item.rationale}</small>{tierNote && <small className="draft-tier">{tierNote}</small>}{doubt?.observation && <small className="draft-doubt">{zh.draftDoubtObservation.replace('{text}', doubt.observation)}</small>}{doubt?.judgment && <small className="draft-doubt">{zh.draftDoubtJudgment.replace('{text}', doubt.judgment)}</small>}</span></label>
    <div><Button size="small" disabled={busy || draft.state !== 'pending' || index === 0} aria-label={zh.selectionMoveUp+' '+photo?.name} onClick={() => { const ids = draft.items.map(i => i.photo_id); [ids[index - 1], ids[index]] = [ids[index], ids[index - 1]]; void run(() => repository.updateDraft(project.id, draft, ids)); }}>↑</Button>
    <Button size="small" disabled={busy || draft.state !== 'pending' || index === draft.items.length - 1} aria-label={zh.selectionMoveDown+' '+photo?.name} onClick={() => { const ids = draft.items.map(i => i.photo_id); [ids[index], ids[index + 1]] = [ids[index + 1], ids[index]]; void run(() => repository.updateDraft(project.id, draft, ids)); }}>↓</Button></div>
   </li>;
  })}</ol>
  <SelectionRevisionContext repository={repository.agent} projectId={project.id} draftId={draft.id} onRepair={repository.repairPreviews ? photoIds => repository.repairPreviews!(project.id, photoIds) : undefined} />
  {error && <p role="alert">{error}</p>}
  {draft.state !== 'pending' ? <p>{zh[draft.state==='applied'?'selectionApplied':'selectionDiscarded']}</p> : <div className="dialog-actions">
   <Button disabled={busy} onClick={() => { void run(() => repository.discardDraft(project.id, draft), true); }}>{zh.selectionDiscard}</Button>
   <Button disabled={busy || stale || selected.length === 0} variant="primary" onClick={() => { void run(() => repository.applyDraft(project.id, draft, selected, 'partial'), true); }}>{zh.selectionAcceptSelected.replace('{count}',String(selected.length))}</Button>
   <Button disabled={busy || stale} onClick={() => setConfirmFull(true)}>{zh.selectionReplace}</Button>
  </div>}
  {confirmFull && <div className="draft-full-confirm"><p>{zh.selectionReplaceHint.replace('{count}',String(draft.items.length))}</p><Button disabled={busy} onClick={() => setConfirmFull(false)}>{zh.selectionBack}</Button><Button disabled={busy || stale} variant="primary" onClick={() => { void run(() => repository.applyDraft(project.id, draft, draft.items.map(item => item.photo_id), 'full'), true); }}>{zh.selectionConfirmReplace}</Button></div>}
 </Modal>;
}
