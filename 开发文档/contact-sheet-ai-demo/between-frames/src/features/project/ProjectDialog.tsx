import { useState } from 'react';
import type { Project } from '../../domain/models';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';

// v1.1 项目模板：静态常量，仅预填输入；不是存储数据（ADR 0009）。
export const PROJECT_TEMPLATES: { key: string; label: string; titleHint: string; brief: string }[] = [
  { key: 'blank', label: '空白项目', titleHint: '', brief: '' },
  { key: 'family-book', label: '家庭摄影书', titleHint: '家的日常', brief: '克制的家庭日常，不要摆拍感；注重时间与成长痕迹，人物可以不正面出现。目标：一本可以按时间线翻阅的小书。' },
  { key: 'street', label: '街头纪实', titleHint: '街道切片', brief: '街头瞬间的诚实记录；关注人物与环境的张力、光影结构和偶然的呼应。避免过度摆布和后期痕迹。' },
  { key: 'travel', label: '旅行编辑', titleHint: '某地 · 几日', brief: '以地点与节奏为主线；地标只作锚点，重心放在当地生活质感和旅途中的意外瞬间。成组叙述，不做明信片合集。' },
  { key: 'portfolio', label: '作品集初选', titleHint: '年度作品集', brief: '从长期拍摄中选出代表性作品；主题一致性优先于数量，宁缺毋滥。每组之间要有呼吸和转折。' },
];

export function ProjectDialog({ project, onClose, onSave, busy }: { project?: Project; onClose: () => void; onSave: (title: string, brief: string, tags?: string[]) => void; busy: boolean }) {
  const [title, setTitle] = useState(project?.title ?? '');
  const [brief, setBrief] = useState(project?.brief ?? '');
  const [tags, setTags] = useState((project?.tags ?? []).join('、'));
  const [templateKey, setTemplateKey] = useState('blank');
  const applyTemplate = (key: string) => {
    setTemplateKey(key);
    const template = PROJECT_TEMPLATES.find(item => item.key === key);
    if (template && template.key !== 'blank') {
      // 仅在用户未自行输入时预填，避免覆盖已有草稿。
      if (!title.trim()) setTitle(template.titleHint);
      if (!brief.trim()) setBrief(template.brief);
    }
  };
  return <Modal title={project ? zh.editIntent : zh.newProject} onClose={onClose}>
    {project && <><label htmlFor="project-tags">标签（用、分隔，最多 12 个）</label><input id="project-tags" className="text-input" maxLength={340} value={tags} placeholder="例如：家庭、2026、胶片" onChange={event => setTags(event.target.value)} /></>}
    <form onSubmit={event => { event.preventDefault(); if (title.trim()) onSave(title.trim(), brief.trim(), tags.split(/、|,/).map(tag => tag.trim()).filter(Boolean)); }}>
      {!project && <fieldset className="template-picker"><legend>从模板开始（可选）</legend>
        <div className="template-options">{PROJECT_TEMPLATES.map(template => <button type="button" key={template.key} aria-pressed={templateKey === template.key} onClick={() => applyTemplate(template.key)}>
          {template.label}{template.key !== 'blank' && <small>{template.titleHint}</small>}
        </button>)}</div>
      </fieldset>}
      <label htmlFor="project-title">{zh.projectTitle}</label><input id="project-title" className="text-input" maxLength={80} required value={title} onChange={event => setTitle(event.target.value)} />
      <label htmlFor="project-brief">{zh.projectBrief}</label><textarea id="project-brief" className="text-input" rows={5} maxLength={1500} value={brief} onChange={event => setBrief(event.target.value)} />
      <div className="dialog-actions"><Button type="submit" variant="primary" disabled={!title.trim() || busy}>{project ? zh.saveProject : zh.createProject}</Button></div>
    </form>
  </Modal>;
}
