import { useRef, useState } from 'react';
import type { DesktopProjectRepository, ObservationPixelAudit } from '../../domain/models';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
import { userFacingError } from '../../services/userFacingError';

/**
 * 补一条没记过的像素版本不能靠猜，所以这里先把"为什么这条补不了"原样摆出来：
 * 后端给原因码，句子仍然由四套语言各自负责，界面不自己编。
 */
export function pixelOutcomeLabel(outcome: string): string {
  const key = `observationPixel_${outcome}`;
  return zh[key] === key ? outcome : zh[key];
}

/**
 * 加"像素版本"那一列之前留下的历史观察，当年没记交给模型的是哪一份字节，
 * 于是各份方案的画面依据一直用不上它们。这一条只管那批旧记录：点名只读本地文件，
 * 补上的指纹也是本地从原图重算再比对出来的，不上传照片、不问模型、不花额度。
 */
export function ProjectObservationVersions({ repository, projectId }: { repository?: DesktopProjectRepository; projectId: string }) {
  const [ledger, setLedger] = useState<ObservationPixelAudit | null>(null);
  const [applied, setApplied] = useState<ObservationPixelAudit | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const lock = useRef(false);
  if (!repository?.auditObservationVersions) return null;
  const act = async (task: () => Promise<void>) => {
    if (lock.current) return;
    lock.current = true; setBusy(true); setError('');
    try { await task(); }
    catch (error) { setError(userFacingError(error)); }
    finally { lock.current = false; setBusy(false); }
  };
  const count = () => { void act(async () => { setLedger(await repository.auditObservationVersions!(projectId)); setApplied(null); }); };
  // 补完再数一遍：这一条说的是现在还剩多少，不能拿动手之前那一遍的数字留着冒充现状。
  const backfill = () => { void act(async () => {
    const report = await repository.backfillObservationVersions!(projectId);
    setApplied(report);
    setLedger(await repository.auditObservationVersions!(projectId));
  }); };
  const hidden = ledger ? ledger.scanned - ledger.items.length : 0;
  const groups = Object.entries((applied ?? ledger)?.items.reduce<Record<string, string[]>>((names, item) => {
    (names[item.outcome] ??= []).push(item.filename);
    return names;
  }, {}) ?? {});
  return <section className="project-pixel-versions micro" aria-label={zh.observationPixelTitle}>
    <p>{zh.observationPixelHint}</p>
    <div className="project-pixel-versions-actions">
      <Button size="small" disabled={busy} onClick={count}>{zh.observationPixelAudit}</Button>
      {/* 一条都补不了的时候不摆这个按钮：能补的数目来自本地比对，不是界面自己猜的。 */}
      {repository.backfillObservationVersions && ledger && ledger.ready > 0 && <Button size="small" disabled={busy} onClick={backfill}>{zh.observationPixelBackfill}</Button>}
    </div>
    {ledger && <p role="status">{ledger.rows === 0
      ? zh.observationPixelClean
      : zh.observationPixelLedger
        .replace('{rows}', String(ledger.rows))
        .replace('{ready}', String(ledger.ready))
        .replace('{refused}', String(ledger.refused))
        .replace('{photos}', String(ledger.photos))}</p>}
    {ledger && ledger.scanned < ledger.rows && <p>{zh.observationPixelScanCapped.replace('{scanned}', String(ledger.scanned)).replace('{rows}', String(ledger.rows))}</p>}
    {ledger && Object.entries(ledger.reasons).map(([reason, total]) => <p key={reason}>{pixelOutcomeLabel(reason)} · {zh.observationPixelReasonCount.replace('{count}', String(total))}</p>)}
    {applied && <p role="status">{zh.observationPixelDone.replace('{backfilled}', String(applied.backfilled ?? 0)).replace('{remaining}', String(applied.remaining ?? 0))}</p>}
    {applied?.recheckError && <p role="alert">{zh.observationPixelRecheckError.replace('{message}', applied.recheckError.message)}</p>}
    {applied?.recheck && <p role="status">{applied.recheck.recheckRemaining > 0
      ? zh.observationPixelRecheckOwed.replace('{count}', String(applied.recheck.recheckedDrafts)).replace('{remaining}', String(applied.recheck.recheckRemaining))
      : zh.observationPixelRecheckDone.replace('{count}', String(applied.recheck.recheckedDrafts))}</p>}
    {applied?.recheck && applied.recheck.recheckFailed > 0 && <p>{zh.observationPixelRecheckFailed.replace('{count}', String(applied.recheck.recheckFailed))}</p>}
    {!!groups.length && <details>
      <summary>{zh.observationPixelNames}</summary>
      <ul>{groups.map(([outcome, names]) => <li key={outcome}><strong>{pixelOutcomeLabel(outcome)}</strong> · {names.join('、')}</li>)}</ul>
      {hidden > 0 && <p>{zh.observationPixelListTruncated.replace('{shown}', String((applied ?? ledger)!.items.length)).replace('{hidden}', String(hidden))}</p>}
    </details>}
    {error && <p role="alert">{error}</p>}
  </section>;
}
