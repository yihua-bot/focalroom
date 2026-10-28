import { useState,type DragEvent } from 'react';
import type { Photo } from '../../domain/models';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
import { projectPhotoSources,type ProjectPhotoSource } from '../photos/projectPhotoSources';
export function PublicationMaterials({photos,sequencePhotoIds,selectedPhotoIds,placedPhotoIds,disabled,onPlace,onDragStart,onDragEnd}:{photos:Photo[];sequencePhotoIds:string[];selectedPhotoIds:string[];placedPhotoIds:string[];disabled:boolean;onPlace:(photo:Photo)=>void;onDragStart:(photo:Photo,event:DragEvent<HTMLElement>)=>void;onDragEnd:()=>void}){
  const [source,setSource]=useState<ProjectPhotoSource>(sequencePhotoIds.length?'sequence':'all'),[query,setQuery]=useState(''),[page,setPage]=useState(0);
  const sources=projectPhotoSources(photos,sequencePhotoIds,selectedPhotoIds),matches=sources[source].filter(photo=>photo.name.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()));
  const pages=Math.max(1,Math.ceil(matches.length/24)),index=Math.min(page,pages-1),placed=new Set(placedPhotoIds);
  return <section className="publication-materials" aria-label={zh.publicationMaterials}>
    <p className="micro">{zh.publicationMaterialsHint}</p>
    <label>{zh.canvasPhotoSource}<select aria-label={zh.canvasPhotoSource} value={source} disabled={disabled} onChange={event=>{setSource(event.target.value as ProjectPhotoSource);setQuery('');setPage(0);}}>{(['sequence','all','selection'] as const).map(value=><option key={value} value={value}>{zh['canvasPhotoSource_'+value]} · {sources[value].length}</option>)}</select></label>
    <input type="search" aria-label={zh.canvasPhotoSearch} placeholder={zh.canvasPhotoSearch} value={query} disabled={disabled} onChange={event=>{setQuery(event.target.value);setPage(0);}}/>
    <div className="publication-material-grid">{matches.slice(index*24,index*24+24).map(photo=><button key={photo.id} type="button" className="publication-material" disabled={disabled||placed.has(photo.id)} draggable={!disabled&&!placed.has(photo.id)} aria-label={zh.publicationPlaceMaterial.replace('{name}',photo.name)} onDragStart={event=>onDragStart(photo,event)} onDragEnd={onDragEnd} onClick={()=>onPlace(photo)}><img src={photo.src} alt="" draggable={false} loading="lazy"/><span>{photo.name}</span><small>{placed.has(photo.id)?zh.publicationPhotoPlaced:zh.publicationOriginalPhoto}</small></button>)}</div>
    {!matches.length&&<p>{zh.canvasPhotoEmpty}</p>}
    {pages>1&&<div className="publication-material-pagination"><Button size="small" disabled={disabled||index===0} onClick={()=>setPage(index-1)}>{zh.canvasPhotoPrevious}</Button><span>{index+1} / {pages}</span><Button size="small" disabled={disabled||index>=pages-1} onClick={()=>setPage(index+1)}>{zh.canvasPhotoNext}</Button></div>}
  </section>;
}
