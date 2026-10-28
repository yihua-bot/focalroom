import type { PublicationLayerView } from '../../domain/publicationLayers';
import { useState,useEffect,useMemo } from 'react';
import type { PublicationLayout, PublicationPreview } from '../../domain/publication';
import { Button } from '../../components/shared/Button';
import { PublicationPaper } from './PublicationPaper';
import { zh } from '../../app/copy';
import { publicationPageName } from './publicationSpreads';
import { DndContext,KeyboardSensor,PointerSensor,closestCenter,pointerWithin,useSensor,useSensors } from '@dnd-kit/core';
import { SortableContext,useSortable,rectSortingStrategy,sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Grip } from 'lucide-react';
import { publicationContinuousPairs,type ContinuousFrame } from '../../domain/publicationContinuous';
import { publicationMovePage } from './publicationPageOrder';

function PageThumbnail({page,previews,active,disabled,reorderable,onSelect,continuous,layers}:{page:PublicationLayout['pages'][number];previews:PublicationPreview[];active:boolean;disabled:boolean;reorderable:boolean;onSelect:()=>void;continuous:ContinuousFrame[];layers:PublicationLayerView[]}){
  const movable=reorderable&&page.pageNumber>0&&!disabled;
  const {attributes,listeners,setNodeRef,setActivatorNodeRef,transform,transition,isDragging,isOver}=useSortable({id:page.id,disabled:!movable});
  return <article ref={setNodeRef} className={`publication-page-item${isDragging?' is-dragging':''}${isOver?' is-over':''}`} style={{transform:CSS.Transform.toString(transform),transition}}>
    <button type="button" className="publication-page-thumb" disabled={disabled} aria-current={active?'page':undefined} aria-label={publicationPageName(page.pageNumber)} onClick={onSelect}><PublicationPaper page={page} previews={previews} thumbnail label="" continuous={continuous} layers={layers}/><span>{publicationPageName(page.pageNumber)}{continuous.some(item=>item.left.id===page.id||item.right.id===page.id)&&<small className="publication-page-pair">{zh.publicationContinuousPhoto}</small>}</span></button>
    {reorderable&&page.pageNumber>0&&<button type="button" ref={setActivatorNodeRef} className="publication-page-drag" disabled={!movable} {...attributes} {...listeners} aria-roledescription={zh.publicationPages} aria-label={zh.publicationDragPage.replace('{page}',publicationPageName(page.pageNumber))}><Grip size={15}/></button>}
  </article>;
}
export function PublicationPages({ pages, previews, activeId, disabled, onSelect,onReorder,onDragActiveChange,onInvalidReorder,layers=[] }: {
  pages: PublicationLayout['pages']; previews: PublicationPreview[]; activeId?: string;
  disabled: boolean; onSelect: (id: string) => void;onReorder?:(ids:string[],activeId:string)=>void;
  layers?:PublicationLayerView[];onDragActiveChange?:(active:boolean)=>void;onInvalidReorder?:()=>void;
}) {
  useEffect(()=>()=>onDragActiveChange?.(false),[onDragActiveChange]);
  const sensors=useSensors(useSensor(PointerSensor),useSensor(KeyboardSensor,{coordinateGetter:sortableKeyboardCoordinates}));
  const [offset, setOffset] = useState<{start:number;anchorId?:string}>();
  const continuous=useMemo(()=>publicationContinuousPairs(pages),[pages]);
  const activeIndex = Math.max(0, pages.findIndex(page => page.id === activeId));
  const start = Math.min(offset?.anchorId===activeId?offset?.start??Math.floor(activeIndex/40)*40:Math.floor(activeIndex/40)*40, Math.max(0, Math.floor((pages.length - 1) / 40) * 40));
  const name=(id:unknown)=>publicationPageName(pages.find(page=>page.id===id)?.pageNumber??1);
  return <DndContext sensors={sensors} collisionDetection={args=>args.pointerCoordinates?pointerWithin(args):closestCenter(args)} onDragStart={()=>onDragActiveChange?.(true)} onDragCancel={()=>onDragActiveChange?.(false)} accessibility={{screenReaderInstructions:{draggable:zh.publicationPageDragKeyboard},announcements:{onDragStart:({active})=>zh.publicationPageDragPicked.replace('{page}',name(active.id)),onDragOver:({over})=>over?zh.publicationPageDragOver.replace('{page}',name(over.id)):zh.publicationPageDragOutside,onDragEnd:()=>zh.publicationPageDragFinished,onDragCancel:()=>zh.publicationPageDragCancelled}}} onDragEnd={({active,over})=>{onDragActiveChange?.(false);if(disabled||!onReorder||!over)return;const order=publicationMovePage(pages,String(active.id),String(over.id));if(order)onReorder(order,String(active.id));else if(active.id!==over.id)onInvalidReorder?.();}}>
    <SortableContext items={pages.slice(start,start+40).map(page=>page.id)} strategy={rectSortingStrategy}><div className="publication-page-list">
    {onReorder&&<p className="publication-page-drag-hint">{continuous.length?zh.publicationContinuousOrderHint:zh.publicationPageDragHint}</p>}
    {pages.slice(start, start + 40).map(page=><PageThumbnail key={page.id} page={page} previews={previews} active={page.id===activeId} disabled={disabled} reorderable={!!onReorder} continuous={continuous} layers={layers} onSelect={()=>onSelect(page.id)}/>)}
    {pages.length > 40 && <div className="publication-page-pagination"><Button size="small" disabled={disabled || start === 0} onClick={() => setOffset({start:Math.max(0,start-40),anchorId:activeId})}>{zh.canvasPhotoPrevious}</Button><span>{Math.floor(start / 40) + 1} / {Math.ceil(pages.length / 40)}</span><Button size="small" disabled={disabled || start + 40 >= pages.length} onClick={() => setOffset({start:start+40,anchorId:activeId})}>{zh.canvasPhotoNext}</Button></div>}
  </div></SortableContext></DndContext>;
}
