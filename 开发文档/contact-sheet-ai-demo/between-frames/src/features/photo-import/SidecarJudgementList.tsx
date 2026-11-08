// 外部软件写在原图旁边的判断：先只列事实，逐条确认（或一键填本地还空着的）之后才进台账。
import { useState } from 'react';
import { zh } from '../../app/copy';
import type { DesktopProjectRepository, SidecarProposal } from '../../domain/models';
import { Button } from '../../components/shared/Button';
import { userFacingError } from '../../services/userFacingError';

export function SidecarJudgementList({ repository, projectId, onApplied }: {
  repository: DesktopProjectRepository; projectId: string; onApplied?: () => void;
}) {
  const [proposals, setProposals] = useState<SidecarProposal[] | null>(null);
  const [applied, setApplied] = useState<string[]>([]);
  const [batch, setBatch] = useState<{ applied: number; skipped: number } | null>(null);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  const read = async () => {
    if (!repository.sidecarProposals) return [];
    return repository.sidecarProposals(projectId);
  };
  const scan = async () => {
    if (!repository.sidecarProposals || busy) return;
    setBusy('scan'); setError('');
    try { setProposals(await read()); }
    catch (failure) { setProposals(null); setError(userFacingError(failure, zh.sidecarReadFailed)); }
    finally { setBusy(''); }
  };
  const apply = async (photoId: string) => {
    if (!repository.applySidecarJudgement || busy) return;
    setBusy(photoId); setError('');
    try {
      await repository.applySidecarJudgement(projectId, photoId);
      setApplied(old => [...old, photoId]);
      if (onApplied) onApplied();
    } catch (failure) { setError(userFacingError(failure, zh.sidecarApplyFailed)); }
    finally { setBusy(''); }
  };
  // 一键之后重新读一遍：这一屏说的是现在，不能拿着点之前的清单说"还剩这些"。
  const backfill = async () => {
    if (!repository.backfillSidecarJudgements || busy) return;
    setBusy('batch'); setError('');
    try {
      const outcome = await repository.backfillSidecarJudgements(projectId);
      setBatch({ applied: outcome.applied, skipped: outcome.skipped });
      setApplied([]);
      setProposals(await read());
      if (onApplied) onApplied();
    } catch (failure) { setError(userFacingError(failure, zh.sidecarApplyFailed)); }
    finally { setBusy(''); }
  };
  const stars = (value: number | null) => value === null ? zh.sidecarNoRating : zh.starOption.replace('{count}', String(value));
  const label = (value: string) => zh[`status.${value}`] || value;
  const batchable = proposals?.filter(item => item.batchSafe).length ?? 0;
  return <section className="sidecar-judgements micro" aria-label={zh.sidecarTitle}>
    <p>{zh.sidecarHint}</p>
    <Button size="small" disabled={Boolean(busy) || !repository.sidecarProposals} onClick={() => { void scan(); }}>{busy === 'scan' ? zh.sidecarReading : proposals ? zh.sidecarRescan : zh.sidecarScan}</Button>
    {proposals && !proposals.length && <p className="muted">{zh.sidecarEmpty}</p>}
    {batch && <p role="status">{zh.sidecarBatchDone.replace('{applied}', String(batch.applied)).replace('{skipped}', String(batch.skipped))}</p>}
    {!!proposals?.length && <>
      <p>{zh.sidecarIntro.replace('{count}', String(proposals.length))}</p>
      {/* 一键只碰"本地还空着"的那些；人工判过的仍然一条一条自己点头。 */}
      {repository.backfillSidecarJudgements && batchable > 0 && <Button size="small" disabled={Boolean(busy)} onClick={() => { void backfill(); }}>{busy === 'batch' ? zh.sidecarBatching : zh.sidecarBatch.replace('{count}', String(batchable))}</Button>}
      {repository.backfillSidecarJudgements && batchable === 0 && <p className="muted">{zh.sidecarBatchNone}</p>}
      <ul className="sidecar-judgement-items">{proposals.map(item => {
        const external = [stars(item.sidecarRating), item.sidecarPick === true ? zh.sidecarPicked : item.sidecarPick === false ? zh.sidecarUnpicked : null, item.colorLabel !== null ? zh.sidecarColorLabel.replace('{count}', String(item.colorLabel)) : null]
          .filter((part): part is string => Boolean(part)).join(' · ');
        return <li key={item.photoId}>
          <strong>{item.filename}</strong>
          <span>{zh.sidecarExternalSays}{external} → {zh.sidecarLocalSays}{stars(item.localRating)} · {label(item.localStatus)}</span>
          {applied.includes(item.photoId)
            ? <em role="status">{zh.sidecarApplied}</em>
            : item.appliesRating || item.appliesStatus
              ? repository.applySidecarJudgement && <Button size="small" disabled={Boolean(busy)} onClick={() => { void apply(item.photoId); }}>{busy === item.photoId ? zh.sidecarApplying : zh.sidecarApply}</Button>
              : <em className="muted">{item.ambiguousRating ? zh.sidecarBlockedAmbiguous : item.sidecarPick === false ? zh.sidecarBlockedUnpicked : zh.sidecarBlockedNoColumn}</em>}
        </li>;
      })}</ul>
    </>}
    {error && <p role="alert" className="error-message">{error}</p>}
  </section>;
}
