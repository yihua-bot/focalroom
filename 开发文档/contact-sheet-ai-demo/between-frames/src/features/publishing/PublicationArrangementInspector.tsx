import { useState } from 'react';
import type { PublicationLayout } from '../../domain/publication';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
import { publicationArrange,publicationPageObjects,publicationObjectKey,publicationSelectionObjects,publicationSelectionBounds,type PublicationArrangeAction } from './publicationArrangement';
import type { PublicationObject } from './publicationGeometry';
type Page=PublicationLayout['pages'][number];
export function PublicationArrangementInspector({page,selected,busy,dirty,canWrite,changes,onToggle,onAll,onClear,onArrange,onSave,onDiscard}:{page:Page;selected:PublicationObject[];busy:boolean;dirty:boolean;canWrite:boolean;changes:number;onToggle:(object:PublicationObject)=>void;onAll:()=>void;onClear:()=>void;onArrange:(action:PublicationArrangeAction,reference:'selection'|'page')=>void;onSave:()=>void;onDiscard:()=>void}){
  const [reference,setReference]=useState<'selection'|'page'>('selection'),[listPage,setListPage]=useState(0);
  const objects=publicationPageObjects(page),boxes=publicationSelectionObjects(page,selected),bounds=boxes?publicationSelectionBounds(boxes):undefined;
  const keys=new Set(selected.map(publicationObjectKey)),pages=Math.max(1,Math.ceil(objects.length/20)),index=Math.min(listPage,pages-1);
  const actions=['left','centerX','right','top','centerY','bottom','distributeX','distributeY'] as const;
  const possible=new Map(actions.map(action=>[action,boxes?publicationArrange(page,boxes,action,reference):undefined]));
  const label=(object:PublicationObject)=>object.kind==='frame'?zh.publicationSelectFrame.replace('{number}',String(page.frames.findIndex(frame=>frame.id===object.id)+1)):zh.publicationSelectText.replace('{number}',String(page.texts.findIndex(text=>text.id===object.id)+1));
  return <section className="publication-arrangement" aria-label={zh.publicationArrangeTitle}>
    {selected.length<2&&<h4>{zh.publicationArrangeTitle}</h4>}<p className="micro">{zh.publicationArrangeHint}</p>
    {selected.length>1&&<><strong className="publication-arrange-count">{zh.publicationArrangeCount.replace('{count}',String(selected.length))}</strong>
      {bounds&&<p className="micro">{Number(bounds.widthMm.toFixed(3))} × {Number(bounds.heightMm.toFixed(3))} mm</p>}
      <label>{zh.publicationArrangeReference}<select disabled={busy||!canWrite} value={reference} onChange={event=>setReference(event.target.value as 'selection'|'page')}><option value="selection">{zh.publicationArrangeReferenceSelection}</option><option value="page">{zh.publicationArrangeReferencePage}</option></select></label>
      <div className="publication-arrange-actions">{actions.map(action=><Button key={action} size="small" disabled={busy||!canWrite||!possible.get(action)} title={!possible.get(action)?zh.publicationArrangeSpaceHint:undefined} onClick={()=>onArrange(action,reference)}>{zh['publicationArrange_'+action]}</Button>)}</div>
      {selected.length>=3&&!possible.get('distributeX')&&!possible.get('distributeY')&&<p className="micro">{zh.publicationArrangeSpaceHint}</p>}
      {!boxes&&<p role="alert">{zh.publicationArrangeInvalid}</p>}
      <p className="micro">{zh.publicationArrangeScaleHint}</p>
      <div className="publication-studio-actions"><Button variant="primary" disabled={busy||!canWrite||!changes||!boxes} onClick={onSave}>{zh.publicationSaveChanges}</Button><Button disabled={busy||!dirty} onClick={onDiscard}>{zh.publicationDiscardChanges}</Button></div>
    </>}
    <details open={selected.length>1?true:undefined}><summary>{zh.publicationArrangeObjectList} · {objects.length}</summary>
      <div className="publication-arrange-list-tools"><Button size="small" disabled={busy||dirty||!objects.length||objects.length>100} onClick={onAll}>{zh.publicationArrangeSelectAll}</Button><Button size="small" disabled={busy||dirty||!selected.length} onClick={onClear}>{zh.publicationArrangeClear}</Button></div>
      <div className="publication-arrange-object-list" role="group" aria-label={zh.publicationArrangeObjectList}>{objects.slice(index*20,index*20+20).map(object=><label className="publication-editor-checkbox" key={publicationObjectKey(object)}><input type="checkbox" checked={keys.has(publicationObjectKey(object))} disabled={busy||dirty||selected.length>=100&&!keys.has(publicationObjectKey(object))} onChange={()=>onToggle({kind:object.kind,id:object.id})}/><span>{label(object)}</span></label>)}</div>
      {pages>1&&<div className="publication-arrange-list-tools"><Button size="small" disabled={busy||index===0} onClick={()=>setListPage(index-1)}>{zh.canvasPhotoPrevious}</Button><span>{index+1} / {pages}</span><Button size="small" disabled={busy||index>=pages-1} onClick={()=>setListPage(index+1)}>{zh.canvasPhotoNext}</Button></div>}
      {objects.length>100&&<p className="micro">{zh.publicationArrangeLimit}</p>}
    </details>
  </section>;
}
