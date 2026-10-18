import { useState } from 'react';
import type { Project, SequenceVersion } from '../../domain/models';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';

type Snapshot = { label: string; photoIds: string[]; photos: { photoId: string; status: string; rating: number; note: string }[] };
const snapshotOf = (project: Project, version?: SequenceVersion): Snapshot => version
  ? { label: version.label, photoIds: version.photoIds, photos: version.photos.map(photo => ({ photoId: photo.photoId, status: photo.status, rating: photo.rating, note: photo.note })) }
  : { label: '当前序列', photoIds: project.sequence, photos: project.photos.map(photo => ({ photoId: photo.id, status: photo.status, rating: photo.rating, note: photo.note })) };

export function VersionsDialog({ project, busy, onSave, onRestore, onClose }: {
  project: Project; busy: boolean; onSave: (label: string) => void; onRestore: (id: string) => void; onClose: () => void;
}) {
  const [label, setLabel] = useState('');
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [mode, setMode] = useState<'restore' | 'compare'>('restore');
  const [baseId, setBaseId] = useState<string | null>(null);
  const [compareId, setCompareId] = useState<string | 'current' | null>(null);
  const selected = project.versions?.find(version => version.id === selectedId);
  const added = selected?.photoIds.filter(id => !project.sequence.includes(id)) ?? [];
  const removed = selected ? project.sequence.filter(id => !selected.photoIds.includes(id)) : [];
  const moved = selected?.photoIds.filter((id, index) => project.sequence.includes(id) && project.sequence.indexOf(id) !== index) ?? [];
  const changed = selected?.photos.filter(saved => {
    const current = project.photos.find(photo => photo.id === saved.photoId);
    return current && (current.status !== saved.status || current.rating !== saved.rating || current.note !== saved.note);
  }).length ?? 0;
  const name = (id: string) => project.photos.find(photo => photo.id === id)?.name ?? id;
  // v1.1 版本对比：A（基准版本）vs B（版本或当前序列）的只读 diff（ADR 0009）。
  const baseVersion = project.versions?.find(version => version.id === baseId);
  const compareVersion = compareId === 'current' ? undefined : project.versions?.find(version => version.id === compareId);
  const base = baseId === null ? null : snapshotOf(project, baseVersion);
  const compare = compareId === null ? null : snapshotOf(project, compareVersion);
  const diff = base && compare ? (() => {
    const addedIds = compare.photoIds.filter(id => !base.photoIds.includes(id));
    const removedIds = base.photoIds.filter(id => !compare.photoIds.includes(id));
    const movedPairs = compare.photoIds
      .map((id, to) => ({ id, from: base.photoIds.indexOf(id), to }))
      .filter(item => item.from >= 0 && item.from !== item.to);
    const stateChanges = compare.photos.map(item => {
      const before = base.photos.find(photo => photo.photoId === item.photoId);
      if (!before) return null;
      const changes: string[] = [];
      if (before.status !== item.status) changes.push(`${before.status} → ${item.status}`);
      if (before.rating !== item.rating) changes.push(`评分 ${before.rating} → ${item.rating}`);
      if (before.note !== item.note) changes.push('备注已修改');
      return changes.length ? { id: item.photoId, changes } : null;
    }).filter(Boolean) as { id: string; changes: string[] }[];
    return { addedIds, removedIds, movedPairs, stateChanges };
  })() : null;
  return <Modal title="作品版本" onClose={() => { if (!busy) onClose(); }} wide>
    <form className="version-save" onSubmit={event => { event.preventDefault(); if (label.trim()) { onSave(label.trim()); setLabel(''); } }}>
      <label htmlFor="version-label">版本名称</label><input id="version-label" className="text-input" value={label} maxLength={80} placeholder="例如：初选 · 25 张" onChange={event => setLabel(event.target.value)} />
      <Button type="submit" variant="primary" disabled={busy || !label.trim()}>保存当前版本</Button>
    </form>
    <div className="version-toolbar" role="tablist" aria-label="版本视图">
      <button role="tab" aria-selected={mode === 'restore'} onClick={() => setMode('restore')}>恢复预览</button>
      <button role="tab" aria-selected={mode === 'compare'} onClick={() => setMode('compare')}>版本对比</button>
    </div>
    {mode === 'restore' ? <div className="version-layout"><div className="version-list" aria-label="已保存版本">
      {project.versions?.length ? project.versions.map(version => <button key={version.id} disabled={busy} aria-pressed={selectedId === version.id} onClick={() => setSelectedId(version.id)}>
        <strong>{version.label}</strong><span>{version.photoIds.length} 张 · {new Date(version.createdAt).toLocaleString()}</span>
      </button>) : <p className="muted">暂无已保存版本。</p>}
    </div><section className="version-preview" aria-label="版本恢复预览">
      {selected ? <><h3>{selected.label}</h3><p>恢复后 {selected.photoIds.length} 张：加入 {added.length} 张，移出 {removed.length} 张，位置变化 {moved.length} 张。</p>
        <p className="muted">人工状态、评分或备注变化：{changed} 张。此版本之后导入的照片仍留在项目中，选片状态恢复为待定。</p>
        <ol>{selected.photoIds.map(id => <li key={id}>{name(id)}</li>)}</ol>
        <Button variant="primary" disabled={busy} onClick={() => onRestore(selected.id)}>确认恢复此版本</Button>
      </> : <p className="muted">选择一个已保存版本，先检查恢复范围。</p>}
    </section></div> : <div className="version-compare" aria-label="版本对比">
      <div className="compare-pickers">
        <label>基准（A）<select aria-label="基准版本" value={baseId ?? ''} onChange={event => setBaseId(event.target.value || null)}>
          <option value="">选择版本…</option>
          {project.versions?.map(version => <option key={version.id} value={version.id}>{version.label}</option>)}
        </select></label>
        <span>对比</span>
        <label>比较（B）<select aria-label="对比版本" value={compareId ?? ''} onChange={event => setCompareId(event.target.value || null)}>
          <option value="">选择版本…</option>
          <option value="current">当前序列</option>
          {project.versions?.map(version => <option key={version.id} value={version.id}>{version.label}</option>)}
        </select></label>
      </div>
      {diff ? <div className="compare-result">
        <h3>{base!.label} → {compare!.label}</h3>
        <p>新增 {diff.addedIds.length} 张 · 移除 {diff.removedIds.length} 张 · 移位 {diff.movedPairs.length} 张 · 状态/评分/备注变化 {diff.stateChanges.length} 张</p>
        {diff.addedIds.length > 0 && <><h4>B 新增</h4><ul>{diff.addedIds.map(id => <li key={id}>{name(id)}</li>)}</ul></>}
        {diff.removedIds.length > 0 && <><h4>B 移除</h4><ul>{diff.removedIds.map(id => <li key={id}>{name(id)}</li>)}</ul></>}
        {diff.movedPairs.length > 0 && <><h4>位置变化</h4><ul>{diff.movedPairs.map(item => <li key={item.id}>{name(item.id)}：第 {item.from + 1} 位 → 第 {item.to + 1} 位</li>)}</ul></>}
        {diff.stateChanges.length > 0 && <><h4>人工判断变化</h4><ul>{diff.stateChanges.map(item => <li key={item.id}>{name(item.id)}：{item.changes.join('；')}</li>)}</ul></>}
      </div> : <p className="muted">选择要比较的版本。</p>}
    </div>}
  </Modal>;
}
