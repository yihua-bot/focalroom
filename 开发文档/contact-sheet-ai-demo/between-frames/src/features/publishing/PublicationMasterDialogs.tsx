import { useEffect,useMemo,useState } from 'react';
import type { ApplyPublicationMaster,SavePublicationMaster,PublicationDocument,PublicationMaster,PublicationMasterPreview,PublicationRepository } from '../../domain/publication';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { zh } from '../../app/copy';
import { userFacingError } from '../../services/userFacingError';
import { PublicationPaper } from './PublicationPaper';
import { PublicationEditDialog } from './PublicationEditDialogs';
import { publicationPageName } from './publicationSpreads';
import { publicationLayers,publicationLayerViews } from '../../domain/publicationLayers';
import { publicationStoredPageGuides } from '../../domain/publicationGuides';
import { publicationDefaultGuideView } from './publicationGuides';
import { publicationOrdinaryFrames,publicationContinuousFrames } from '../../domain/publicationContinuous';
type Page=PublicationDocument['layout']['pages'][number];
export function PublicationSaveMaster({page,masters,existing,busy,pending,notice,onClose,onSave}:{page:Page;masters:PublicationMaster[];existing?:string;busy:boolean;pending:boolean;notice:string;onClose:()=>void;onSave:(data:SavePublicationMaster)=>Promise<boolean>}){
  const [newId]=useState(()=>crypto.randomUUID()),[choice,setChoice]=useState(existing??'');
  const old=masters.find(m=>m.id===choice);
  const [name,setName]=useState(()=>old?.name??''),[includeText,setIncludeText]=useState(false);
  const savedGuides=publicationStoredPageGuides(page);
  const [initialIncludeGuides]=useState(()=>!!savedGuides&&(!old||!!old.guides));
  const [includeGuides,setIncludeGuides]=useState(initialIncludeGuides);
  const dirty=name!==(masters.find(m=>m.id===existing)?.name??'')||includeText||choice!==(existing??'')||includeGuides!==initialIncludeGuides;
  const capacity=publicationOrdinaryFrames(page).length<=64&&(includeText?page.texts.length:page.texts.filter(t=>{try{return JSON.parse(t.contentJson).kind==='page_number';}catch{return false;}}).length)<=16&&(!!old||masters.length<16);
  return <PublicationEditDialog title={old?zh.publicationUpdateMaster:zh.publicationSaveMaster} dirty={dirty} busy={busy} pending={pending} notice={notice} onClose={onClose}>
    <p>{zh.publicationSaveMasterHint}</p>{publicationOrdinaryFrames(page).length!==page.frames.length&&<p>{zh.publicationContinuousMasterHint}</p>}
    <form onSubmit={event=>{event.preventDefault();if(!name.trim()||!capacity||busy||pending)return;void onSave({masterId:old?.id??newId,name:name.trim(),sourcePageId:page.id,expectedMasterRevision:old?.revision??null,includeText,includeGuides}).then(saved=>{if(saved)onClose();});}}>
      <label>{zh.publicationMasterDestination}<select disabled={busy||pending} value={choice} onChange={event=>{setChoice(event.target.value);setName(masters.find(m=>m.id===event.target.value)?.name??'');setIncludeGuides(!!savedGuides&&(!event.target.value||!!masters.find(m=>m.id===event.target.value)?.guides));}}><option value="">{zh.publicationNewMaster}</option>{masters.map(master=><option value={master.id} key={master.id}>{master.name}</option>)}</select></label>
      <label>{zh.publicationMasterName}<input maxLength={80} value={name} disabled={busy||pending} onChange={event=>setName(event.target.value)}/></label>
      <label className="publication-editor-checkbox"><input type="checkbox" checked={includeText} disabled={busy||pending} onChange={event=>setIncludeText(event.target.checked)}/>{zh.publicationMasterIncludeText}</label>
      <label className="publication-editor-checkbox"><input type="checkbox" checked={includeGuides} disabled={busy||pending||!savedGuides} onChange={event=>setIncludeGuides(event.target.checked)}/>{zh.publicationMasterIncludeGuides}</label>
      <p className="micro">{!savedGuides?zh.publicationMasterGuidesNotSaved:includeGuides?zh.publicationMasterGuidesIncluded:zh.publicationMasterGuidesPreserved}</p>
      <p className="micro">{zh.publicationMasterContentCount.replace('{frames}',String(publicationOrdinaryFrames(page).length)).replace('{texts}',String(page.texts.length))}</p>
      {!capacity&&<p role="alert">{zh.publicationMasterCapacity}</p>}
      <div className="dialog-actions"><Button variant="primary" type="submit" disabled={busy||pending||!capacity||!name.trim()}>{old?zh.publicationUpdateMaster:zh.publicationSaveMaster}</Button><Button disabled={busy} onClick={onClose}>{dirty&&!pending?zh.publicationDiscardChanges:zh.publicationBack}</Button></div>
    </form>
  </PublicationEditDialog>;
}
export function PublicationApplyMaster({projectId,publicationId,document,masters,activePageId,repository,busy,pending,notice,onClose,onApply}:{projectId:string;publicationId:string;document:PublicationDocument;masters:PublicationMaster[];activePageId:string;repository:PublicationRepository;busy:boolean;pending:boolean;notice:string;onClose:()=>void;onApply:(data:ApplyPublicationMaster)=>Promise<boolean>}){
  const [instanceId]=useState(()=>crypto.randomUUID()),[choice,setChoice]=useState(masters[0]?.id??'');
  const [selected,setSelected]=useState(()=>new Set([activePageId]));
  const [preview,setPreview]=useState<{key:string;value?:PublicationMasterPreview;error?:string}>();
  const [authorization,setAuthorization]=useState<{key:string;allowed:boolean}>();
  const [pageId,setPageId]=useState(activePageId);
  const master=masters.find(m=>m.id===choice);
  const key=JSON.stringify({masterId:choice,masterRevision:master?.revision??0,pageIds:document.layout.pages.filter(p=>selected.has(p.id)).map(p=>p.id),instanceId,overwriteLocal:false});
  const data=useMemo(()=>JSON.parse(key) as ApplyPublicationMaster,[key]);
  const scopeKey=String(document.revision)+':'+key;
  const allowed=authorization?.key===scopeKey&&authorization.allowed;
  useEffect(()=>{let active=true;if(!data.pageIds.length||!data.masterId)return;
    void repository.previewMaster!(projectId,publicationId,document.revision,data).then(value=>{if(active)setPreview({key:scopeKey,value});}).catch(error=>{if(active)setPreview({key:scopeKey,error:userFacingError(error,zh.publicationMasterPreviewFailed)});});return()=>{active=false;};
  },[repository,projectId,publicationId,document.revision,data,scopeKey]);
  const result=preview?.key===scopeKey?preview:undefined;
  const current=result?.value?.layout.pages.find(p=>p.id===pageId&&selected.has(p.id))??result?.value?.layout.pages.find(p=>selected.has(p.id));
  const override=Boolean(result?.value?.overriddenPageIds.length);
  const valid=!!result?.value&&data.pageIds.length>0&&(!override||allowed);
  const inner=document.layout.pages.filter(p=>p.pageNumber>0);
  return <Modal title={zh.publicationApplyMaster} className="publication-apply-master" onClose={()=>{if(!busy)onClose();}}>
    <p>{zh.publicationApplyMasterHint}</p>
    <p className="micro">{master?.guides?zh.publicationMasterGuidesReplace:zh.publicationMasterGuidesPreserved}</p>
    <div className="publication-master-apply-layout"><section>
      <label>{zh.publicationChooseMaster}<select disabled={busy||pending} value={choice} onChange={event=>setChoice(event.target.value)}>{masters.map(master=><option value={master.id} key={master.id}>{master.name}</option>)}</select></label>
      <div className="publication-master-selection-tools"><Button size="small" disabled={busy||pending||inner.length>100} onClick={()=>setSelected(new Set(inner.map(p=>p.id)))}>{zh.publicationSelectInnerPages}</Button><Button size="small" disabled={busy||pending} onClick={()=>setSelected(new Set([activePageId]))}>{zh.publicationSelectCurrentPage}</Button></div>
      <div className="publication-master-page-selection" role="group" aria-label={zh.publicationMasterTargetPages}>{document.layout.pages.map(page=><label className="publication-editor-checkbox" key={page.id}><input type="checkbox" disabled={busy||pending||(!selected.has(page.id)&&selected.size>=100)} checked={selected.has(page.id)} onChange={()=>setSelected(old=>{const next=new Set(old);if(!next.delete(page.id))next.add(page.id);return next;})}/>{publicationPageName(page.pageNumber)}</label>)}</div>
      {override&&<div className="publication-master-override"><p>{zh.publicationMasterLocalChanges.replace('{count}',String(result!.value!.overriddenPageIds.length))}</p><label className="publication-editor-checkbox"><input type="checkbox" disabled={busy||pending} checked={!!allowed} onChange={event=>setAuthorization({key:scopeKey,allowed:event.target.checked})}/>{zh.publicationMasterOverwrite}</label></div>}
      {result?.error&&<p role="alert">{result.error}</p>}
      {!data.pageIds.length&&<p>{zh.publicationMasterChoosePages}</p>}
    </section><aside className="publication-master-preview">
      {current&&result?.value?<><select aria-label={zh.publicationMasterPreviewPage} value={current.id} disabled={busy} onChange={event=>setPageId(event.target.value)}>{result.value.layout.pages.filter(p=>selected.has(p.id)).map(page=><option value={page.id} key={page.id}>{publicationPageName(page.pageNumber)}</option>)}</select><div><PublicationPaper assist={master?.guides||publicationStoredPageGuides(current)?{...publicationDefaultGuideView,rulers:true,grid:true,margins:true}:undefined} layers={publicationLayerViews(publicationLayers(result.value.layout))} continuous={publicationContinuousFrames(result.value.layout)} page={current} previews={result.value.previews} label={zh.publicationPreview}/></div></>:data.pageIds.length&&!result?.error?<p role="status">{zh.publicationMasterPreparing}</p>:null}
    </aside></div>
    {notice&&<p role="status">{notice}</p>}
    <div className="dialog-actions"><span>{zh.publicationMasterSelectedPages.replace('{count}',String(data.pageIds.length))}</span><Button disabled={busy} onClick={onClose}>{zh.publicationBack}</Button><Button variant="primary" disabled={busy||pending||!valid} onClick={()=>void onApply({...data,overwriteLocal:!!allowed}).then(saved=>{if(saved)onClose();})}>{zh.publicationApplyMaster}</Button></div>
  </Modal>;
}
