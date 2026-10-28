import type { ComponentProps } from 'react';
import { ProjectPhotoPicker } from '../photos/ProjectPhotoPicker';
import { zh } from '../../app/copy';
export function CanvasPhotoPicker(props:Omit<ComponentProps<typeof ProjectPhotoPicker>,'copy'|'maxSelected'|'renderPreview'|'tools'|'className'>){
  return <ProjectPhotoPicker {...props} copy={{title:zh.canvasAddPhotos,description:zh.canvasPhotoSourceHint,sourceLabel:zh.canvasPhotoSource,sourceNames:{sequence:zh.canvasPhotoSource_sequence,all:zh.canvasPhotoSource_all,selection:zh.canvasPhotoSource_selection},sourceHints:{sequence:zh.canvasPhotoHint_sequence,all:zh.canvasPhotoHint_all,selection:zh.canvasPhotoHint_selection},search:zh.canvasPhotoSearch,selectAll:zh.canvasPhotoSelectAll,clear:zh.canvasPhotoClear,placed:zh.canvasPhotoPlaced,empty:zh.canvasPhotoEmpty,selected:zh.canvasPhotoSelected,previous:zh.canvasPhotoPrevious,next:zh.canvasPhotoNext,cancel:zh.canvasPhotoCancel,adding:zh.canvasPhotoAdding,addCount:zh.canvasPhotoAddCount}}/>;
}
