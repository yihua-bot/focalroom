import { useState } from 'react';
import { ChevronRight } from 'lucide-react';
import type { DesktopProjectRepository } from '../../domain/models';
import { zh } from '../../app/copy';
import { ProjectObservationVersions } from './ProjectObservationVersions';
import { ProjectPreviewHealth } from './ProjectPreviewHealth';
import { SidecarJudgementList } from '../photo-import/SidecarJudgementList';

/**
 * 三条本机自查平时折成一行：它们说的是文件事实，不该长期占住选片网格的位置。
 * 收起用 hidden 而不是不渲染，展开过的数字与名单不会因为收起就要再点一次。
 */
export function LocalFileChecks({ repository, projectId, onApplied }: {
  repository?: DesktopProjectRepository; projectId: string; onApplied?: () => void;
}) {
  const [open, setOpen] = useState(false);
  const preview = Boolean(repository?.auditPreviews);
  const pixels = Boolean(repository?.auditObservationVersions);
  const sidecar = Boolean(repository?.sidecarProposals);
  if (!preview && !pixels && !sidecar) return null;
  return <section className="local-checks micro" aria-label={zh.localChecksTitle}>
    <button type="button" className="local-checks-toggle" aria-expanded={open} onClick={() => setOpen(value => !value)}>
      <ChevronRight size={13} aria-hidden="true" /><strong>{zh.localChecksTitle}</strong><span>{zh.localChecksHint}</span>
    </button>
    <div className="local-checks-body" hidden={!open}>
      {preview && <ProjectPreviewHealth repository={repository} projectId={projectId} />}
      {pixels && <ProjectObservationVersions repository={repository} projectId={projectId} />}
      {sidecar && repository && <SidecarJudgementList repository={repository} projectId={projectId} onApplied={onApplied} />}
    </div>
  </section>;
}
