import { useState } from 'react';
import type { HistoricalPhotoObservation } from '../../domain/revisionContext';
import type { PreviewRepairOutcome } from '../../domain/models';
import { outcomeByPhoto, previewRepairLabel, previewStatusAfterRepair } from './previewRepair';
import { Button } from '../shared/Button';
import { zh } from '../../app/copy';

// 这张的本地文件状态只在此处说一次：修复确认文件能读了，就不能继续挂着"读不出来"或"对不上"那句。
function versionStatus(photo: HistoricalPhotoObservation, outcome?: string) {
  const restored = photo.currentPixelVersion === 'unreadable' || photo.currentPixelVersion === 'mismatch'
    ? previewStatusAfterRepair(outcome) : null;
  if (restored) return restored;
  return photo.currentPixelVersion === 'verified'
    ? zh.chatImageVersionVerified.replace('{hash}', photo.previewSha256?.slice(0, 8) ?? '')
    : photo.currentPixelVersion === 'mismatch' ? zh.chatImageVersionMismatch
    : photo.currentPixelVersion === 'unreadable' ? zh.chatImageVersionUnreadable : zh.chatImageVersionUnrecorded;
}

export function HistoricalObservations({ observations, total, onRepair }: { observations: HistoricalPhotoObservation[]; total: number; onRepair?: (photoIds: string[]) => Promise<PreviewRepairOutcome[]> }) {
  const [outcomes, setOutcomes] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const broken = observations.filter(photo => photo.currentPixelVersion === 'unreadable');
  const repair = async (photoIds: string[]) => {
    if (!onRepair) return;
    setBusy(true);
    try { const done = outcomeByPhoto(await onRepair(photoIds)); setOutcomes(previous => ({ ...previous, ...done })); }
    // 整批请求失败时按每张记"没有结局"，让它们各自的旧说法原样留着。
    catch { setOutcomes(previous => ({ ...previous, ...Object.fromEntries(photoIds.map(id => [id, ''])) })); }
    finally { setBusy(false); }
  };
  return <section className="revision-observations" aria-label={zh.editorialRevisionObservations}>
    <h5>{zh.editorialRevisionObservations} · {total}</h5><p className="muted">{zh.editorialRevisionObservationHint}</p>
    {onRepair && !!broken.length && <p><Button size="small" disabled={busy} onClick={() => { void repair(broken.map(photo => photo.photoId)); }}>{broken.length > 1 ? zh.chatRepairPreviewAll.replace('{count}', String(broken.length)) : zh.chatRepairPreview}</Button></p>}
    {observations.map(photo => <article className="revision-observation" key={`${photo.taskId}:${photo.callId}:${photo.photoId}`}>
      <strong>{photo.photoName}</strong>
      {photo.currentHashMatches && photo.currentPixelVersion === 'verified' && photo.thumbnailPath && <img className="revision-evidence-thumbnail" src={photo.thumbnailPath} alt={photo.photoName} loading="lazy" />}
      {!photo.currentHashMatches && <p role="status">{zh.editorialRevisionPhotoChanged}</p>}
      {photo.currentHashMatches && <p role="status">{versionStatus(photo, outcomes[photo.photoId])}</p>}
      {onRepair && photo.currentPixelVersion === 'unreadable' && <span className="chat-preview-repair">{outcomes[photo.photoId] !== undefined && <small role="status">{previewRepairLabel(outcomes[photo.photoId])}</small>}</span>}
      <ul>{photo.visibleFacts.map((fact,index) => <li key={index}>{fact}</li>)}</ul><p>{photo.interpretation}</p><p className="muted">{photo.uncertainty}</p>
    </article>)}
  </section>;
}
