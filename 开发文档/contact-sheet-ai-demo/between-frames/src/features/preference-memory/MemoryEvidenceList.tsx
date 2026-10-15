import type { MemoryEvidence } from '../../domain/memory';
import type { Photo } from '../../domain/models';
import { Button } from '../../components/shared/Button';

const kindLabels: Record<string, string> = { keep: '人工保留', reject: '人工淘汰', comparison: '两图比较', draft_apply: '采纳草案', draft_reject: '丢弃草案', note: '备注' };

function explanation(item: MemoryEvidence, photos: Photo[]) {
  const name = (id: unknown) => typeof id === 'string' ? (photos.find(photo => photo.id === id)?.name ?? id.slice(0, 8)) : '';
  const notes = Array.isArray(item.detail.notes) ? item.detail.notes : [];
  const lines = notes.flatMap(value => typeof value === 'object' && value !== null && 'note' in value && typeof value.note === 'string'
    ? [`${'photo_id' in value ? name(value.photo_id) : ''}：${value.note}`] : []);
  if (typeof item.detail.note === 'string') lines.push(item.detail.note);
  if (typeof item.detail.reason === 'string') lines.push(item.detail.reason);
  const ids = Array.isArray(item.detail.photo_ids) ? item.detail.photo_ids.map(name).filter(Boolean) : [name(item.photoId)].filter(Boolean);
  return { lines, names: ids.join('、') };
}

export function MemoryEvidenceList({ ids, evidence, photos, busy, onExclude, onDelete }: {
  ids: number[]; evidence: MemoryEvidence[]; photos: Photo[]; busy?: boolean;
  onExclude?: (item: MemoryEvidence) => void;
  onDelete?: (item: MemoryEvidence) => void;
}) {
  return <ol className="timeline">{ids.map(id => {
    const item = evidence.find(value => value.id === id);
    if (!item) return <li key={id}>证据 #{id} 暂不可用</li>;
    const detail = explanation(item, photos);
    return <li key={id} data-evidence-id={id}>
      <time>{new Date(item.createdAt).toLocaleString('zh-CN')}</time>
      <p>#{id} · {kindLabels[item.kind]}{detail.names ? ` · ${detail.names}` : ''}{item.excluded ? ' · 已排除学习' : ''}</p>
      {detail.lines.map((line, index) => <p className="micro memory-explanation" key={index}>{line}</p>)}
      {!detail.lines.length && <p className="micro">该操作没有留下文字理由。</p>}
      {onExclude && <Button size="small" disabled={busy} aria-label={`${item.excluded ? '恢复' : '排除'}证据 #${id}`} onClick={() => onExclude(item)}>
        {item.excluded ? '恢复参与学习' : '排除学习'}
      </Button>}
      {onDelete && <Button size="small" disabled={busy} aria-label={`删除证据 #${id}`} onClick={() => { if (window.confirm('物理删除这条证据？不可撤销，引用它的假设将失去该证据（引用清空的假设会退役）。')) onDelete(item); }}>
        删除
      </Button>}
    </li>;
  })}</ol>;
}
