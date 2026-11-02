import { ArrowUpRight, Plus } from 'lucide-react';
import { useState } from 'react';
import type { Project } from '../../domain/models';
import { zh } from '../../app/copy';
import { Button } from '../../components/shared/Button';
import { AdminStats } from '../../components/shared/AdminStats';
import { EmptyState } from '../../components/shared/EmptyState';
import { getCopyLocale } from '../../app/copy';
import { formatDate } from '../../i18n/format';

export function ProjectList({ projects, onOpen, onCreate, onQuickStart, local = false, archived = false, onArchive, onRestore, onDelete, onToggleArchive }: {
  projects: Project[]; onOpen: (id: string) => void; onCreate: () => void;
  onQuickStart?: () => void; local?: boolean; archived?: boolean;
  onArchive?: (project: Project) => void; onRestore?: (project: Project) => void; onDelete?: (project: Project) => void; onToggleArchive?: () => void;
}) {
  const [filterTag, setFilterTag] = useState<string | null>(null);
  const allTags = [...new Set(projects.flatMap(project => project.tags ?? []))].sort();
  const visible = filterTag ? projects.filter(project => project.tags?.includes(filterTag)) : projects;
  return <section className="project-list"><span className="eyebrow">YOUR PROJECTS / FOCALROOM</span><div className="heading">
    <div><h1>{zh.projectListTitle}</h1></div><div className="top-actions">{local && !archived && onQuickStart && <Button onClick={onQuickStart}>{zh["project.quickStart"]}</Button>}<Button variant="primary" onClick={onCreate}><Plus size={15} />{zh.newProject}</Button></div></div>
    {allTags.length > 0 && <div className="tag-filter" aria-label={zh.filterByTagAria}>{[zh['common.all'], ...allTags].map(tag => <button key={tag} aria-pressed={filterTag === (tag === zh['common.all'] ? null : tag)} onClick={() => setFilterTag(tag === zh['common.all'] ? null : tag)}>{tag}</button>)}</div>}
    <div className="project-list-tools">{!local && <p className="preview-banner">{zh.preview}</p>}{local && <Button size="small" onClick={onToggleArchive}>{archived ? zh.backToProjects : zh.archiveProject}</Button>}</div>
    {visible.length ? <div className="project-cards">{visible.map(project => <article className="project-card" key={project.id}><button className="project-open" aria-label={project.title} onClick={() => { if (archived) onRestore?.(project); else onOpen(project.id); }}>
      <div className="project-cover">{project.photos[0] ? <img src={project.photos[0].src} alt={project.photos[0].name} /> : <span>{zh.emptyTitle}</span>}</div>
      <div className="project-card-body">{(archived || !local) && <span className="eyebrow">{archived ? zh.archivedLabel : zh.demoProject}</span>}<h2>{project.title}<ArrowUpRight size={19} /></h2>
        <p>{zh.cardStats.replace('{photos}', String(project.photos.length)).replace('{selected}', String(project.sequence.length))}</p>
        {(project.tags?.length ?? 0) > 0 && <p className="tag-chips">{project.tags!.map(tag => <span key={tag}>{tag}</span>)}</p>}{project.updatedAt && <p>{zh.lastModified} {formatDate(project.updatedAt, getCopyLocale())}</p>}</div></button>
      {local && <div className="project-card-actions">{archived && onDelete && <Button size="small" onClick={() => onDelete(project)}>{zh.deleteProject}</Button>}<Button size="small" onClick={() => { if (archived) onRestore?.(project); else onArchive?.(project); }}>{archived ? zh.restoreProject : zh.archiveProject}</Button></div>}
    </article>)}<AdminStats /></div>
      : visible.length === 0 && filterTag ? <EmptyState title={zh.emptyTagTitle} hint={zh.emptyTagHint} action={<Button onClick={() => setFilterTag(null)}>{zh.allPhotos}</Button>} /> : <EmptyState title={zh.emptyNoProjects} hint={zh.projectListHint} action={<Button onClick={onCreate}>{zh.newProject}</Button>} />}
  </section>;
}
