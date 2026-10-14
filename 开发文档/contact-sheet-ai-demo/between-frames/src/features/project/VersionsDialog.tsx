import { useState } from 'react';
import type { Project } from '../../domain/models';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';

export function VersionsDialog({ project, busy, onSave, onRestore, onClose }: {
  project: Project; busy: boolean; onSave: (label: string) => void; onRestore: (id: string) => void; onClose: () => void;
}) {
  const [label, setLabel] = useState('');
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = project.versions?.find(version => version.id === selectedId);
  const added = selected?.photoIds.filter(id => !project.sequence.includes(id)) ?? [];
  const removed = selected ? project.sequence.filter(id => !selected.photoIds.includes(id)) : [];
  const moved = selected?.photoIds.filter((id, index) => project.sequence.includes(id) && project.sequence.indexOf(id) !== index) ?? [];
  const changed = selected?.photos.filter(saved => {
    const current = project.photos.find(photo => photo.id === saved.photoId);
    return current && (current.status !== saved.status || current.rating !== saved.rating || current.note !== saved.note);
  }).length ?? 0;
  const name = (id: string) => project.photos.find(photo => photo.id === id)?.name ?? id;
  return <Modal title="作品版本" description="保存选片、顺序、人工评分和已保存备注。恢复会形成可撤销的新编辑。" onClose={() => { if (!busy) onClose(); }} wide>
    <form className="version-save" onSubmit={event => { event.preventDefault(); if (label.trim()) { onSave(label.trim()); setLabel(''); } }}>
      <label htmlFor="version-label">版本名称</label><input id="version-label" className="text-input" value={label} maxLength={80} placeholder="例如：初选 · 25 张" onChange={event => setLabel(event.target.value)} />
      <Button type="submit" variant="primary" disabled={busy || !label.trim()}>保存当前版本</Button>
    </form>
    <div className="version-layout"><div className="version-list" aria-label="已保存版本">
      {project.versions?.length ? project.versions.map(version => <button key={version.id} disabled={busy} aria-pressed={selectedId === version.id} onClick={() => setSelectedId(version.id)}>
        <strong>{version.label}</strong><span>{version.photoIds.length} 张 · {new Date(version.createdAt).toLocaleString()}</span>
      </button>) : <p className="muted">尚未保存版本。给当前作品序列留一个回看的节点。</p>}
    </div><section className="version-preview" aria-label="版本恢复预览">
      {selected ? <><h3>{selected.label}</h3><p>恢复后 {selected.photoIds.length} 张：加入 {added.length} 张，移出 {removed.length} 张，位置变化 {moved.length} 张。</p>
        <p className="muted">人工状态、评分或备注变化：{changed} 张。此版本之后导入的照片仍留在项目中，选片状态恢复为待定。</p>
        <ol>{selected.photoIds.map(id => <li key={id}>{name(id)}</li>)}</ol>
        <Button variant="primary" disabled={busy} onClick={() => onRestore(selected.id)}>确认恢复此版本</Button>
      </> : <p className="muted">选择一个已保存版本，先检查恢复范围。</p>}
    </section></div>
  </Modal>;
}
