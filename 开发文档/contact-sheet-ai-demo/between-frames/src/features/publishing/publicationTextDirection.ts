import type { CSSProperties } from 'react';
import type { PublicationTextDirection } from '@betweenframes/contracts';
/** 旋转内排字框而不是物理对象，缩放、选择和命中范围仍使用原来的毫米矩形。 */
export function publicationTextContentStyle(direction:PublicationTextDirection='horizontal',aspectRatio=1):CSSProperties{
  const base:CSSProperties={height:'100%',minHeight:0,boxSizing:'border-box',writingMode:direction==='vertical-rl'||direction==='vertical-lr'?direction:'horizontal-tb',textOrientation:'mixed'};
  if(direction!=='rotate-90'&&direction!=='rotate-270')return base;
  const ratio=Number.isFinite(aspectRatio)&&aspectRatio>0?aspectRatio:1;
  return {...base,position:'absolute',left:'50%',top:'50%',width:100/ratio+'%',height:100*ratio+'%',transform:'translate(-50%, -50%) rotate('+(direction==='rotate-90'?90:270)+'deg)',transformOrigin:'center'};
}
