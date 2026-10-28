import { useState,type ReactNode } from 'react';
import type { Photo } from '../../domain/models';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import '../canvas/canvas-photo-picker.css';
import { projectPhotoSources,type ProjectPhotoSource } from './projectPhotoSources';

export type PhotoPickerSource = ProjectPhotoSource;
export interface PhotoPickerCopy {
  title:string;description:string;sourceLabel:string;sourceNames:Record<PhotoPickerSource,string>;sourceHints:Record<PhotoPickerSource,string>;
  search:string;selectAll:string;clear:string;placed:string;empty:string;selected:string;previous:string;next:string;cancel:string;adding:string;addCount:string;
}
export function ProjectPhotoPicker({photos,sequencePhotoIds,selectedPhotoIds,placedPhotoIds=[],busy,error,disabled=false,canAdd=true,onClose,onAdd,copy,maxSelected=300,renderPreview,tools,className=''}:{
  photos:Photo[];sequencePhotoIds:string[];selectedPhotoIds:string[];placedPhotoIds?:string[];busy:boolean;error?:string;disabled?:boolean;canAdd?:boolean;onClose:()=>void;onAdd:(ids:string[])=>void;
  copy:PhotoPickerCopy;maxSelected?:number;renderPreview?:(photos:Photo[])=>ReactNode;tools?:ReactNode;className?:string;
}) {
  const [source, setSource] = useState<PhotoPickerSource>(sequencePhotoIds.length ? 'sequence' : 'all');
  const [chosen, setChosen] = useState<Set<string>>(new Set());
  const [query, setQuery] = useState('');
  const [page, setPage] = useState(0);
  const sources = projectPhotoSources(photos,sequencePhotoIds,selectedPhotoIds);
  const available = sources[source].filter(photo => photo.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()));
  const visible = available.slice(page * 60, (page + 1) * 60);
  const placed = new Set(placedPhotoIds);
  const selectable = available.filter(photo => !placed.has(photo.id));
  // 放置顺序来自所选来源；搜索与分页不改变照片顺序或作品序列。
  const ids = sources[source].filter(photo => chosen.has(photo.id) && !placed.has(photo.id)).map(photo => photo.id);
  return <Modal wide className={className} title={copy.title} description={copy.description} onClose={() => { if (!busy) onClose(); }}>
    <div className="canvas-photo-sources" role="group" aria-label={copy.sourceLabel}>
      {(['sequence', 'all', 'selection'] as const).map(value => <Button key={value} disabled={busy||disabled} aria-pressed={source === value} onClick={() => { setSource(value); setChosen(new Set()); setPage(0); setQuery(''); }}>{copy.sourceNames[value]} · {sources[value].length}</Button>)}
    </div>
    <p className="micro">{copy.sourceHints[source]}</p>
    {tools}
    <div className={renderPreview?"project-photo-picker-body":""}><div>
    <div className="canvas-photo-picker-tools"><input type="search" aria-label={copy.search} placeholder={copy.search} value={query} disabled={busy||disabled} onChange={event => { setQuery(event.target.value); setPage(0); }} />
      <Button size="small" disabled={busy || disabled || selectable.length === 0 || selectable.length > maxSelected} onClick={() => setChosen(new Set(selectable.map(photo => photo.id)))}>{copy.selectAll}</Button>
      <Button size="small" disabled={busy || disabled || !ids.length} onClick={() => setChosen(new Set())}>{copy.clear}</Button></div>
    <div className="canvas-photo-picker-grid">
      {visible.map(photo => <label key={photo.id} className={chosen.has(photo.id) ? 'is-selected' : ''}>
        <img src={photo.src} alt="" loading="lazy" />
        <span><input type={maxSelected===1?"radio":"checkbox"} name={maxSelected===1?"project-photo-choice":undefined} checked={chosen.has(photo.id) && !placed.has(photo.id)} disabled={busy || disabled || placed.has(photo.id) || (maxSelected>1 && !chosen.has(photo.id) && chosen.size >= maxSelected)} onChange={() => setChosen(previous => { if(maxSelected===1)return new Set([photo.id]);const next = new Set(previous); if (!next.delete(photo.id)) next.add(photo.id); return next; })} />{photo.name}</span>{placed.has(photo.id) && <small>{copy.placed}</small>}
      </label>)}
    </div>
    {!available.length && <p>{copy.empty}</p>}
    </div>{renderPreview&&<aside className="project-photo-picker-preview">{renderPreview(sources[source].filter(photo=>ids.includes(photo.id)))}</aside>}</div>
    {error && <p role="alert">{error}</p>}
    <div className="canvas-photo-picker-footer"><span>{copy.selected.replace('{count}', String(ids.length)).replace('{limit}',String(maxSelected))}</span>
      {available.length > 60 && <div><Button size="small" disabled={busy || disabled || page === 0} onClick={() => setPage(value => value - 1)}>{copy.previous}</Button><span>{page + 1} / {Math.ceil(available.length / 60)}</span><Button size="small" disabled={busy || disabled || (page + 1) * 60 >= available.length} onClick={() => setPage(value => value + 1)}>{copy.next}</Button></div>}
      <Button disabled={busy} onClick={onClose}>{copy.cancel}</Button><Button variant="primary" disabled={busy || disabled || !canAdd || ids.length === 0 || ids.length > maxSelected} onClick={() => onAdd(ids)}>{busy ? copy.adding : copy.addCount.replace('{count}', String(ids.length))}</Button>
    </div>
  </Modal>;
}
