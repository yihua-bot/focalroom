import type { Photo } from '../../domain/models';
import { Button } from '../shared/Button';
import { zh } from '../../app/copy';

export function SelectionDock({ photos, onSelect, onSequence }: { photos: Photo[]; onSelect: (id: string) => void; onSequence: () => void }) {
  return <footer className="selection-dock"><div className="dock-label"><strong>Selected {String(photos.length).padStart(2, '0')}</strong><small>{zh.dockSelectedSequence}</small></div>
    <div className="dock-images">{photos.length ? photos.map(photo => <button key={photo.id} onClick={() => onSelect(photo.id)} aria-label={zh.dockViewPhoto.replace('{name}', photo.name)}><img src={photo.src} alt={photo.name} /></button>) : <span>{zh.dockNoSelected}</span>}</div>
    <Button size="small" onClick={onSequence}>{zh.editSequenceArrow}</Button>
  </footer>;
}
