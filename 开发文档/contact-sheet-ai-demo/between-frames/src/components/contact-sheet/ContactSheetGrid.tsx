import { Check, X } from 'lucide-react';
import type { Photo, PhotoStatus } from '../../domain/models';
import { zh } from '../../app/copy';
import { useEffect, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { SelectionGesture } from '../../domain/selection';

type GridProps = {
  photos: Photo[]; activeId: string | null; dense: boolean; busy: boolean;
  selectedIds: string[];
  onSelect: (id: string, gesture?: SelectionGesture) => void; onDecide: (id: string, status: PhotoStatus) => void; onEnlarge: (photo: Photo) => void;
};

export function ContactSheetGrid(props: GridProps) {
  return props.photos.length >= 100 ? <VirtualGrid {...props} /> : <div className={`contact-grid ${props.dense ? 'dense' : ''}`} aria-label="照片网格">{props.photos.map(photo => <PhotoTile key={photo.id} photo={photo} {...props} />)}</div>;
}

function PhotoTile({ photo, activeId, selectedIds, busy, onSelect, onDecide, onEnlarge }: GridProps & { photo: Photo }) {
  const selected = selectedIds.includes(photo.id);
  return (
    <article key={photo.id} className={`tile ${photo.status} ${selected ? 'selected' : ''} ${activeId === photo.id ? 'active' : ''}`} data-testid={`photo-${photo.index}`}>
      <button className="photo-wrap" aria-label={`查看第 ${photo.index} 张 ${photo.name}`} aria-pressed={selected}
        onClick={event => onSelect(photo.id, { toggle: event.metaKey || event.ctrlKey, range: event.shiftKey })} onDoubleClick={() => onEnlarge(photo)}>
        <img loading="lazy" src={photo.src} alt={photo.name} draggable={false} />
        <span className="number">{String(photo.index).padStart(2, '0')}</span>
        {photo.isMissing && <span className="missing-flag">原图失联</span>}
        {photo.status !== 'undecided' && <span className={`flag ${photo.status}`}>{photo.status === 'keep' ? '◇' : '×'} {zh.status[photo.status]}</span>}
      </button><div className="tile-under"><span>FRAME {String(photo.index).padStart(2, '0')}</span><div className="tile-actions">
        <button disabled={busy} aria-label={`标记保留 ${photo.name}`} aria-pressed={photo.status === 'keep'} onClick={() => onDecide(photo.id, 'keep')}><Check size={14} /></button>
        <button disabled={busy} aria-label={`标记淘汰 ${photo.name}`} aria-pressed={photo.status === 'reject'} onClick={() => onDecide(photo.id, 'reject')}><X size={14} /></button>
      </div></div>{photo.rating > 0 && <div className="tile-rating" aria-label={`人工评分 ${photo.rating} 星`}>{'★'.repeat(photo.rating)}</div>}</article>);
}

function VirtualGrid(props: GridProps) {
  const root = useRef<HTMLDivElement>(null);
  const [layout, setLayout] = useState({ columns: 4, width: 800, margin: 0 });
  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const observer = new ResizeObserver(() => {
      const scroll = element.closest('.workspace-scroll');
      const columns = getComputedStyle(element).gridTemplateColumns.split(' ').length;
      const next = { columns, width: element.clientWidth, margin: scroll ? element.getBoundingClientRect().top - scroll.getBoundingClientRect().top + scroll.scrollTop : 0 };
      setLayout(previous => previous.columns === next.columns && previous.width === next.width && Math.abs(previous.margin - next.margin) < 1 ? previous : next);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [props.dense]);
  // 本工程未启用 React Compiler；虚拟列表需保留库提供的可变测量实例。
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({ count: Math.ceil(props.photos.length / layout.columns), getScrollElement: () => root.current?.closest('.workspace-scroll') as HTMLElement | null,
    estimateSize: () => (layout.width - (layout.columns - 1) * 16) / layout.columns / 1.37 + 64, overscan: 3, scrollMargin: layout.margin });
  const index = props.photos.findIndex(photo => photo.id === props.activeId);
  // 折叠面板和调整窗口会改变列宽；清除旧行高估计，避免空隙或重叠。
  useEffect(() => { virtualizer.measure(); }, [layout.width, layout.columns, virtualizer]);
  useEffect(() => { if (index >= 0) virtualizer.scrollToIndex(Math.floor(index / layout.columns), { align: 'auto' }); }, [index, layout.columns, virtualizer]);
  return <div ref={root} aria-label="照片网格" className={`contact-grid virtual-grid ${props.dense ? 'dense' : ''}`} style={{ height: virtualizer.getTotalSize(), position: 'relative', display: 'grid' }}>
    {virtualizer.getVirtualItems().map(row => <div key={row.key} className="virtual-row" style={{ position: 'absolute', top: 0, left: 0, width: '100%', display: 'grid', gridTemplateColumns: `repeat(${layout.columns}, minmax(0, 1fr))`, gap: 16, transform: `translateY(${row.start - layout.margin}px)` }}>
      {props.photos.slice(row.index * layout.columns, (row.index + 1) * layout.columns).map(photo => <PhotoTile key={photo.id} {...props} photo={photo} />)}
    </div>)}
  </div>;
}
