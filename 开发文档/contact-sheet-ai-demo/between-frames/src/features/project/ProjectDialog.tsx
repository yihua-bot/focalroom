import { useState } from 'react';
import type { Project } from '../../domain/models';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';

export function ProjectDialog({ project, onClose, onSave, busy }: { project?: Project; onClose: () => void; onSave: (title: string, brief: string) => void; busy: boolean }) {
  const [title, setTitle] = useState(project?.title ?? '');
  const [brief, setBrief] = useState(project?.brief ?? '');
  return <Modal title={project ? zh.editIntent : zh.newProject} description="写下作品名称与创作意图，作为每次判断的背景。" onClose={onClose}>
    <form onSubmit={event => { event.preventDefault(); if (title.trim()) onSave(title.trim(), brief.trim()); }}>
      <label htmlFor="project-title">{zh.projectTitle}</label><input id="project-title" className="text-input" maxLength={80} required value={title} onChange={event => setTitle(event.target.value)} />
      <label htmlFor="project-brief">{zh.projectBrief}</label><textarea id="project-brief" className="text-input" rows={5} maxLength={1500} value={brief} onChange={event => setBrief(event.target.value)} />
      <div className="dialog-actions"><Button type="submit" variant="primary" disabled={!title.trim() || busy}>{project ? zh.saveProject : zh.createProject}</Button></div>
    </form>
  </Modal>;
}
