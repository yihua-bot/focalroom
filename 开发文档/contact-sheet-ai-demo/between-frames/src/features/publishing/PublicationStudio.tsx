import { useCallback,useEffect,useRef,useState } from 'react';
import type { PublicationDocument,PublicationFrameUpdate,PublicationInfo,PublicationRepository,PublicationVersion,PublicationVersionInfo } from '../../domain/publication';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { userFacingError } from '../../services/userFacingError';
import { zh } from '../../app/copy';
import { PublicationPaper } from './PublicationPaper';
import './publication-studio.css';

const numbers=['xMm','yMm','widthMm','heightMm','cropX','cropY','cropZoom'] as const;
type Values=Record<typeof numbers[number],string>&{fitMode:PublicationFrameUpdate['fitMode']};
const values=(frame:PublicationFrameUpdate):Values=>({...Object.fromEntries(numbers.map(key=>[key,String(frame[key])])) as Record<typeof numbers[number],string>,fitMode:frame.fitMode});
function title(version:PublicationVersionInfo){if(version.kind==='saved')return version.label;return ({'创建出版物':zh.publicationAutoCreate,'从作品序列生成':zh.publicationAutoGenerate,'调整版面':zh.publicationAutoEdit,'开始记录版本':zh.publicationAutoBaseline} as Record<string,string>)[version.label]??(version.label.startsWith('恢复：')?zh.publicationAutoRestore:zh.publicationAutoEdit);}
type Props={repository:PublicationRepository;projectId:string;publication:PublicationInfo;readOnly:boolean;onClose:()=>void;onChanged:()=>Promise<void>};
export function PublicationStudio(props:Props){return <PublicationStudioBody key={`${props.projectId}:${props.publication.id}`} {...props}/>;}
function PublicationStudioBody({repository,projectId,publication,readOnly,onClose,onChanged}:Props){
  const [document,setDocument]=useState<PublicationDocument>();
  const [history,setHistory]=useState<PublicationVersionInfo[]>([]),[hasMore,setHasMore]=useState(false);
  const [selectedVersion,setSelectedVersion]=useState<PublicationVersion>();
  const [pageId,setPageId]=useState(''),[frameId,setFrameId]=useState('');
  const [edit,setEdit]=useState<Values>(),[label,setLabel]=useState('');
  const [compare,setCompare]=useState(false),[confirmRestore,setConfirmRestore]=useState(false),[confirmClose,setConfirmClose]=useState(false);
  const [busy,setBusy]=useState(false),[notice,setNotice]=useState('');
  const lock=useRef(false),alive=useRef(true),readGeneration=useRef(0);
  const layout=selectedVersion?.layout??document?.layout;
  const page=layout?.pages.find(p=>p.id===pageId)??layout?.pages[0];
  const frame=page?.frames.find(f=>f.id===frameId);
  const baseline=frame?values({...frame,frameId:frame.id}):undefined;
  const dirty=Boolean(edit&&baseline&&JSON.stringify(edit)!==JSON.stringify(baseline));
  const pending=repository.pendingCommands?.(projectId).some(r=>r.publicationId===publication.id);
  const writable=!readOnly&&!selectedVersion&&document?.layout.publication.state==='draft'&&!pending;
  const canRestore=!readOnly&&Boolean(selectedVersion)&&Boolean(document?.revision??publication.revision)&&publication.state==='draft'&&!pending;
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
    try{await operation();if(!alive.current)return;setSelectedVersion(undefined);setConfirmRestore(false);setEdit(undefined);setLabel('');const loaded=await reload();if(!alive.current)return;setNotice(loaded?success:zh.publicationSavedReloadFailed);await onChanged();}
    catch(error){if(alive.current)setNotice(userFacingError(error,zh.publicationSaveFailed));}
    finally{lock.current=false;if(alive.current)setBusy(false);}
  };
  const openVersion=async(version:PublicationVersionInfo)=>{
    if(lock.current||dirty)return;lock.current=true;setBusy(true);setNotice('');const generation=++readGeneration.current;
    try{const result=await repository.version!(projectId,publication.id,version.id);if(alive.current&&generation===readGeneration.current){setSelectedVersion(result);setFrameId('');setEdit(undefined);setConfirmRestore(false);}}
    catch(error){if(alive.current&&generation===readGeneration.current)setNotice(userFacingError(error,zh.publicationHistoryFailed));}
    finally{lock.current=false;if(alive.current)setBusy(false);}
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
      {!!layout?.pages.length&&<select aria-label={zh.publicationSelectPage} value={page?.id??''} disabled={busy||dirty} onChange={event=>{setPageId(event.target.value);setFrameId('');setEdit(undefined);}}>{layout.pages.map(p=><option key={p.id} value={p.id}>{p.pageNumber===0?zh.publicationCover:zh.publicationPageNumber.replace('{number}',String(p.pageNumber))}</option>)}</select>}
      {!page&&<p>{zh.publicationNoPages}</p>}
      <div className={`publication-studio-papers ${compare&&selectedVersion?'is-comparing':''}`}>
        {compare&&selectedVersion&&currentPage&&<PublicationPaper page={currentPage} previews={document!.previews} label={zh.publicationCurrent}/>}
        {page&&<PublicationPaper page={page} previews={selectedVersion?.previews??document?.previews??[]} selectedFrame={frameId} editing={proposed} disabled={busy||dirty} onSelect={id=>{setFrameId(id);setEdit(undefined);}} label={selectedVersion?title(selectedVersion.version):zh.publicationCurrent}/>}
      </div>
    </main><aside className="publication-studio-inspector">
      <h3>{selectedVersion?zh.publicationSavedView:zh.publicationFrame}</h3>
      {selectedVersion&&<p>{zh.publicationSavedHint}</p>}
      {!frame&&!selectedVersion&&<p>{zh.publicationSelectFrameHint}</p>}
      {frame&&<form onSubmit={event=>{event.preventDefault();if(!writable||!proposed||!document)return;void run(()=>repository.editFrame!(projectId,publication.id,document.revision,{frameId:frame.id,xMm:proposed.xMm,yMm:proposed.yMm,widthMm:proposed.widthMm,heightMm:proposed.heightMm,fitMode:proposed.fitMode,cropX:proposed.cropX,cropY:proposed.cropY,cropZoom:proposed.cropZoom,assetId:frame.assetId,renditionId:frame.renditionId}),zh.publicationSaved);}}>
        {numbers.map(key=><label key={key}>{zh[`publicationField_${key}`]}<input type="number" step={key==='cropX'||key==='cropY'?0.01:0.1} disabled={busy||!writable} value={(edit??baseline!)[key]} onChange={event=>setEdit(old=>({...old??baseline!,[key]:event.target.value}))}/></label>)}
        <label>{zh.publicationFit}<select disabled={busy||!writable} value={(edit??baseline!).fitMode} onChange={event=>setEdit(old=>({...old??baseline!,fitMode:event.target.value as Values['fitMode']}))}>{(['fit','fill','stretch'] as const).map(mode=><option key={mode} value={mode}>{zh[`publicationFit_${mode}`]}</option>)}</select></label>
        <div className="publication-studio-actions"><Button type="submit" disabled={busy||!writable||!dirty||!proposed}>{zh.publicationSaveChanges}</Button><Button disabled={busy||!dirty} onClick={()=>setEdit(undefined)}>{zh.publicationDiscardChanges}</Button></div>
      </form>}
      {!selectedVersion&&<form onSubmit={event=>{event.preventDefault();if(!writable||!document||!label.trim()||dirty)return;void run(()=>repository.saveVersion!(projectId,publication.id,document.revision,label.trim()),zh.publicationVersionSaved);}}><h4>{zh.publicationSaveVersion}</h4><input aria-label={zh.publicationVersionName} placeholder={zh.publicationVersionName} maxLength={120} value={label} disabled={busy||!writable||dirty} onChange={event=>setLabel(event.target.value)}/><Button type="submit" size="small" disabled={busy||!writable||dirty||!label.trim()}>{zh.publicationSaveVersion}</Button></form>}
      {selectedVersion&&canRestore&&!confirmRestore&&<Button disabled={busy} onClick={()=>setConfirmRestore(true)}>{zh.publicationRestoreVersion}</Button>}
      {confirmRestore&&selectedVersion&&<div role="group" aria-label={zh.publicationRestoreConfirm}><p>{zh.publicationRestoreHint}</p><Button disabled={busy||!canRestore} onClick={()=>void run(()=>repository.restoreVersion!(projectId,publication.id,document?.revision??publication.revision!,selectedVersion.version.id),zh.publicationRestored)}>{zh.publicationRestoreConfirm}</Button><Button disabled={busy} onClick={()=>setConfirmRestore(false)}>{zh.publicationBack}</Button></div>}
    </aside></div>
    {notice&&<p role="status" className="publication-studio-notice">{notice}</p>}
    {confirmClose&&<div role="group" aria-label={zh.publicationDiscardConfirm}><p>{zh.publicationDiscardConfirm}</p><Button onClick={onClose}>{zh.publicationDiscardChanges}</Button><Button onClick={()=>setConfirmClose(false)}>{zh.publicationBack}</Button></div>}
  </Modal>;
}
