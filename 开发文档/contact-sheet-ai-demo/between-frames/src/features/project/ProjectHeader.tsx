import { Plus, Sparkles } from 'lucide-react';
import type { Project } from '../../domain/models';
import { zh } from '../../app/copy';
import { Button } from '../../components/shared/Button';

export function ProjectHeader({ project, onEdit, onImport, onAI, readOnly = false }: { project: Project; onEdit: () => void; onImport: () => void; onAI: () => void; readOnly?: boolean }) {
  return <header className="project-header"><div className="eyebrow">{project.id === 'sample-project' ? '示例项目' : ''}</div>
    <div className="heading"><div><h1>{project.title}</h1></div>
      <div className="top-actions"><Button disabled={readOnly} onClick={onImport}><Plus size={15} />{zh.import}</Button>
        <Button disabled={readOnly} variant="primary" onClick={onAI}><Sparkles size={15} />{zh.ai}</Button></div></div>
    <div className="brief"><span className="quote">“</span><p>{project.brief || '添加创作意图'}</p>
      <Button variant="quiet" size="small" disabled={readOnly} onClick={onEdit}>{zh.editIntent} ↗</Button></div>
  </header>;
}
