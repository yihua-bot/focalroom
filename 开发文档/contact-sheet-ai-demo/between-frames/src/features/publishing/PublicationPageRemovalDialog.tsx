import { useMemo,useState } from 'react';
import type { RemovePublicationPages } from '@betweenframes/contracts';
import type { PublicationDocument } from '../../domain/publication';
import { publicationContinuousFrames } from '../../domain/publicationContinuous';
import { publicationLayers,publicationLayerViews } from '../../domain/publicationLayers';
import { publicationChapters } from '../../domain/publicationChapters';
import { publicationPageRemoval,PublicationPageRemovalError } from './publicationPageRemoval';
import { publicationPageName } from './publicationSpreads';
import { PublicationPaper } from './PublicationPaper';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { zh } from '../../app/copy';
export interface PublicationPageRemovalScope{document:PublicationDocument;pageIds:string[];chapterId:string|null}
function reason(error:unknown){
  const code=error instanceof PublicationPageRemovalError?error.reason:'scope';
  return code==='locked'?zh.publicationRemovePagesLocked:code==='boundary'?zh.publicationRemovePagesBoundary:code==='chapter'?zh.publicationRemovePagesChapter:code==='parity'?zh.publicationRemovePagesParity:code==='noop'?zh.publicationRemovePagesNoop:zh.publicationRemovePagesInvalid;
}
export function PublicationPageRemovalDialog({scope,busy,pending,stale,notice,onClose,onConfirm}:{
  scope:PublicationPageRemovalScope;busy:boolean;pending:boolean;stale:boolean;notice:string;onClose:()=>void;onConfirm:(data:RemovePublicationPages,focusId?:string)=>Promise<boolean>;
}){
  const {document,pageIds,chapterId}=scope;
  const choices=useMemo(()=>[null,pageIds[0]].map(retainBlankPageId=>{
    const data={pageIds,chapterId,retainBlankPageId};
    try{return {data,plan:publicationPageRemoval(document.layout,data),error:undefined};}catch(error){return {data,plan:undefined,error};}
  }),[document,pageIds,chapterId]);
  const [retain,setRetain]=useState(()=>!choices[0].plan&&!!choices[1].plan),[previewId,setPreviewId]=useState(pageIds[0]);
  const selected=choices[retain?1:0],plan=selected.plan;
  const before=document.layout.pages.find(p=>p.id===previewId),after=plan?.preview.pages.find(p=>p.id===plan.focusId);
  const afterOriginal=after?document.layout.pages.find(p=>p.id===after.id):undefined;
  const chapter=chapterId?publicationChapters(document.layout).find(c=>c.id===chapterId):undefined;
  const targetPages=document.layout.pages.filter(p=>pageIds.includes(p.id));
  const frames=targetPages.reduce((n,p)=>n+p.frames.length,0),texts=targetPages.reduce((n,p)=>n+p.texts.length,0);
  const locked=busy||pending||stale;
  return <Modal title={chapterId?zh.publicationRemoveChapterPages:zh.publicationRemovePageGroup} className="publication-page-removal-dialog" description={zh.publicationRemovePagesHint} onClose={()=>{if(!busy)onClose();}}>
    {chapter&&<strong>{chapter.title}</strong>}
    <p>{zh.publicationRemovePagesSummary.replace('{pages}',String(pageIds.length)).replace('{photos}',String(frames)).replace('{texts}',String(texts))}</p>
    <div className="publication-page-removal-body"><section className="publication-page-removal-choices">
    <fieldset className="publication-remove-page-options"><legend>{zh.publicationRemovePagesChoice}</legend>
      {choices.map((choice,index)=><label key={index}><input type="radio" name="publication-page-removal-mode" checked={retain===(index===1)} disabled={locked||!choice.plan} onChange={()=>setRetain(index===1)}/><span><strong>{index===0?zh.publicationRemoveAllPages:zh.publicationRetainBlankPage}</strong><small>{index===0?zh.publicationRemoveAllPagesHint:zh.publicationRetainBlankPageHint}{!choice.plan&&' · '+reason(choice.error)}</small></span></label>)}
    </fieldset>
    <details className="publication-removal-page-list"><summary>{zh.publicationRemoveAffectedPages.replace('{count}',String(pageIds.length))}</summary><ul>{targetPages.map(p=><li key={p.id}>{publicationPageName(p.pageNumber)} · {zh.publicationRemovePageObjects.replace('{photos}',String(p.frames.length)).replace('{texts}',String(p.texts.length))}</li>)}</ul></details>
    <label className="publication-removal-preview-selector">{zh.publicationRemoveInspectPage}<select disabled={busy} value={previewId} onChange={event=>setPreviewId(event.target.value)}>{targetPages.map(p=><option key={p.id} value={p.id}>{publicationPageName(p.pageNumber)}</option>)}</select></label>
    </section><div className="publication-removal-preview">
      <section><h4>{zh.publicationBeforeRemoval}</h4>{before&&<PublicationPaper page={before} previews={document.previews} continuous={publicationContinuousFrames(document.layout)} layers={publicationLayerViews(publicationLayers(document.layout))} label={zh.publicationBeforeRemoval}/>}</section>
      <section><h4>{retain?zh.publicationRetainedBlankPreview:zh.publicationRemoveFollowingPreview}</h4>{after&&plan?<PublicationPaper page={after} previews={document.previews} continuous={publicationContinuousFrames(plan.preview)} layers={publicationLayerViews(publicationLayers(plan.preview))} label={zh.publicationAfterRemoval}/>:<p>{plan?zh.publicationNoPages:reason(selected.error)}</p>}</section>
    </div>
    </div>
    {after&&afterOriginal&&after.pageNumber!==afterOriginal.pageNumber&&<p className="micro publication-removal-page-identity">{zh.publicationRemovePageIdentity.replace('{before}',publicationPageName(afterOriginal.pageNumber)).replace('{after}',publicationPageName(after.pageNumber))}</p>}
    {plan&&<p className="publication-removal-result">{zh.publicationRemovePagesResult.replace('{removed}',String(plan.removedPageIds.length)).replace('{retained}',retain?'1':'0')}</p>}
    {stale&&<p role="alert">{zh.publicationRemovePagesStale}</p>}
    <div className="dialog-actions"><Button variant="primary" disabled={locked||!plan} onClick={()=>{if(plan)void onConfirm(plan.data,plan.focusId).then(saved=>{if(saved)onClose();});}}>{zh.publicationRemovePagesConfirm}</Button><Button disabled={busy} onClick={onClose}>{zh.publicationBack}</Button></div>
    {notice&&<p role="status">{notice}</p>}
  </Modal>;
}
