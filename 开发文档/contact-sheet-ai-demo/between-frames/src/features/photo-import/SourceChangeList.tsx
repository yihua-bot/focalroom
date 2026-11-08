// 同一来源文件被外部软件就地改写：这里只报事实，人点头之后那条记录才换成文件现在的内容。
import { useState } from 'react';
import { zh } from '../../app/copy';
import type { DesktopProjectRepository, ImportChange } from '../../domain/models';
import { Button } from '../../components/shared/Button';
import { userFacingError } from '../../services/userFacingError';

const size = (bytes: number) => zh.sizeMegabytes.replace('{value}', (bytes / 1048576).toFixed(1));

export function SourceChangeList({ repository, projectId, changes, onApplied }: {
  repository: DesktopProjectRepository; projectId: string; changes: ImportChange[]; onApplied?: () => void;
}) {
  const [applied, setApplied] = useState<string[]>([]);
  const [busy, setBusy] = useState('');
  const [error, setError] = useState('');
  if (!changes.length) return null;
  const apply = async (photoId: string) => {
    if (!repository.applyPhotoChange || busy) return;
    setBusy(photoId); setError('');
    try {
      await repository.applyPhotoChange(projectId, photoId);
      setApplied(old => [...old, photoId]);
      if (onApplied) onApplied();
    } catch (failure) { setError(userFacingError(failure, zh.changeApplyFailed)); }
    finally { setBusy(''); }
  };
  // 报的是尺寸与字节的差，不是"哪个文件"：要改哪条记录由这条记录在册的原图决定。
  const from = (change: ImportChange) => zh.changeStored.replace('{width}', String(change.previousWidth)).replace('{height}', String(change.previousHeight)).replace('{size}', size(change.previousSize));
  const to = (change: ImportChange) => zh.changeOnDisk.replace('{width}', String(change.incomingWidth)).replace('{height}', String(change.incomingHeight)).replace('{size}', size(change.incomingSize));
  return <section className="import-changes" aria-label={zh.changeTitle}>
    <p>{zh.changeIntro.replace('{count}', String(changes.length))}</p>
    <ul className="import-change-items">{changes.map(change => <li key={change.photoId}>
      <strong>{change.filename}</strong>
      <span>{from(change)} → {to(change)}</span>
      {applied.includes(change.photoId)
        ? <em role="status">{zh.changeApplied}</em>
        : repository.applyPhotoChange && <Button size="small" disabled={Boolean(busy)} onClick={() => { void apply(change.photoId); }}>{busy === change.photoId ? zh.changeApplying : zh.changeApply}</Button>}
    </li>)}</ul>
    {error && <p role="alert" className="error-message">{error}</p>}
  </section>;
}
