import { useRef, useState } from 'react';
import type { DesktopProjectRepository, ProjectDraftRecheck, ProjectPreviewAudit } from '../../domain/models';
import { previewRepairLabel } from '../../components/review/previewRepair';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
import { userFacingError } from '../../services/userFacingError';

/**
 * 本地预览能不能读出来，是这个项目自己的文件事实，和某一份审稿方案没有关系，
 * 所以这条状态常驻在图库一侧；方案弹窗里只留与那份方案口径有关的重算。
 */
export function ProjectPreviewHealth({ repository, projectId }: { repository?: DesktopProjectRepository; projectId: string }) {
  const [audit, setAudit] = useState<ProjectPreviewAudit | null>(null);
  const [repaired, setRepaired] = useState<ProjectPreviewAudit | null>(null);
  const [recheck, setRecheck] = useState<ProjectDraftRecheck | null>(null);
  const [recheckError, setRecheckError] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const lock = useRef(false);
  if (!repository?.auditPreviews) return null;
  const act = async (task: () => Promise<void>) => {
    if (lock.current) return;
    lock.current = true; setBusy(true); setError('');
    try { await task(); }
    catch (error) { setError(userFacingError(error)); }
    finally { lock.current = false; setBusy(false); }
  };
  // 数一遍只读文件头，一个字节都不改；修的时候才动那两份按内容指纹命名的缓存，原图始终只读。
  const count = () => { void act(async () => { setAudit(await repository.auditPreviews!(projectId)); setRepaired(null); setRecheck(null); setRecheckError(''); }); };
  const repairAll = () => { void act(async () => {
    const report = await repository.repairAllPreviews!(projectId);
    setRepaired(report);
    setRecheck(report.recheck ?? null);
    setRecheckError(report.recheckError?.message ?? '');
    // 修完再数一遍：这条状态说的是现在，不能把动手之前那一遍的数字留着冒充现状。
    setAudit(await repository.auditPreviews!(projectId));
  }); };
  // 一轮算不完的口径接着算：这条通道不碰照片文件，所以不必先把预览再修一遍。
  const continueRecheck = () => { void act(async () => {
    setRecheck(await repository.recheckProjectEvidence!(projectId));
    setRecheckError('');
  }); };
  const hidden = audit ? audit.needsRepair + audit.blocked - audit.items.length : 0;
  const groups = Object.entries((audit?.items ?? []).reduce<Record<string, string[]>>((names, item) => {
    (names[item.outcome] ??= []).push(item.filename);
    return names;
  }, {}));
  return <section className="project-preview-health micro" aria-label={zh.projectPreviewHealthTitle}>
    <p>{zh.projectPreviewHealthHint}</p>
    <div className="project-preview-health-actions">
      <Button size="small" disabled={busy} onClick={count}>{zh.projectPreviewAudit}</Button>
      {/* 只有确实能重算的那批才摆这个按钮：原图丢了或换过的，这个动作修不了，不拿它当万能解。 */}
      {repository.repairAllPreviews && audit && audit.needsRepair > 0 && <Button size="small" disabled={busy} onClick={repairAll}>{zh.projectPreviewRepairAll}</Button>}
      {/* 还欠着口径才摆这个按钮：欠账数来自回执，不是界面自己猜"应该都算过了"。 */}
      {repository.recheckProjectEvidence && (!!recheckError || (recheck?.recheckRemaining ?? 0) > 0) && <Button size="small" disabled={busy} onClick={continueRecheck}>{zh.projectPreviewRecheckContinue}</Button>}
    </div>
    {audit && <p role="status">{audit.needsRepair + audit.blocked === 0
      ? zh.projectPreviewClean.replace('{total}', String(audit.total))
      : zh.projectPreviewAuditDone.replace('{total}', String(audit.total)).replace('{repair}', String(audit.needsRepair)).replace('{blocked}', String(audit.blocked))}</p>}
    {audit && Object.entries(audit.reasons).map(([reason, total]) => <p key={reason}>{previewRepairLabel(reason)} · {zh.projectPreviewReasonCount.replace('{count}', String(total))}</p>)}
    {audit && audit.scanned < audit.total && <p>{zh.projectPreviewScanCapped.replace('{scanned}', String(audit.scanned)).replace('{total}', String(audit.total))}</p>}
    {repaired && <p role="status">{zh.projectPreviewRepairDone.replace('{repaired}', String(repaired.repaired ?? 0)).replace('{remaining}', String(repaired.remaining ?? 0)).replace('{rechecked}', String(repaired.recheck?.recheckedDrafts ?? 0))}</p>}
    {recheckError && <p role="alert">{zh.projectPreviewRecheckError.replace('{message}', recheckError)}</p>}
    {recheck && <p role="status">{recheck.recheckRemaining > 0
      ? zh.projectPreviewRecheckOwed.replace('{remaining}', String(recheck.recheckRemaining)).replace('{total}', String(recheck.drafts))
      : zh.projectPreviewRecheckClear.replace('{count}', String(recheck.recheckedDrafts)).replace('{scanned}', String(recheck.draftsScanned))}</p>}
    {recheck && recheck.recheckFailed > 0 && <p>{zh.projectPreviewRecheckFailed.replace('{count}', String(recheck.recheckFailed))}</p>}
    {recheck && recheck.draftsScanned < recheck.drafts && <p>{zh.projectPreviewDraftScanCapped.replace('{scanned}', String(recheck.draftsScanned)).replace('{total}', String(recheck.drafts))}</p>}
    {!!groups.length && <details>
      <summary>{zh.projectPreviewNames}</summary>
      <ul>{groups.map(([outcome, names]) => <li key={outcome}><strong>{previewRepairLabel(outcome)}</strong> · {names.join('、')}</li>)}</ul>
      {hidden > 0 && <p>{zh.projectPreviewListTruncated.replace('{shown}', String(audit!.items.length)).replace('{hidden}', String(hidden))}</p>}
    </details>}
    {error && <p role="alert">{error}</p>}
  </section>;
}