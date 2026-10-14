import { ArrowUpRight, Plus } from 'lucide-react';
import type { Project } from '../../domain/models';
import { zh } from '../../app/copy';
import { Button } from '../../components/shared/Button';
import { EmptyState } from '../../components/shared/EmptyState';

export function ProjectList({ projects, onOpen, onCreate, local = false, archived = false, onArchive, onRestore, onToggleArchive }: {
  projects: Project[]; onOpen: (id: string) => void; onCreate: () => void; local?: boolean; archived?: boolean;
  onArchive?: (project: Project) => void; onRestore?: (project: Project) => void; onToggleArchive?: () => void;
}) {
  return <section className="project-list"><span className="eyebrow">YOUR PROJECTS / 序间</span><div className="heading">
    <div><h1>{zh.projectListTitle}</h1><p>{zh.projectListHint}</p></div><Button variant="primary" onClick={onCreate}><Plus size={15} />{zh.newProject}</Button></div>
    <div className="project-list-tools"><p className="preview-banner">{local ? '本地项目 · 自动保存 · 原图只读' : zh.preview}</p>{local && <Button size="small" onClick={onToggleArchive}>{archived ? '返回项目' : '归档项目'}</Button>}</div>
    {projects.length ? <div className="project-cards">{projects.map(project => <article className="project-card" key={project.id}><button className="project-open" onClick={() => { if (archived) onRestore?.(project); else onOpen(project.id); }}>
      <div className="project-cover">{project.photos[0] ? <img src={project.photos[0].src} alt={project.photos[0].name} /> : <span>从一张照片开始。</span>}</div>
      <div className="project-card-body"><span className="eyebrow">{local ? (archived ? 'ARCHIVED PROJECT' : 'LOCAL PROJECT') : project.id === 'sample-project' ? 'SAMPLE PROJECT' : 'PREVIEW PROJECT'}</span><h2>{project.title}<ArrowUpRight size={19} /></h2>
        <p>{project.photos.length} 张图像 · {project.sequence.length} 张入选 · 本地项目</p>{project.updatedAt && <p>最近修改 {new Date(project.updatedAt).toLocaleString('zh-CN')}</p>}</div></button>
      {local && <div className="project-card-actions"><Button size="small" onClick={() => { if (archived) onRestore?.(project); else onArchive?.(project); }}>{archived ? '恢复项目' : '归档项目'}</Button></div>}
    </article>)}</div>
      : <EmptyState title="还没有项目" hint={zh.projectListHint} action={<Button onClick={onCreate}>{zh.newProject}</Button>} />}
  </section>;
}
