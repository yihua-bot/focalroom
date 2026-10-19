import { useMemo, useState } from 'react';
import { Modal } from '../../components/shared/Modal';
import { EVENT_KIND_LABELS } from '../../services/TauriRepository';
import type { Project } from '../../domain/models';

type ActorFilter = 'all' | 'user' | 'system' | 'agent';

const ACTOR_OPTIONS: { value: ActorFilter; label: string }[] = [
  { value: 'all', label: '全部操作者' },
  { value: 'user', label: '人工' },
  { value: 'system', label: '系统' },
  { value: 'agent', label: 'AI 方案' },
];

/** 按操作者（人工/系统/Agent 草案）与版本恢复点浏览操作历史；只读不改写。 */
export function ActivityDialog({ project, onClose }: { project: Project; onClose: () => void }) {
  const [actor, setActor] = useState<ActorFilter>('all');
  const [kind, setKind] = useState('all');
  const [versionId, setVersionId] = useState('all');
  const kinds = useMemo(() => [...new Set(project.events.map(event => event.type))], [project.events]);
  const versions = project.versions ?? [];
  const since = versionId === 'all' ? null : (versions.find(version => version.id === versionId)?.createdAt ?? null);
  const events = useMemo(() => project.events
    .filter(event =>
      (actor === 'all' || (actor === 'agent' ? event.type.startsWith('draft_') : event.actor === actor))
      && (kind === 'all' || event.type === kind)
      && (!since || event.at >= since))
    .sort((a, b) => b.at.localeCompare(a.at)), [project.events, actor, kind, since]);
  return <Modal title="操作历史" description="按操作者与版本恢复点查看全部正式变更；回滚走版本恢复，不改写历史。" onClose={onClose}>
    <div className="activity-filters">
      <label>操作者
        <select aria-label="按操作者筛选" value={actor} onChange={event => setActor(event.target.value as ActorFilter)}>
          {ACTOR_OPTIONS.map(option => <option key={option.value} value={option.value}>{option.label}</option>)}
        </select>
      </label>
      <label>类型
        <select aria-label="按类型筛选" value={kind} onChange={event => setKind(event.target.value)}>
          <option value="all">全部类型</option>
          {kinds.map(item => <option key={item} value={item}>{EVENT_KIND_LABELS[item] ?? item}</option>)}
        </select>
      </label>
      <label>版本
        <select aria-label="按版本恢复点筛选" value={versionId} onChange={event => setVersionId(event.target.value)}>
          <option value="all">全部版本</option>
          {versions.map(version => <option key={version.id} value={version.id}>{version.label || '版本'} · {new Date(version.createdAt).toLocaleString('zh-CN')}</option>)}
        </select>
      </label>
    </div>
    {versionId !== 'all' && since && <p className="micro">显示 {new Date(since).toLocaleString('zh-CN')} 该版本保存以来的操作。</p>}
    <ul className="activity-list" aria-label="操作历史列表">
      {events.map(event => <li key={event.id} className="activity-item">
        <span className="activity-actor" data-actor={event.actor === 'system' ? 'system' : event.type.startsWith('draft_') ? 'agent' : 'user'}>
          {event.actor === 'system' ? '系统' : event.type.startsWith('draft_') ? 'AI' : '人工'}
        </span>
        <span className="activity-body">
          <strong>{event.description}</strong>
          {event.photoId && <small> · {project.photos.find(photo => photo.id === event.photoId)?.name ?? event.photoId}</small>}
          {event.type === 'version_save' && <em className="activity-restore-point">恢复点</em>}
        </span>
        <time>{new Date(event.at).toLocaleString('zh-CN')}</time>
      </li>)}
      {!events.length && <li className="activity-empty">当前筛选没有匹配的操作。</li>}
    </ul>
  </Modal>;
}
