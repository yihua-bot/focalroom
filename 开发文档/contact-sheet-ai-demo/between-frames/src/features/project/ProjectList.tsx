import { ArrowUpRight, Plus } from 'lucide-react';
import { useState } from 'react';
import type { Project } from '../../domain/models';
import { zh } from '../../app/copy';
import { Button } from '../../components/shared/Button';
import { EmptyState } from '../../components/shared/EmptyState';

export function ProjectList({ projects, onOpen, onCreate, local = false, archived = false, onArchive, onRestore, onToggleArchive }: {
  projects: Project[]; onOpen: (id: string) => void; onCreate: () => void; local?: boolean; archived?: boolean;
  onArchive?: (project: Project) => void; onRestore?: (project: Project) => void; onToggleArchive?: () => void;
}) {
  const [filterTag, setFilterTag] = useState<string | null>(null);
  const allTags = [...new Set(projects.flatMap(project => project.tags ?? []))].sort();
  const visible = filterTag ? projects.filter(project => project.tags?.includes(filterTag)) : projects;
  return <section className="project-list"><span className="eyebrow">YOUR PROJECTS / FOCALROOM</span><div className="heading">
    <div><h1>{zh.projectListTitle}</h1><p>{zh.projectListHint}</p></div><Button variant="primary" onClick={onCreate}><Plus size={15} />{zh.newProject}</Button></div>
    {allTags.length > 0 && <div className="tag-filter" aria-label="按标签筛选">{['全部', ...allTags].map(tag => <button key={tag} aria-pressed={filterTag === (tag === '全部' ? null : tag)} onClick={() => setFilterTag(tag === '全部' ? null : tag)}>{tag}</button>)}</div>}
    <div className="project-list-tools"><p className="preview-banner">{local ? '本机项目缓存 · 打开项目查看共享状态 · 原图只读' : zh.preview}</p>{local && <Button size="small" onClick={onToggleArchive}>{archived ? '返回项目' : '归档项目'}</Button>}</div>
    {visible.length ? <div className="project-cards">{visible.map(project => <article className="project-card" key={project.id}><button className="project-open" onClick={() => { if (archived) onRestore?.(project); else onOpen(project.id); }}>
      <div className="project-cover">{project.photos[0] ? <img src={project.photos[0].src} alt={project.photos[0].name} /> : <span>从一张照片开始。</span>}</div>
      <div className="project-card-body"><span className="eyebrow">{local ? (archived ? 'ARCHIVED PROJECT' : 'PROJECT CACHE') : project.id === 'sample-project' ? 'SAMPLE PROJECT' : 'PREVIEW PROJECT'}</span><h2>{project.title}<ArrowUpRight size={19} /></h2>
        <p>{project.photos.length} 张图像 · {project.sequence.length} 张入选 · {local ? '本机项目缓存' : '内存预览'}</p>
        {(project.tags?.length ?? 0) > 0 && <p className="tag-chips">{project.tags!.map(tag => <span key={tag}>{tag}</span>)}</p>}{project.updatedAt && <p>最近修改 {new Date(project.updatedAt).toLocaleString('zh-CN')}</p>}</div></button>
      {local && <div className="project-card-actions"><Button size="small" onClick={() => { if (archived) onRestore?.(project); else onArchive?.(project); }}>{archived ? '恢复项目' : '归档项目'}</Button></div>}
    </article>)}</div>
      : visible.length === 0 && filterTag ? <EmptyState title="此标签下没有项目" hint="换一个标签，或清除筛选。" action={<Button onClick={() => setFilterTag(null)}>查看全部</Button>} /> : <EmptyState title="还没有项目" hint={zh.projectListHint} action={<Button onClick={onCreate}>{zh.newProject}</Button>} />}
  </section>;
}
