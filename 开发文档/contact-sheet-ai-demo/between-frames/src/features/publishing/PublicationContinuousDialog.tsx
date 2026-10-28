import { useMemo,useState } from 'react';
import type { PublicationLayout,PublicationPreview,PublicationSpreadFrame,PublicationUnspreadFrame } from '../../domain/publication';
import { publicationLayers,publicationLayerViews } from '../../domain/publicationLayers';
import { publicationContinuousFrames,publicationContinuousPair,publicationLinkPreview,publicationUnlinkPreview } from '../../domain/publicationContinuous';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { zh } from '../../app/copy';
import { PublicationDimensionInput } from './PublicationDimensionInput';
import { PublicationPaper } from './PublicationPaper';
import { publicationPageName } from './publicationSpreads';
import { publicationFrameNumbers as numbers,publicationFrameValues,publicationParseFrame } from './publicationFrameForm';

export type ContinuousEditor={mode:'link'|'unlink';layout:PublicationLayout;previews:PublicationPreview[];frameId:string;leftId:string;rightId:string;revision:number};
export function PublicationContinuousDialog({editor,busy,pending,canWrite,notice,onClose,onLink,onUnlink}:{editor:ContinuousEditor;busy:boolean;pending:boolean;canWrite:boolean;notice:string;onClose:()=>void;onLink:(data:PublicationSpreadFrame)=>Promise<boolean>;onUnlink:(data:PublicationUnspreadFrame)=>Promise<boolean>}){
  const [left,right]=publicationContinuousPair(editor.layout.pages,editor.leftId,editor.rightId),source=editor.layout.pages.flatMap(page=>page.frames).find(frame=>frame.id===editor.frameId)!;
  const [target,setTarget]=useState(left.id),[confirmClose,setConfirmClose]=useState(false);
  const initial=useMemo(()=>editor.mode==='link'?{...source,xMm:left.marginMm,yMm:Math.max(left.marginMm,right.marginMm),widthMm:left.widthMm+right.widthMm-left.marginMm-right.marginMm,heightMm:left.heightMm-Math.max(left.marginMm,right.marginMm)*2}:{...source,xMm:left.marginMm,yMm:left.marginMm,widthMm:left.widthMm-left.marginMm*2,heightMm:left.heightMm-left.marginMm*2,fitMode:'fit' as const,cropX:.5,cropY:.5,cropZoom:1},[editor.mode,left,right,source]);
  const [values,setValues]=useState(()=>publicationFrameValues(initial));
  const geometry=publicationParseFrame(values),data=geometry?{frameId:source.id,leftPageId:left.id,rightPageId:right.id,geometry}:undefined,local=geometry?{frameId:source.id,targetPageId:target,geometry}:undefined;
  let preview:PublicationLayout|undefined;try{if(data&&local)preview=editor.mode==='link'?publicationLinkPreview(editor.layout,data):publicationUnlinkPreview(editor.layout,local);}catch{ /* 保留旧版面；无效预览不得提交。 */ }
  const continuous=preview?publicationContinuousFrames(preview):[],disabled=busy||pending||!canWrite;
  const close=()=>{if(busy)return;if(pending)onClose();else setConfirmClose(true);};
  return <Modal title={editor.mode==='link'?zh.publicationExpandContinuous:zh.publicationUnlinkContinuous} className="publication-continuous-dialog" onClose={close}>
    <p>{editor.mode==='link'?zh.publicationContinuousPairHint:zh.publicationContinuousReframeHint}</p>
    <form onSubmit={event=>{event.preventDefault();if(disabled||!preview||!data||!local)return;void(editor.mode==='link'?onLink(data):onUnlink(local)).then(saved=>{if(saved)onClose();});}}>
      <div className="publication-continuous-editor"><section>
        {editor.mode==='unlink'&&<label>{zh.publicationContinuousTarget}<select disabled={disabled} value={target} onChange={event=>setTarget(event.target.value)}>{[left,right].map(page=><option value={page.id} key={page.id}>{publicationPageName(page.pageNumber)}</option>)}</select></label>}
        <fieldset><legend>{zh.publicationPositionSize}</legend>{numbers.slice(0,4).map(key=><label key={key}>{zh['publicationField_'+key]}<PublicationDimensionInput value={values[key]} disabled={disabled} onChange={value=>setValues(old=>({...old,[key]:value}))}/></label>)}</fieldset>
        <label>{zh.publicationFit}<select disabled={disabled} value={values.fitMode} onChange={event=>setValues(old=>({...old,fitMode:event.target.value as typeof old.fitMode}))}>{(['fit','fill','stretch'] as const).map(value=><option value={value} key={value}>{zh['publicationFit_'+value]}</option>)}</select></label>
        <fieldset><legend>{zh.publicationCrop}</legend>{numbers.slice(4).map(key=><label key={key}>{zh['publicationField_'+key]}<input type="number" step="0.1" value={values[key]} disabled={disabled} onChange={event=>setValues(old=>({...old,[key]:event.target.value}))}/></label>)}</fieldset>
      </section><div className="publication-continuous-preview"><div className="publication-spread is-continuous">{preview?[left,right].map(page=><PublicationPaper layers={publicationLayerViews(publicationLayers(preview!))} key={page.id} page={preview!.pages.find(item=>item.id===page.id)!} previews={editor.previews} continuous={continuous} selectedFrame={source.id} label={zh.publicationPreview}/>):<p role="alert">{zh.publicationContinuousInvalid}</p>}</div></div></div>
      {notice&&<p role="status">{notice}</p>}
      <div className="dialog-actions"><Button variant="primary" type="submit" disabled={disabled||!preview}>{editor.mode==='link'?zh.publicationExpandContinuous:zh.publicationUnlinkContinuous}</Button><Button disabled={busy} onClick={onClose}>{zh.publicationBack}</Button></div>
    </form>
    {confirmClose&&<div role="group" aria-label={zh.publicationDiscardConfirm}><p>{zh.publicationDiscardConfirm}</p><Button disabled={busy} onClick={onClose}>{zh.publicationDiscardChanges}</Button><Button onClick={()=>setConfirmClose(false)}>{zh.publicationBack}</Button></div>}
  </Modal>;
}
