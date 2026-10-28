import type { PublicationLayerView } from '../../domain/publicationLayers';
import { useState } from 'react';
import type { Photo } from '../../domain/models';
import type { PublicationLayout,PublicationPreview } from '../../domain/publication';
import { ProjectPhotoPicker } from '../photos/ProjectPhotoPicker';
import { PublicationPaper } from './PublicationPaper';
import { publicationPhotoLayout } from './publicationPhotoLayout';
import { zh } from '../../app/copy';
import { publicationVisibleFrames,type ContinuousFrame } from '../../domain/publicationContinuous';
type Page=PublicationLayout['pages'][number];
export function PublicationPhotoPicker({page,previews,photos,sequencePhotoIds,selectedPhotoIds,replacement,busy,pending,error,onClose,onSave,continuous=[],layers=[]}:{page:Page;previews:PublicationPreview[];photos:Photo[];sequencePhotoIds:string[];selectedPhotoIds:string[];replacement?:string;busy:boolean;pending:boolean;error:string;onClose:()=>void;onSave:(ids:string[],columns:number)=>Promise<boolean>;continuous?:ContinuousFrame[];layers?:PublicationLayerView[]}){
  const [columns,setColumns]=useState(0);
  const protectedIds=new Set(layers.filter(node=>node.effectiveLocked||!node.effectiveVisible||node.parentId).map(node=>node.id));
  const emptyCount=page.frames.filter(f=>!f.assetId&&!protectedIds.has(f.id)).length;
  const filling=!!replacement&&!page.frames.find(f=>f.id===replacement)?.assetId;
  const paired=continuous.find(item=>item.frame.id===replacement),visible=paired?[...publicationVisibleFrames(paired.left,continuous),...publicationVisibleFrames(paired.right,continuous)]:publicationVisibleFrames(page,continuous);
  const placed=previews.filter(preview=>visible.some(frame=>frame.id===preview.frameId&&frame.id!==replacement)).flatMap(preview=>preview.photoId?[preview.photoId]:[]);
  return <ProjectPhotoPicker className="publication-photo-picker" photos={photos} sequencePhotoIds={sequencePhotoIds} selectedPhotoIds={selectedPhotoIds} placedPhotoIds={placed} busy={busy} disabled={pending} canAdd={!pending} error={error} maxSelected={replacement?1:Math.min(16,emptyCount||100-page.frames.length)} onClose={onClose}
    onAdd={ids=>void onSave(ids,columns).then(saved=>{if(saved)onClose();})}
    copy={{title:filling?zh.publicationFillFrame:replacement?zh.publicationReplacePhoto:zh.publicationAddPhotos,description:filling||(!replacement&&emptyCount)?zh.publicationFillFramesHint:replacement?zh.publicationReplacePhotoHint:zh.publicationPhotoArrangeHint,sourceLabel:zh.canvasPhotoSource,sourceNames:{sequence:zh.canvasPhotoSource_sequence,all:zh.canvasPhotoSource_all,selection:zh.canvasPhotoSource_selection},sourceHints:{sequence:zh.publicationPhotoHint_sequence,all:zh.publicationPhotoHint_all,selection:zh.publicationPhotoHint_selection},search:zh.canvasPhotoSearch,selectAll:zh.canvasPhotoSelectAll,clear:zh.canvasPhotoClear,placed:zh.publicationPhotoPlaced,empty:zh.canvasPhotoEmpty,selected:zh.publicationPhotoSelected,previous:zh.canvasPhotoPrevious,next:zh.canvasPhotoNext,cancel:zh.canvasPhotoCancel,adding:zh.publicationSaving,addCount:filling?zh.publicationFillFrame:replacement?zh.publicationReplacePhoto:zh.publicationPhotoPlaceCount}}
    tools={!replacement&&!emptyCount&&<label className="publication-photo-layout-option">{zh.publicationPhotoColumns}<select value={columns} disabled={busy||pending} onChange={event=>setColumns(Number(event.target.value))}>{[0,1,2,3,4].map(n=><option key={n} value={n}>{n===0?zh.publicationPhotoColumnsAuto:zh.publicationPhotoColumnCount.replace('{count}',String(n))}</option>)}</select></label>}
    renderPreview={chosen=>{
      let preview=page,images=previews;
      if(replacement){if(chosen[0]){images=[...previews.filter(p=>p.frameId!==replacement),{frameId:replacement,thumbnailSrc:chosen[0].src}];preview={...page,frames:page.frames.map(frame=>frame.id===replacement?{...frame,cropX:.5,cropY:.5,cropZoom:1}:frame)};}}
      else if(chosen.length){let index=0;const layout=publicationPhotoLayout(page,chosen,columns,()=>`preview-${index++}`,protectedIds);preview=layout.preview;images=[...previews,...layout.previewImages.map(frame=>({frameId:frame.frameId,thumbnailSrc:chosen[frame.photoIndex].src}))];}
      return <PublicationPaper layers={layers} page={preview} previews={images} selectedFrame={replacement} label={zh.publicationPreview} continuous={continuous.map(item=>replacement===item.frame.id&&chosen[0]?{...item,frame:{...item.frame,cropX:.5,cropY:.5,cropZoom:1}}:item)}/>;
    }}/>
}
