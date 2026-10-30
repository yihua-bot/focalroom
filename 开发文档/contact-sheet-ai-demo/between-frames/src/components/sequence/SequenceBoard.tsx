import { DndContext, KeyboardSensor, PointerSensor, closestCenter, useSensor, useSensors } from '@dnd-kit/core';
import { SortableContext, useSortable, arrayMove, rectSortingStrategy, sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Grip, ArrowUp, ArrowDown } from 'lucide-react';
import type { Photo } from '../../domain/models';

function SequenceCard({ photo, index, total, active, busy, onSelect, onOpen, onMove }: {
  photo: Photo; index: number; total: number; active: boolean; busy: boolean;
  onSelect: (id: string) => void; onOpen: (photo: Photo) => void; onMove: (from: number, to: number) => void;
}) {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging, isOver } = useSortable({ id: photo.id, disabled: busy });
  return <article ref={setNodeRef} data-testid="sequence-card" className={`sequence-card ${active ? 'selected' : ''} ${isDragging ? 'dragging' : ''} ${isOver ? 'drop-target' : ''}`}
    style={{ transform: CSS.Transform.toString(transform), transition }}>
    <button className="sequence-photo" onClick={() => onSelect(photo.id)} onDoubleClick={() => onOpen(photo)} aria-label={`查看序列第 ${index + 1} 张 ${photo.name}（双击放大）`}><img src={photo.src} alt={photo.name} draggable={false} /></button>
    <header><span>{String(index + 1).padStart(2, '0')} · {photo.name}</span><button ref={setActivatorNodeRef} className="drag-handle" aria-label={`拖动 ${photo.name}`} {...attributes} {...listeners}><Grip size={17} /></button></header>
    <div className="sequence-moves"><button aria-label={`上移 ${photo.name}`} disabled={busy || index === 0} onClick={() => onMove(index, index - 1)}><ArrowUp size={13} />上移</button>
      <button aria-label={`下移 ${photo.name}`} disabled={busy || index === total - 1} onClick={() => onMove(index, index + 1)}><ArrowDown size={13} />下移</button></div>
  </article>;
}

export function SequenceBoard({ photos, activeId, busy, onSelect, onOpen, onReorder }: {
  photos: Photo[]; activeId: string | null; busy: boolean; onSelect: (id: string) => void; onOpen: (photo: Photo) => void; onReorder: (ids: string[]) => void;
}) {
  // 把手立即启动拖动，避免合并移动事件只触发距离阈值却没有更新位移。
  const sensors = useSensors(useSensor(PointerSensor), useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }));
  const ids = photos.map(photo => photo.id);
  const move = (from: number, to: number) => { if (from >= 0 && to >= 0 && to < ids.length && from !== to) onReorder(arrayMove(ids, from, to)); };
  return <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={({ active, over }) => { if (over) move(ids.indexOf(String(active.id)), ids.indexOf(String(over.id))); }}>
    <SortableContext items={ids} strategy={rectSortingStrategy}><div className="sequence-grid" aria-label="作品序列">
      {photos.map((photo, index) => <SequenceCard key={photo.id} photo={photo} index={index} total={photos.length} active={activeId === photo.id} busy={busy} onSelect={onSelect} onOpen={onOpen} onMove={move} />)}
    </div></SortableContext>
  </DndContext>;
}
