import { useCallback,useEffect,useRef,useState } from 'react';
import type { PreflightReport,PublicationDocument,PublicationFrameUpdate,PublicationInfo,PublicationRepository,PublicationVersion,PublicationVersionInfo } from '../../domain/publication';
import { preflightDetail,preflightLabel,preflightScope,preflightSeverity,preflightSummary } from './preflightDetails';
import { MAX_CAPTION_CHARS,addCaption,captionAligns,captionDraft,captionPositions,captionValues,editCaption,newCaption } from './captionDraft';
import type { CaptionValues } from './captionDraft';
import { MAX_WHOLE_BOOK_PAGES,editPage,findPreset,pageDraft,pageFields,pageGeometry,pageValues,paperPresets,presetLabel,wholeBookBlockers } from './pageDraft';
import type { PageValues } from './pageDraft';
import { nextStateActions,publicationActionLabel,publicationStateLabel } from './publicationWorkflow';
import { specDraft,specValues } from './exhibitionSpec';
import type { SpecValues } from './exhibitionSpec';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { userFacingError } from '../../services/userFacingError';
import { zh } from '../../app/copy';
import { PublicationPaper } from './PublicationPaper';
import { PublicationReviewPanel } from './PublicationReview';
import './publication-studio.css';

const numbers=['xMm','yMm','widthMm','heightMm','cropX','cropY','cropZoom'] as const;
// 纸张的宽高和图框的宽高同名，标签必须区分开，否则读屏里出现两个"宽度（mm）"。
const pageLabels:Record<(typeof pageFields)[number],string>={widthMm:'publicationField_pageWidthMm',heightMm:'publicationField_pageHeightMm',marginMm:'publicationField_marginMm',bleedMm:'publicationField_bleedMm'};
// 页面在屏幕上有两处要报名字（选择框与"哪几页放不下"），同一句文案只写一次。
const pageName=(page:{pageNumber:number})=>page.pageNumber===0?zh.publicationCover:zh.publicationPageNumber.replace('{number}',String(page.pageNumber));
type Values=Record<typeof numbers[number],string>&{fitMode:PublicationFrameUpdate['fitMode']};
const values=(frame:PublicationFrameUpdate):Values=>({...Object.fromEntries(numbers.map(key=>[key,String(frame[key])])) as Record<typeof numbers[number],string>,fitMode:frame.fitMode});
function title(version:PublicationVersionInfo){if(version.kind==='saved')return version.label;return ({'创建出版物':zh.publicationAutoCreate,'从作品序列生成':zh.publicationAutoGenerate,'调整版面':zh.publicationAutoEdit,'开始记录版本':zh.publicationAutoBaseline,'定展览规格':zh.publicationSpecVersion,'退回草稿':zh.publicationStateAction_draft,'送审':zh.publicationStateAction_in_review,'定稿':zh.publicationStateAction_finalized,'交付':zh.publicationStateAction_delivered} as Record<string,string>)[version.label]??(version.label.startsWith('恢复：')?zh.publicationAutoRestore:zh.publicationAutoEdit);}
type Props={repository:PublicationRepository;projectId:string;publication:PublicationInfo;readOnly:boolean;onClose:()=>void;onChanged:()=>Promise<void>};
export function PublicationStudio(props:Props){return <PublicationStudioBody key={`${props.projectId}:${props.publication.id}`} {...props}/>;}
function PublicationStudioBody({repository,projectId,publication,readOnly,onClose,onChanged}:Props){
  const [document,setDocument]=useState<PublicationDocument>();
  const [history,setHistory]=useState<PublicationVersionInfo[]>([]),[hasMore,setHasMore]=useState(false);
  const [selectedVersion,setSelectedVersion]=useState<PublicationVersion>();
  const [pageId,setPageId]=useState(''),[frameId,setFrameId]=useState('');
  const [edit,setEdit]=useState<Values>(),[label,setLabel]=useState('');
  const [textId,setTextId]=useState(''),[caption,setCaption]=useState<CaptionValues>(),[confirmCaption,setConfirmCaption]=useState(false);
  const [pageSetting,setPageSetting]=useState<PageValues>();
  const [specSetting,setSpecSetting]=useState<SpecValues>();
  const [compare,setCompare]=useState(false),[confirmRestore,setConfirmRestore]=useState(false),[confirmClose,setConfirmClose]=useState(false);
  const [busy,setBusy]=useState(false),[notice,setNotice]=useState('');
  const [preflight,setPreflight]=useState<PreflightReport>(),[preflightBusy,setPreflightBusy]=useState(false),[minPpi,setMinPpi]=useState(300);
  const [sheet,setSheet]=useState<{path:string;sha256:string;items:number}>(),[exportBusy,setExportBusy]=useState(false);
  const lock=useRef(false),alive=useRef(true),readGeneration=useRef(0),preflightToken=useRef(0),exportLock=useRef(false);
  const layout=selectedVersion?.layout??document?.layout;
  const page=layout?.pages.find(p=>p.id===pageId)??layout?.pages[0];
  const frame=page?.frames.find(f=>f.id===frameId);
  const baseline=frame?values({...frame,frameId:frame.id}):undefined;
  const savedText=page?.texts.find(t=>t.id===textId);
  const captionBase=savedText?captionValues(savedText):undefined;
  // 新建的图注还没有落到版面，只要面板开着就算未保存；已存在的则按值比对，回传原值不算改动。
  const captionDirty=Boolean(caption&&(captionBase?JSON.stringify(caption)!==JSON.stringify(captionBase):true));
  const pageBase=page?pageValues(page):undefined;
  const proposedPage=pageSetting?pageDraft(pageSetting):undefined;
  const pageDirty=Boolean(pageSetting&&pageBase&&JSON.stringify(pageSetting)!==JSON.stringify(pageBase));
  // 展览规格只有展览这一本才有：摄影书里这两个数没有对应的事实可判。
  const isExhibition=document?.layout.publication.pubType==='exhibition';
  const savedSpec=document?specValues(document.layout.publication.specJson):undefined;
  const proposedSpec=specSetting?specDraft(specSetting):undefined;
  const specDirty=Boolean(specSetting&&savedSpec&&JSON.stringify(specSetting)!==JSON.stringify(savedSpec));
  // 一次只允许一份未保存的草稿：纸张改动没落地时不能再开图注，图注改动没落地时纸张输入是锁着的。
  const openFrame=Boolean(edit),openCaption=Boolean(caption);
  const dirty=Boolean(edit&&baseline&&JSON.stringify(edit)!==JSON.stringify(baseline))||captionDirty||pageDirty||specDirty;
  const draft=caption?captionDraft(caption):captionBase?captionDraft(captionBase):undefined;
  const captionPreview=draft&&textId?{id:textId,...draft}:undefined;
  const pending=repository.pendingCommands?.(projectId).some(r=>r.publicationId===publication.id);
  // 审校状态以重新载入的版面为准：列表里的 publication 可能是改动前的旧值。
  const state=document?.layout.publication.state??publication.state;
  const forwardTo=nextStateActions(state);
  const writable=!readOnly&&!selectedVersion&&state==='draft'&&!pending;
  // 状态以刚读回来的版面为准，列表里那份 publication 可能是旧的：现在状态能在本窗口里改，旧副本会真的把恢复锁死或放行。
  const canRestore=!readOnly&&Boolean(selectedVersion)&&Boolean(document?.revision??publication.revision)&&state==='draft'&&!pending;
  // 整本统一开本的目标数就是当前页面上的数（含未保存草稿）；草稿无效时没有可套用的目标。
  const bookPages=document?.layout.pages??[];
  const applyTarget=pageSetting?proposedPage:(page?pageGeometry(page):undefined);
  // 服务端只会回一句笼统的"内容超出页面及出血范围"，不点名是哪一页，所以点名要在出门前做。
  const applyBlockers=applyTarget?wholeBookBlockers(bookPages,applyTarget):[];
  const applyDiffers=Boolean(applyTarget&&bookPages.some(item=>item.widthMm!==applyTarget.widthMm||item.heightMm!==applyTarget.heightMm||item.marginMm!==applyTarget.marginMm||item.bleedMm!==applyTarget.bleedMm));
  const applyTooMany=bookPages.length>MAX_WHOLE_BOOK_PAGES;
  const canApplyAll=Boolean(repository.editPages&&writable&&applyTarget&&applyDiffers&&!applyBlockers.length&&!applyTooMany&&bookPages.length>1&&!busy&&!openCaption&&!openFrame);
  const reload=useCallback(async()=>{
    const generation=++readGeneration.current;
    const [current,versions]=await Promise.allSettled([Promise.resolve().then(()=>repository.document!(projectId,publication.id)),Promise.resolve().then(()=>repository.versions!(projectId,publication.id))]);
    if(!alive.current||generation!==readGeneration.current)return;
    if(current.status==='fulfilled')setDocument(current.value);else{setDocument(undefined);setNotice(userFacingError(current.reason,zh.publicationLoadFailed));}
    if(versions.status==='fulfilled'){setHistory(versions.value);setHasMore(versions.value.length===30);}else setNotice(userFacingError(versions.reason,zh.publicationHistoryFailed));
    return current.status==='fulfilled'&&versions.status==='fulfilled';
  },[repository,projectId,publication.id]);
  useEffect(()=>{alive.current=true;readGeneration.current++;void Promise.resolve().then(reload);return()=>{alive.current=false;};
  },[reload]);
  const run=async(operation:()=>Promise<void>,success:string)=>{
    if(lock.current||readOnly)return;lock.current=true;setBusy(true);setNotice('');readGeneration.current++;
    try{await operation();if(!alive.current)return;setSelectedVersion(undefined);setConfirmRestore(false);setEdit(undefined);setLabel('');setCaption(undefined);setConfirmCaption(false);setPageSetting(undefined);setSpecSetting(undefined);setPreflight(undefined);setSheet(undefined);preflightToken.current++;const loaded=await reload();if(!alive.current)return;setNotice(loaded?success:zh.publicationSavedReloadFailed);await onChanged();}
    catch(error){if(alive.current)setNotice(userFacingError(error,zh.publicationSaveFailed));}
    finally{lock.current=false;if(alive.current)setBusy(false);}
  };
  const openVersion=async(version:PublicationVersionInfo)=>{
    if(lock.current||dirty)return;lock.current=true;setBusy(true);setNotice('');const generation=++readGeneration.current;
    try{const result=await repository.version!(projectId,publication.id,version.id);if(alive.current&&generation===readGeneration.current){setSelectedVersion(result);setFrameId('');setEdit(undefined);setTextId('');setCaption(undefined);setPageSetting(undefined);setSpecSetting(undefined);setConfirmCaption(false);setConfirmRestore(false);}}
    catch(error){if(alive.current&&generation===readGeneration.current)setNotice(userFacingError(error,zh.publicationHistoryFailed));}
    finally{lock.current=false;if(alive.current)setBusy(false);}
  };
  const check=async()=>{
    if(preflightBusy||lock.current||!repository.preflight)return;
    setPreflightBusy(true);setNotice('');const token=++preflightToken.current;
    try{const report=await repository.preflight(projectId,publication.id,minPpi);if(alive.current&&token===preflightToken.current)setPreflight(report);}
    catch(error){if(alive.current&&token===preflightToken.current)setPreflight(undefined);if(alive.current)setNotice(userFacingError(error,zh.publicationPreflightFailed));}
    finally{if(alive.current)setPreflightBusy(false);}
  };
  // 导出是一条只读命令，不改版面也不碰幂等队列，所以它绕开 run()：run() 在只读视图里直接返回，而"看这本并出货"正是最常见的组合。
  const exportSheet=async()=>{
    if(exportLock.current||!repository.exportWorklist||!document)return;
    exportLock.current=true;setExportBusy(true);setNotice('');setSheet(undefined);
    const items=document.layout.pages.reduce((total,page)=>total+page.frames.length+page.texts.length,0);
    try{const receipt=await repository.exportWorklist(projectId,publication.id);
      if(!alive.current)return;
      if(receipt)setSheet({path:receipt.path,sha256:receipt.sha256,items});else setNotice(zh.publicationExportCancelled);
    }
    catch(error){if(alive.current)setNotice(userFacingError(error,zh.publicationExportFailed));}
    finally{exportLock.current=false;if(alive.current)setExportBusy(false);}
  };
  const close=()=>{if(lock.current)return;if(dirty&&!pending)setConfirmClose(true);else onClose();};
  const proposed=(()=>{if(!frame||!edit)return undefined;const numeric=Object.fromEntries(numbers.map(key=>[key,edit[key].trim()===''?NaN:Number(edit[key])]));if(!Object.values(numeric).every(Number.isFinite)||numeric.widthMm<=0||numeric.heightMm<=0||numeric.cropZoom<1)return undefined;return {...frame,...numeric,fitMode:edit.fitMode};})();
  const currentPage=document?.layout.pages.find(p=>p.pageNumber===page?.pageNumber);
  return <Modal title={zh.publicationStudio} className="publication-studio" onClose={close}>
    <header className="publication-studio-header"><div><strong>{publication.title}</strong><span>{selectedVersion?`${zh.publicationVersion} ${selectedVersion.version.revision}`:`${zh.publicationCurrent} · ${document?.revision??publication.revision??'—'}`}</span></div>
      {selectedVersion&&<><Button size="small" disabled={busy} onClick={()=>{readGeneration.current++;setSelectedVersion(undefined);setConfirmRestore(false);setEdit(undefined);}}>{zh.publicationBackToCurrent}</Button><Button size="small" aria-pressed={compare} disabled={busy} onClick={()=>setCompare(value=>!value)}>{zh.publicationCompare}</Button></>}
      {pending&&<Button size="small" disabled={busy||readOnly} onClick={()=>void run(()=>repository.retryPending!(projectId,publication.id),zh.publicationConfirmed)}>{zh.publicationConfirmPrevious}</Button>}
    </header>
    <div className="publication-studio-layout"><aside className="publication-studio-history"><h3>{zh.publicationVersions}</h3>
      {!history.length&&<p>{zh.publicationNoVersions}</p>}
      {history.map(version=><button type="button" key={version.id} disabled={busy||dirty} aria-pressed={selectedVersion?.version.id===version.id} onClick={()=>void openVersion(version)}><strong>{title(version)}</strong><span>{zh.publicationVersion} {version.revision} · {new Date(version.createdAt).toLocaleString()}</span></button>)}
      {hasMore&&<Button size="small" disabled={busy||dirty} onClick={()=>{if(lock.current)return;lock.current=true;setBusy(true);void repository.versions!(projectId,publication.id,history[history.length-1].revision).then(rows=>{if(alive.current){setHistory(old=>[...old,...rows]);setHasMore(rows.length===30);}}).catch(error=>{if(alive.current)setNotice(userFacingError(error,zh.publicationHistoryFailed));}).finally(()=>{lock.current=false;if(alive.current)setBusy(false);});}}>{zh.publicationMoreVersions}</Button>}
    </aside><main className="publication-studio-preview">
      {!!layout?.pages.length&&<select aria-label={zh.publicationSelectPage} value={page?.id??''} disabled={busy||dirty} onChange={event=>{setPageId(event.target.value);setFrameId('');setEdit(undefined);setTextId('');setCaption(undefined);setPageSetting(undefined);setConfirmCaption(false);}}>{layout.pages.map(p=><option key={p.id} value={p.id}>{pageName(p)}</option>)}</select>}
      {!page&&<p>{zh.publicationNoPages}</p>}
      <div className={`publication-studio-papers ${compare&&selectedVersion?'is-comparing':''}`}>
        {compare&&selectedVersion&&currentPage&&<PublicationPaper page={currentPage} previews={document!.previews} label={zh.publicationCurrent}/>}
        {page&&<PublicationPaper page={page} previews={selectedVersion?.previews??document?.previews??[]} selectedFrame={frameId} selectedText={textId} editing={proposed} editingPage={selectedVersion?undefined:proposedPage} editingCaption={selectedVersion?undefined:captionPreview} disabled={busy||dirty} onSelect={id=>{setFrameId(id);setEdit(undefined);setTextId('');setCaption(undefined);setConfirmCaption(false);}} onSelectText={selectedVersion?undefined:id=>{setFrameId('');setEdit(undefined);const text=page.texts.find(t=>t.id===id);setTextId(id);setCaption(text?captionValues(text):undefined);setConfirmCaption(false);}} label={selectedVersion?title(selectedVersion.version):zh.publicationCurrent}/>}
      </div>
      {!readOnly&&!selectedVersion&&repository.changeState&&document&&<section className="publication-studio-state">
        <h3>{zh.publicationState}</h3>
        <p className="publication-studio-state-now">{publicationStateLabel(state)}</p>
        <p className="publication-studio-state-hint">{zh.publicationStateHint}</p>
        {!!forwardTo.length&&<div className="publication-studio-actions">{forwardTo.map(target=><Button key={target} size="small" disabled={busy||dirty||pending} onClick={()=>void run(()=>repository.changeState!(projectId,publication.id,document.revision,target),zh.publicationStateChanged)}>{publicationActionLabel(target)}</Button>)}</div>}
        {state!=='draft'&&<p className="publication-studio-state-locked">{zh.publicationStateLocked}</p>}
      </section>}
      {!selectedVersion&&repository.review&&document&&<PublicationReviewPanel repository={repository} projectId={projectId} publicationId={publication.id} readOnly={readOnly}/>}
      {!selectedVersion&&repository.editSpec&&isExhibition&&savedSpec&&<form className="publication-studio-spec" onSubmit={event=>{event.preventDefault();if(!writable||!document||!proposedSpec)return;void run(()=>repository.editSpec!(projectId,publication.id,document.revision,proposedSpec),zh.publicationSpecSaved);}}>
        <h3>{zh.publicationSpec}</h3>
        <p className="publication-studio-spec-hint">{zh.publicationSpecHint}</p>
        <label>{zh.publicationSpecDistance}<input type="number" min={300} max={20000} step={50} disabled={busy||!writable||openCaption||openFrame||pageDirty} value={(specSetting??savedSpec).viewingDistanceMm} onChange={event=>setSpecSetting(old=>({...old??savedSpec,viewingDistanceMm:event.target.value}))}/></label>
        <label>{zh.publicationSpecHang}<input type="number" min={300} max={3000} step={10} disabled={busy||!writable||openCaption||openFrame||pageDirty} value={(specSetting??savedSpec).hangCenterMm} onChange={event=>setSpecSetting(old=>({...old??savedSpec,hangCenterMm:event.target.value}))}/></label>
        <p className="publication-studio-spec-note">{zh.publicationSpecNote}</p>
        {specSetting&&!proposedSpec&&<p className="publication-studio-captions-problem">{zh.publicationSpecInvalid}</p>}
        <div className="publication-studio-actions"><Button type="submit" disabled={busy||!writable||!specDirty||!proposedSpec}>{zh.publicationSpecSave}</Button><Button disabled={busy||!specDirty} onClick={()=>setSpecSetting(undefined)}>{zh.publicationDiscardChanges}</Button></div>
      </form>}
      {!selectedVersion&&repository.preflight&&<section className="publication-studio-preflight">
        <h3>{zh.publicationPreflight}</h3>
        <div className="publication-studio-preflight-bar">
          <label>{zh.publicationPreflightMinPpi}<select aria-label={zh.publicationPreflightMinPpi} disabled={preflightBusy} value={minPpi} onChange={event=>{setMinPpi(Number(event.target.value));setPreflight(undefined);preflightToken.current++;}}>{[72,150,300].map(ppi=><option key={ppi} value={ppi}>{ppi} PPI</option>)}</select></label>
          <Button size="small" disabled={preflightBusy} onClick={()=>void check()}>{preflight?zh.publicationPreflightRerun:zh.publicationPreflightRun}</Button>
        </div>
        {preflight&&<p role="status" className={`publication-studio-preflight-summary ${preflight.blocking?'is-blocking':'is-clear'}`}>{preflightSummary(preflight)}</p>}
        {!!preflight?.issues.length&&<ul className="publication-studio-preflight-issues">{preflight.issues.map((issue,index)=><li key={`${issue.code}:${index}`}>
          <span className={`publication-studio-preflight-severity is-${issue.severity}`}>{preflightSeverity(issue.severity)}</span>
          <strong>{preflightScope(issue)}</strong>
          <span className="publication-studio-preflight-label">{preflightLabel(issue.code)}</span>
          {!!preflightDetail(issue)&&<em className="publication-studio-preflight-detail">{preflightDetail(issue)}</em>}
        </li>)}</ul>}
        {preflight?.truncated&&<p className="publication-studio-preflight-truncated">{zh.publicationPreflightTruncated}</p>}
      </section>}
      {!selectedVersion&&repository.exportWorklist&&document&&<section className="publication-studio-export">
        <h3>{zh.publicationExport}</h3>
        <p className="publication-studio-export-hint">{zh.publicationExportHint}</p>
        <Button size="small" disabled={exportBusy||busy} onClick={()=>void exportSheet()}>{zh.publicationExportRun}</Button>
        {!!sheet&&<div role="status" className="publication-studio-export-receipt">
          <p>{zh.publicationExportSaved.replace('{items}',String(sheet.items)).replace('{path}',sheet.path)}</p>
          <details><summary>{zh.publicationExportChecksum}</summary><small>SHA256 {sheet.sha256}</small></details>
        </div>}
      </section>}
    </main><aside className="publication-studio-inspector">
      <h3>{selectedVersion?zh.publicationSavedView:zh.publicationFrame}</h3>
      {selectedVersion&&<p>{zh.publicationSavedHint}</p>}
      {!frame&&!selectedVersion&&<p>{zh.publicationSelectFrameHint}</p>}
      {frame&&<form onSubmit={event=>{event.preventDefault();if(!writable||!proposed||!document)return;void run(()=>repository.editFrame!(projectId,publication.id,document.revision,{frameId:frame.id,xMm:proposed.xMm,yMm:proposed.yMm,widthMm:proposed.widthMm,heightMm:proposed.heightMm,fitMode:proposed.fitMode,cropX:proposed.cropX,cropY:proposed.cropY,cropZoom:proposed.cropZoom,assetId:frame.assetId,renditionId:frame.renditionId}),zh.publicationSaved);}}>
        {numbers.map(key=><label key={key}>{zh[`publicationField_${key}`]}<input type="number" step={key==='cropX'||key==='cropY'?0.01:0.1} disabled={busy||!writable} value={(edit??baseline!)[key]} onChange={event=>setEdit(old=>({...old??baseline!,[key]:event.target.value}))}/></label>)}
        <label>{zh.publicationFit}<select disabled={busy||!writable} value={(edit??baseline!).fitMode} onChange={event=>setEdit(old=>({...old??baseline!,fitMode:event.target.value as Values['fitMode']}))}>{(['fit','fill','stretch'] as const).map(mode=><option key={mode} value={mode}>{zh[`publicationFit_${mode}`]}</option>)}</select></label>
        <div className="publication-studio-actions"><Button type="submit" disabled={busy||!writable||!dirty||!proposed}>{zh.publicationSaveChanges}</Button><Button disabled={busy||!dirty} onClick={()=>setEdit(undefined)}>{zh.publicationDiscardChanges}</Button></div>
      </form>}
      {!selectedVersion&&repository.editPage&&page&&<form className="publication-studio-paper-settings" onSubmit={event=>{event.preventDefault();if(!writable||!document||!proposedPage)return;void run(()=>repository.editPage!(projectId,publication.id,document.revision,editPage(page.id,proposedPage)),zh.publicationPaperSaved);}}>
        <h4>{zh.publicationPaper}</h4>
        <label>{zh.publicationPaperPreset}<select disabled={busy||!writable||openCaption||openFrame||specDirty} value={findPreset(pageSetting??pageBase!)||'custom'} onChange={event=>{const preset=paperPresets.find(item=>item.id===event.target.value);if(!preset)return;setPageSetting(old=>({...old??pageBase!,widthMm:String(preset.widthMm),heightMm:String(preset.heightMm)}));}}>{paperPresets.map(preset=><option key={preset.id} value={preset.id}>{presetLabel(preset)}</option>)}<option value="custom" disabled>{zh.publicationPaperCustom}</option></select></label>
        {pageFields.map(key=><label key={key}>{zh[pageLabels[key]]}<input type="number" step={key==='bleedMm'?0.5:0.1} disabled={busy||!writable||openCaption||openFrame||specDirty} value={(pageSetting??pageBase!)[key]} onChange={event=>setPageSetting(old=>({...old??pageBase!,[key]:event.target.value}))}/></label>)}
        <p className="publication-studio-paper-note">{zh.publicationPaperHint}</p>
        {pageSetting&&!proposedPage&&<p className="publication-studio-captions-problem">{zh.publicationPaperInvalid}</p>}
        <div className="publication-studio-actions"><Button type="submit" disabled={busy||!writable||!pageDirty||!proposedPage||specDirty}>{zh.publicationPaperSave}</Button><Button disabled={busy||!pageDirty} onClick={()=>setPageSetting(undefined)}>{zh.publicationPaperDiscard}</Button></div>
        {bookPages.length>1&&repository.editPages&&<>
          <div className="publication-studio-actions"><Button disabled={!canApplyAll} onClick={()=>{if(!applyTarget||!document)return;void run(()=>repository.editPages!(projectId,publication.id,document.revision,bookPages.map(item=>editPage(item.id,applyTarget))),zh.publicationPaperApplyAllSaved);}}>{zh.publicationPaperApplyAll}</Button></div>
          {applyTooMany&&<p className="publication-studio-paper-note">{zh.publicationPaperApplyAllTooMany}</p>}
          {!!applyBlockers.length&&<p className="publication-studio-captions-problem">{zh.publicationPaperApplyAllBlocked.replace('{pages}',applyBlockers.map(pageName).join(zh.publicationPaperApplyAllJoin))}</p>}
        </>}
      </form>}
      {!selectedVersion&&repository.addText&&<section className="publication-studio-captions">
        <h4>{zh.publicationCaption}</h4>
        <Button size="small" disabled={busy||!writable||!page||pageDirty||specDirty} onClick={()=>{if(!page)return;setFrameId('');setEdit(undefined);setTextId(crypto.randomUUID());setCaption(newCaption(page));setConfirmCaption(false);}}>{zh.publicationCaptionAdd}</Button>
        {!page&&<p>{zh.publicationCaptionSelectPage}</p>}
        {page&&!page.texts.length&&!caption&&<p>{zh.publicationCaptionEmpty}</p>}
        {page&&!!page.texts.length&&!caption&&<p>{zh.publicationCaptionHint}</p>}
        {caption&&<form onSubmit={event=>{event.preventDefault();if(!writable||!document||!page||!textId||!draft)return;void run(()=>savedText?repository.editText!(projectId,publication.id,document.revision,editCaption(textId,draft)):repository.addText!(projectId,publication.id,document.revision,addCaption(textId,page.id,draft)),zh.publicationCaptionSaved);}}>
          <label>{zh.publicationCaptionText}<textarea rows={3} maxLength={MAX_CAPTION_CHARS} disabled={busy||!writable} value={caption.text} onChange={event=>setCaption({...caption,text:event.target.value})}/></label>
          {captionPositions.map(key=><label key={key}>{zh[`publicationField_${key}`]}<input type="number" step={0.1} disabled={busy||!writable} value={caption[key]} onChange={event=>setCaption({...caption,[key]:event.target.value})}/></label>)}
          <label>{zh.publicationCaptionSize}<input type="number" min={6} max={72} step={0.5} disabled={busy||!writable} value={caption.sizePt} onChange={event=>setCaption({...caption,sizePt:event.target.value})}/></label>
          <label>{zh.publicationCaptionAlign}<select disabled={busy||!writable} value={caption.align} onChange={event=>setCaption({...caption,align:event.target.value as CaptionValues['align']})}>{captionAligns.map(mode=><option key={mode} value={mode}>{zh[`publicationCaptionAlign_${mode}`]}</option>)}</select></label>
          {!draft&&<p className="publication-studio-captions-problem">{zh.publicationCaptionInvalid}</p>}
          <div className="publication-studio-actions"><Button type="submit" disabled={busy||!writable||!dirty||!draft}>{zh.publicationSaveChanges}</Button><Button disabled={busy||!dirty} onClick={()=>{if(savedText)setCaption(captionBase);else{setTextId('');setCaption(undefined);}}}>{zh.publicationDiscardChanges}</Button></div>
          {savedText&&!confirmCaption&&<Button size="small" disabled={busy||!writable} onClick={()=>setConfirmCaption(true)}>{zh.publicationCaptionDelete}</Button>}
          {confirmCaption&&savedText&&<div role="group" aria-label={zh.publicationCaptionDeleteConfirm}><p>{zh.publicationCaptionDeleteConfirm}</p><Button size="small" disabled={busy||!writable} onClick={()=>void run(async()=>{await repository.removeText!(projectId,publication.id,document?.revision??publication.revision!,textId);setTextId('');setCaption(undefined);setConfirmCaption(false);},zh.publicationCaptionDeleted)}>{zh.publicationCaptionDelete}</Button><Button size="small" disabled={busy} onClick={()=>setConfirmCaption(false)}>{zh.publicationBack}</Button></div>}
        </form>}
      </section>}
      {!selectedVersion&&<form onSubmit={event=>{event.preventDefault();if(!writable||!document||!label.trim()||dirty)return;void run(()=>repository.saveVersion!(projectId,publication.id,document.revision,label.trim()),zh.publicationVersionSaved);}}><h4>{zh.publicationSaveVersion}</h4><input aria-label={zh.publicationVersionName} placeholder={zh.publicationVersionName} maxLength={120} value={label} disabled={busy||!writable||dirty} onChange={event=>setLabel(event.target.value)}/><Button type="submit" size="small" disabled={busy||!writable||dirty||!label.trim()}>{zh.publicationSaveVersion}</Button></form>}
      {selectedVersion&&canRestore&&!confirmRestore&&<Button disabled={busy} onClick={()=>setConfirmRestore(true)}>{zh.publicationRestoreVersion}</Button>}
      {confirmRestore&&selectedVersion&&<div role="group" aria-label={zh.publicationRestoreConfirm}><p>{zh.publicationRestoreHint}</p><Button disabled={busy||!canRestore} onClick={()=>void run(()=>repository.restoreVersion!(projectId,publication.id,document?.revision??publication.revision!,selectedVersion.version.id),zh.publicationRestored)}>{zh.publicationRestoreConfirm}</Button><Button disabled={busy} onClick={()=>setConfirmRestore(false)}>{zh.publicationBack}</Button></div>}
    </aside></div>
    {notice&&<p role="status" className="publication-studio-notice">{notice}</p>}
    {confirmClose&&<div role="group" aria-label={zh.publicationDiscardConfirm}><p>{zh.publicationDiscardConfirm}</p><Button onClick={onClose}>{zh.publicationDiscardChanges}</Button><Button onClick={()=>setConfirmClose(false)}>{zh.publicationBack}</Button></div>}
  </Modal>;
}
