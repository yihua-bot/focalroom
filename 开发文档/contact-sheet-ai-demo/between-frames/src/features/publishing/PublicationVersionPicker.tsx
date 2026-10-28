import { useEffect,useRef,useState } from 'react';
import type { PublicationLayoutFrame,PublicationPhotoVersion,PublicationPhotoVersions,PublicationRepository } from '../../domain/publication';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { userFacingError } from '../../services/userFacingError';
import { zh } from '../../app/copy';
const key=(version:PublicationPhotoVersion)=>version.renditionId??'original';
const name=(version:PublicationPhotoVersion)=>version.renditionId===null?zh.publicationOriginalPhoto:version.variantName&&!/^(lightroom_classic|capture_one)-[a-f0-9]{64}$/.test(version.variantName)?version.variantName:version.externalSoftware==='lightroom_classic'?'Lightroom Classic':version.externalSoftware==='capture_one'?'Capture One':zh.publicationEditedPhoto;
type Props={repository:PublicationRepository;projectId:string;publicationId:string;revision:number;frame:PublicationLayoutFrame;busy:boolean;pending:boolean;notice:string;onClose:()=>void;onSave:(renditionId:string|null)=>Promise<boolean>};
export function PublicationVersionPicker(props:Props){return <PublicationVersionPickerBody key={`${props.projectId}:${props.publicationId}:${props.revision}:${props.frame.id}:${props.frame.renditionId??"original"}`} {...props}/>;}
function PublicationVersionPickerBody({repository,projectId,publicationId,revision,frame,busy,pending,notice,onClose,onSave}:Props){
  const [data,setData]=useState<PublicationPhotoVersions>(),[choices,setChoices]=useState<PublicationPhotoVersion[]>([]),[chosen,setChosen]=useState(frame.renditionId??'original');
  const [loaded,setLoaded]=useState<Set<string>>(()=>new Set()),[failed,setFailed]=useState<Set<string>>(()=>new Set());
  const [loading,setLoading]=useState(true),[error,setError]=useState('');const generation=useRef(0);
  useEffect(()=>{const attempt=++generation.current;void repository.photoVersions!(projectId,publicationId,revision,frame).then(value=>{if(attempt!==generation.current)return;setData(value);setChoices([value.original,value.current,...value.choices]);}).catch(reason=>{if(attempt===generation.current)setError(userFacingError(reason,zh.publicationPhotoVersionsFailed));}).finally(()=>{if(attempt===generation.current)setLoading(false);});return()=>{generation.current=attempt+1;};},[repository,projectId,publicationId,revision,frame]);
  const distinct=[...new Map(choices.map(value=>[key(value),value])).values()];
  const selected=distinct.find(item=>key(item)===chosen),current=data?.current;
  const disabled=busy||pending||loading;
  const mark=(setter:typeof setLoaded,id:string)=>setter(old=>old.has(id)?old:new Set(old).add(id));
  const display=(item:PublicationPhotoVersion,label:string)=><figure><figcaption>{label} · {name(item)}</figcaption>{item.thumbnailSrc&&!failed.has(key(item))?<img src={item.thumbnailSrc} alt={label} onLoad={()=>mark(setLoaded,key(item))} onError={()=>mark(setFailed,key(item))}/>:<p>{zh.publicationPreviewUnavailable}</p>}<span>{item.renditionId?item.source.renditionWidth:item.source.width} × {item.renditionId?item.source.renditionHeight:item.source.height} px</span></figure>;
  return <Modal title={zh.publicationPhotoVersions} className="publication-version-picker" onClose={()=>{if(!busy)onClose();}}>
    <p>{zh.publicationPhotoVersionsHint}</p>
    <div className="publication-version-picker-body"><section>
      <div className="publication-version-options" role="radiogroup" aria-label={zh.publicationPhotoVersions}>
        {distinct.map((item,index)=><label key={key(item)} className={key(item)===chosen?'is-selected':''}><input type="radio" name="publication-photo-version" aria-label={name(item)+' · '+(index+1)} value={key(item)} checked={key(item)===chosen} disabled={disabled||!item.thumbnailSrc||failed.has(key(item))} onChange={()=>setChosen(key(item))}/>{item.thumbnailSrc&&!failed.has(key(item))&&<img src={item.thumbnailSrc} alt="" onLoad={()=>mark(setLoaded,key(item))} onError={()=>mark(setFailed,key(item))}/>}<span><strong>{name(item)}</strong><small>{item.externalSoftware==='lightroom_classic'?'Lightroom Classic':item.externalSoftware==='capture_one'?'Capture One':item.renditionId?zh.publicationEditedPhoto:zh.publicationOriginalPhoto}{item.createdAt&&Number.isFinite(Date.parse(item.createdAt))?' · '+new Date(item.createdAt).toLocaleString():''}</small>{key(item)===(frame.renditionId??'original')&&<small>{zh.publicationVersionCurrentlyUsed}</small>}{(!item.thumbnailSrc||failed.has(key(item)))&&<small>{zh.publicationPreviewUnavailable}</small>}</span></label>)}
      </div>
      {data?.next&&<Button size="small" disabled={disabled} onClick={()=>{const attempt=generation.current;setLoading(true);setError('');void repository.photoVersions!(projectId,publicationId,revision,frame,data.next!).then(value=>{if(attempt===generation.current){setData(value);setChoices(old=>[...old,...value.choices]);}}).catch(reason=>{if(attempt===generation.current)setError(userFacingError(reason,zh.publicationPhotoVersionsFailed));}).finally(()=>{if(attempt===generation.current)setLoading(false);});}}>{zh.publicationMorePhotoVersions}</Button>}
      {loading&&<p role="status">{zh.publicationPhotoVersionsLoading}</p>}
      {error&&<p role="alert">{error}</p>}
      {data&&!data.choices.length&&!data.next&&distinct.length===1&&<p className="micro">{zh.publicationNoEditedPhotos}</p>}
    </section><aside className="publication-version-comparison">{current&&display(current,zh.publicationVersionCurrentlyUsed)}{selected&&display(selected,zh.publicationVersionPreview)}</aside></div>
    {notice&&<p role="status">{notice}</p>}
    <div className="dialog-actions"><Button disabled={busy} onClick={onClose}>{zh.publicationBack}</Button><Button variant="primary" disabled={disabled||!selected?.thumbnailSrc||!loaded.has(chosen)||failed.has(chosen)||selected.renditionId===frame.renditionId} onClick={()=>{if(selected)void onSave(selected.renditionId).then(saved=>{if(saved)onClose();});}}>{zh.publicationUsePhotoVersion}</Button></div>
  </Modal>;
}
