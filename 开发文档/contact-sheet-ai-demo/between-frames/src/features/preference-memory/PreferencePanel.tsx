import type { PreviewEvent } from '../../domain/models';
import { zh } from '../../app/copy';
import { EmptyState } from '../../components/shared/EmptyState';

export function PreferencePanel({ events }: { events: PreviewEvent[] }) {
  return <section><h3>{zh.memoryTitle}</h3><p className="muted">{zh.memoryHint}</p><div className="stat-row">
    <div className="info-card"><span className="eyebrow">YOUR CHOICES</span><strong className="stat">{events.filter(event => event.type === 'decision').length}</strong><small>人工选择</small></div>
    <div className="info-card"><span className="eyebrow">COMPARISONS</span><strong className="stat">{events.filter(event => event.type === 'compare').length}</strong><small>比较记录</small></div></div>
    <div className="divider" /><div className="eyebrow">RECENT DECISIONS / 人工记录</div>
    {events.length ? <ol className="timeline">{events.slice(0, 20).map(event => <li key={event.id}><time>{new Date(event.at).toLocaleTimeString('zh-CN')}</time><p>{event.description}</p></li>)}</ol>
      : <EmptyState title={zh.noEvents} hint="从一次人工选择开始。" />}
  </section>;
}
