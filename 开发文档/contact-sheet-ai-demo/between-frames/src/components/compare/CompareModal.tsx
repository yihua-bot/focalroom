import { useState } from 'react';
import type { Photo } from '../../domain/models';
import { Modal } from '../shared/Modal';
import { Button } from '../shared/Button';
import { zh } from '../../app/copy';
import { PhotoViewport, ZoomControls, fitZoom, type PhotoZoom } from '../shared/PhotoViewport';

export function CompareModal({ left, right, onClose, onCompare, busy }: {
  left: Photo; right: Photo; onClose: () => void; onCompare: (preferred: string | null, reason: string) => void; busy: boolean;
}) {
  const [reason, setReason] = useState('');
  const [linked, setLinked] = useState(true);
  const [zooms, setZooms] = useState([fitZoom, fitZoom]);
  const change = (index: number, zoom: PhotoZoom) => setZooms(previous => linked ? [zoom, zoom] : previous.map((current, i) => i === index ? zoom : current));
  return <Modal title={zh.compareTitle} description={zh.compareHint} onClose={onClose} wide>
    <label className="sync-toggle"><input type="checkbox" checked={linked} onChange={event => { setLinked(event.target.checked); if (event.target.checked) setZooms([zooms[0], zooms[0]]); }} />同步缩放与平移</label>
    <div className="compare-cols">{[left, right].map((photo, i) => <section key={photo.id} aria-label={i ? '右图' : '左图'}>
      <ZoomControls zoom={zooms[i]} onChange={zoom => change(i, zoom)} label={i ? '右图' : '左图'} /><PhotoViewport photo={photo} zoom={zooms[i]} onChange={zoom => change(i, zoom)} />
      <p className="micro">{i ? 'RIGHT' : 'LEFT'} · FRAME {String(photo.index).padStart(2, '0')} · {photo.name}</p>
      <Button disabled={busy} onClick={() => onCompare(photo.id, reason.trim())} variant={i ? 'default' : 'primary'}>{i ? zh.preferRight : zh.preferLeft}</Button></section>)}</div>
    <label htmlFor="compare-reason">{zh.compareReason}</label><textarea id="compare-reason" className="text-input" rows={2} maxLength={2000} value={reason} onChange={event => setReason(event.target.value)} />
    <div className="dialog-actions"><Button disabled={busy} onClick={() => onCompare(null, reason.trim())}>{zh.uncertain}</Button></div>
  </Modal>;
}
