import { useMemo,useState } from 'react';
import type { SetPublicationCover } from '@betweenframes/contracts';
import type { PublicationDocument } from '../../domain/publication';
import { publicationPageCover } from '../../domain/publicationCover';
import { publicationLayers,publicationLayerViews } from '../../domain/publicationLayers';
import { publicationCoverPlan,PublicationCoverError } from './publicationCoverPlan';
import { PublicationPaper } from './PublicationPaper';
import { PublicationEditDialog } from './PublicationEditDialogs';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
const fields=['panelWidthMm','spineWidthMm','heightMm','marginMm','bleedMm'] as const;
export function PublicationCoverDialog({document,busy,pending,stale,notice,onClose,onSave}:{document:PublicationDocument;busy:boolean;pending:boolean;stale:boolean;notice:string;onClose:()=>void;onSave:(data:SetPublicationCover)=>Promise<boolean>}){
  const front=document.layout.pages.find(page=>page.pageNumber===0),back=document.layout.pages.find(page=>page.pageNumber===-1),cover=front?publicationPageCover(front):undefined;
  const [id]=useState(()=>front?.id??crypto.randomUUID());
  const [initial]=useState(()=>({panelWidthMm:String(cover?.panelWidthMm??front?.widthMm??210),spineWidthMm:String(cover?.spineWidthMm??0),heightMm:String(front?.heightMm??297),marginMm:String(front?.marginMm??15),bleedMm:String(front?.bleedMm??3)}));
  const [values,setValues]=useState(initial),[placement,setPlacement]=useState<SetPublicationCover['contentPlacement']>('followPanels'),[clear,setClear]=useState(false);
  const data=useMemo<SetPublicationCover>(()=>({pageId:id,backPageId:back?.id??null,settings:{panelWidthMm:Number(values.panelWidthMm.trim()||NaN),spineWidthMm:Number(values.spineWidthMm.trim()||NaN)},heightMm:Number(values.heightMm.trim()||NaN),marginMm:Number(values.marginMm.trim()||NaN),bleedMm:Number(values.bleedMm.trim()||NaN),contentPlacement:placement,clearPageSettingsConfirmed:clear}),[id,back?.id,values,placement,clear]);
  const result=useMemo(()=>{try{return {plan:publicationCoverPlan(document.layout,data),error:undefined};}catch(error){return {plan:undefined,error};}},[document,data]);
  const dirty=!cover||JSON.stringify(values)!==JSON.stringify(initial)||clear;
  const frozen=busy||pending||stale;
  const labels={panelWidthMm:zh.publicationCoverPanelWidth,spineWidthMm:zh.publicationCoverSpineWidth,heightMm:zh.publicationPageField_heightMm,marginMm:zh.publicationPageField_marginMm,bleedMm:zh.publicationPageField_bleedMm};
  const code=result.error instanceof PublicationCoverError?result.error.reason:'scope';
  const error=code==='locked'?zh.publicationCoverLocked:code==='settings'?zh.publicationCoverClearRequired:code==='crossing'?zh.publicationCoverCrossing:code==='bounds'?zh.publicationCoverBounds:code==='noop'?zh.publicationCoverNoop:zh.publicationCoverInvalid;
  const hasSettings=[front,back].some(page=>page?.layoutJson&&(!cover||Object.keys(JSON.parse(page.layoutJson)).some(key=>key!=='focalroomCoverV1')));
  return <PublicationEditDialog title={zh.publicationCoverSettings} dirty={dirty} busy={busy} pending={pending} notice={notice} onClose={onClose}>
    <p>{zh.publicationCoverHint}</p>
    <div className="publication-cover-editor">
      <form onSubmit={event=>{event.preventDefault();if(result.plan&&!frozen)void onSave(data).then(saved=>{if(saved)onClose();});}}>
        <fieldset className="publication-editor-fields"><legend>{zh.publicationPageSize}</legend>{fields.map(key=><label key={key}>{labels[key]}<input type="number" step="0.1" value={values[key]} disabled={frozen} onChange={event=>setValues(old=>({...old,[key]:event.target.value}))}/></label>)}</fieldset>
        <p className="micro">{zh.publicationCoverSpineHint}</p>
        {cover&&<label>{zh.publicationCoverPlacement}<select value={placement} disabled={frozen} onChange={event=>setPlacement(event.target.value as typeof placement)}><option value="followPanels">{zh.publicationCoverFollowPanels}</option><option value="keepPosition">{zh.publicationCoverKeepPosition}</option></select></label>}
        {hasSettings&&<label className="publication-editor-checkbox"><input type="checkbox" checked={clear} disabled={frozen} onChange={event=>setClear(event.target.checked)}/>{zh.publicationCoverClearSettings}</label>}
        {back&&<p>{zh.publicationCoverMergingHint}</p>}
        {!result.plan&&<p role="alert">{error}</p>}{stale&&<p role="alert">{zh.publicationRemovePagesStale}</p>}
        <div className="dialog-actions"><Button variant="primary" type="submit" disabled={frozen||!result.plan}>{zh.publicationSaveChanges}</Button><Button disabled={busy} onClick={onClose}>{dirty&&!pending?zh.publicationDiscardChanges:zh.publicationBack}</Button></div>
      </form>
      <section className="publication-cover-preview"><h4>{zh.publicationCoverAfter}</h4>{result.plan?<><PublicationPaper page={result.plan.page} previews={document.previews} layers={publicationLayerViews(publicationLayers(result.plan.preview))} label={zh.publicationPreview}/><p className="micro">{zh.publicationCoverSummary.replace('{width}',String(result.plan.page.widthMm)).replace('{height}',String(result.plan.page.heightMm))}</p><p>{zh.publicationCoverEditHint}</p></>:<p>{zh.publicationCoverInvalid}</p>}
      {front&&<details><summary>{zh.publicationCoverBefore}</summary><PublicationPaper page={front} previews={document.previews} layers={publicationLayerViews(publicationLayers(document.layout))} label={zh.publicationCurrentLayout}/></details>}</section>
    </div>
  </PublicationEditDialog>;
}
