import { useState } from 'react';
import type { PublicationLayerNode,PublicationLayers } from '../../domain/publicationLayers';
import { publicationLayerViews,publicationLayerMembers } from '../../domain/publicationLayers';
import { Button } from '../../components/shared/Button';
import { Eye,EyeOff,LockKeyhole,LockKeyholeOpen,ChevronRight,ChevronDown } from 'lucide-react';
import { zh } from '../../app/copy';
export function PublicationLayersPanel({layers,sceneIds,selectedIds,activeId,disabled,dirty,canGroup,label,onSelect,onAction,onSave,onDiscard,onRemove}:{layers:PublicationLayers;sceneIds:string[];selectedIds:string[];activeId?:string;disabled:boolean;dirty:boolean;canGroup:boolean;label:(node:PublicationLayerNode)=>string;onSelect:(node:PublicationLayerNode,additive:boolean)=>void;onAction:(type:'visible'|'locked'|'front'|'back'|'forward'|'backward'|'ungroup'|'group'|'name',id:string,name?:string)=>void;onSave:()=>void;onDiscard:()=>void;onRemove?:(id:string)=>void}){
  const [collapsed,setCollapsed]=useState<Set<string>>(()=>new Set()),[page,setPage]=useState(0);
  const views=publicationLayerViews(layers),scene=new Set(sceneIds),byId=new Map(views.map(node=>[node.id,node])),rows:typeof views=[];
  const visit=(parent:string|null)=>{for(const node of [...views].reverse().filter(item=>item.parentId===parent)){if(!publicationLayerMembers(layers,node.id,views).some(member=>scene.has(member.id)))continue;rows.push(node);if(node.kind==='group'&&!collapsed.has(node.id))visit(node.id);}};visit(null);
  const pages=Math.max(1,Math.ceil(rows.length/40)),index=Math.min(page,pages-1),active=activeId?byId.get(activeId):undefined;
  const activeMembers=active?.kind==='group'?publicationLayerMembers(layers,active.id,views):[],groupLocked=!!active?.effectiveLocked||activeMembers.some(member=>byId.get(member.id)?.effectiveLocked);
  return <section className="publication-layers" aria-label={zh.publicationLayers}>
    <p className="micro">{zh.publicationLayerHint}</p>
    {canGroup&&<Button size="small" disabled={disabled} onClick={()=>onAction('group','')}>{zh.publicationGroup}</Button>}
    <div className="publication-layer-list">{rows.slice(index*40,index*40+40).map(node=><div key={node.id} data-publication-layer-id={node.id} className={'publication-layer-row'+(activeId===node.id||selectedIds.includes(node.id)?' is-selected':'')} style={{paddingInlineStart:node.depth*10}}>
      {node.kind==='group'&&<button type="button" className="publication-layer-icon" aria-label={(collapsed.has(node.id)?zh.publicationLayerExpand:zh.publicationLayerCollapse).replace('{name}',label(node))} onClick={()=>setCollapsed(old=>{const value=new Set(old);if(!value.delete(node.id))value.add(node.id);return value;})}>{collapsed.has(node.id)?<ChevronRight size={13}/>:<ChevronDown size={13}/>}</button>}
      <button type="button" className="publication-layer-select" disabled={disabled} aria-pressed={activeId===node.id||selectedIds.includes(node.id)} aria-label={zh.publicationLayerSelect.replace('{name}',label(node))} onClick={event=>onSelect(node,event.shiftKey||event.metaKey||event.ctrlKey)}><span>{label(node)}</span>{node.kind==='group'&&<small>{publicationLayerMembers(layers,node.id,views).length}</small>}</button>
      <button type="button" className="publication-layer-icon" disabled={disabled||node.visible&&!node.effectiveVisible} aria-pressed={!node.effectiveVisible} aria-description={node.visible&&!node.effectiveVisible?zh.publicationLayerInheritedHidden:undefined} aria-label={(node.visible?zh.publicationLayerHide:zh.publicationLayerShow).replace('{name}',label(node))} onClick={()=>onAction('visible',node.id)}>{node.effectiveVisible?<Eye size={14}/>:<EyeOff size={14}/>}</button>
      <button type="button" className="publication-layer-icon" disabled={disabled||node.effectiveLocked&&!node.locked} aria-description={node.effectiveLocked&&!node.locked?zh.publicationLayerInheritedLock:undefined} aria-pressed={node.effectiveLocked} aria-label={(node.locked?zh.publicationLayerUnlock:zh.publicationLayerLock).replace('{name}',label(node))} onClick={()=>onAction('locked',node.id)}>{node.effectiveLocked?<LockKeyhole size={13}/>:<LockKeyholeOpen size={13}/>}</button>
    </div>)}</div>
    {!rows.length&&<p>{zh.publicationLayerEmpty}</p>}
    {pages>1&&<div className="publication-page-pagination"><Button size="small" disabled={index===0} onClick={()=>setPage(index-1)}>{zh.canvasPhotoPrevious}</Button><span>{index+1} / {pages}</span><Button size="small" disabled={index>=pages-1} onClick={()=>setPage(index+1)}>{zh.canvasPhotoNext}</Button></div>}
    {active&&<div className="publication-layer-controls">
      <label>{zh.publicationLayerName}<input key={active.id+':'+(active.name??'')} disabled={disabled} maxLength={80} defaultValue={label(active)} onBlur={event=>{const name=event.target.value.trim();if(name&&name!==label(active))onAction('name',active.id,name);}} onKeyDown={event=>{if(event.key==='Enter'){event.preventDefault();event.currentTarget.blur();}}}/></label>
      <div>{(['front','forward','backward','back'] as const).map(action=><Button key={action} size="small" disabled={disabled||active.effectiveLocked} onClick={()=>onAction(action,active.id)}>{zh['publicationLayer_'+action]}</Button>)}</div>
      {active.kind==='group'&&<Button size="small" disabled={disabled} onClick={()=>onAction('ungroup',active.id)}>{zh.publicationUngroup}</Button>}
      {active.kind==='group'&&onRemove&&<><Button size="small" disabled={disabled||dirty||groupLocked||activeMembers.length>100} onClick={()=>onRemove(active.id)}>{zh.publicationRemoveGroup}</Button><p className="micro">{activeMembers.length>100?zh.publicationRemoveGroupLimit:groupLocked?zh.publicationRemoveGroupLocked:zh.publicationGroupActionHint}</p></>}
      {!active.effectiveVisible&&active.visible&&<p>{zh.publicationLayerInheritedHidden}</p>}
      {active.effectiveLocked&&!active.locked&&<p>{zh.publicationLayerInheritedLock}</p>}
    </div>}
    {dirty&&<div className="publication-layer-staged" role="group" aria-label={zh.publicationLayerPreview}><p>{zh.publicationLayerPreviewHint}</p><Button variant="primary" size="small" disabled={disabled} onClick={onSave}>{zh.publicationLayerSave}</Button><Button size="small" disabled={disabled} onClick={onDiscard}>{zh.publicationDiscardChanges}</Button></div>}
  </section>;
}
