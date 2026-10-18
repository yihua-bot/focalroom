import type { Photo } from '../../domain/models';
import { Button } from '../shared/Button';

export function SelectionDock({ photos, onSelect, onSequence }: { photos: Photo[]; onSelect: (id: string) => void; onSequence: () => void }) {
  return <footer className="selection-dock"><div className="dock-label"><strong>Selected {String(photos.length).padStart(2, '0')}</strong><small>作品初选序列</small></div>
    <div className="dock-images">{photos.length ? photos.map(photo => <button key={photo.id} onClick={() => onSelect(photo.id)} aria-label={`查看入选 ${photo.name}`}><img src={photo.src} alt={photo.name} /></button>) : <span>暂无入选照片</span>}</div>
    <Button size="small" onClick={onSequence}>编辑序列 →</Button>
  </footer>;
}
