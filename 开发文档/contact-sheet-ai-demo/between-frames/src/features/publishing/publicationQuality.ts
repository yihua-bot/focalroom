import type { PublicationLayoutFrame } from '../../domain/publication';
/** 使用指定源/图像版本的像素记录，绝不使用屏幕缩略图尺寸计算清晰度。 */
export function publicationEffectivePpi(frame:PublicationLayoutFrame):number|undefined {
  const source=frame.source;if(!source)return;
  const width=source.renditionId?source.renditionWidth:source.width,height=source.renditionId?source.renditionHeight:source.height;
  if(!width||!height||width<=0||height<=0||frame.widthMm<=0||frame.heightMm<=0||frame.cropZoom<1)return;
  const axes=[frame.widthMm/width,frame.heightMm/height];
  const scale=(frame.fitMode==='fit'?Math.min(...axes):Math.max(...axes))*frame.cropZoom;
  return Number.isFinite(scale)&&scale>0?25.4/scale:undefined;
}
