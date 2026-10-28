import { publicationStoredTextDirection } from '../../domain/publicationTypography';
import type { PublicationLayout,PublicationTextUpdate } from '../../domain/publication';
type Page=PublicationLayout['pages'][number];
type Text=Page['texts'][number];
function object(raw:string|null|undefined):Record<string,unknown>{try{const value=JSON.parse(raw??'{}');return value&&typeof value==='object'&&!Array.isArray(value)?value:{};}catch{return {};}}
export function publicationTextValues(page:Page,text:Text|undefined,id:string,kind:'text'|'page_number'):PublicationTextUpdate {
  const content=object(text?.contentJson),style=object(text?.styleJson);
  const direction=publicationStoredTextDirection(text?.styleJson);
  return {...(direction===undefined?{}:{textDirection:direction}),textId:text?.id??id,pageId:page.id,xMm:text?.xMm??page.marginMm,yMm:text?.yMm??(kind==='page_number'?page.heightMm-page.marginMm-10:page.marginMm),widthMm:text?.widthMm??page.widthMm-page.marginMm*2,heightMm:text?.heightMm??(kind==='page_number'?10:Math.min(35,page.heightMm-page.marginMm*2)),kind:content.kind==='page_number'?'page_number':text?'text':kind,text:typeof content.text==='string'?content.text:'',fontFamily:['serif','sans-serif','monospace'].includes(String(style.fontFamily))?style.fontFamily as PublicationTextUpdate['fontFamily']:'serif',fontSizePt:typeof style.fontSizePt==='number'?style.fontSizePt:kind==='page_number'?10:18,lineHeight:typeof style.lineHeight==='number'?style.lineHeight:1.4,align:['left','center','right'].includes(String(style.align))?style.align as PublicationTextUpdate['align']:'left',bold:style.bold===true,color:typeof style.color==='string'&&/^#[\da-fA-F]{6}$/.test(style.color)?style.color:'#222222'};
}

export function publicationTextStyle(value:PublicationTextUpdate){return {fontFamily:value.fontFamily,fontSizePt:value.fontSizePt,lineHeight:value.lineHeight,align:value.align,bold:value.bold,color:value.color,...(value.textDirection===undefined?{}:{textDirection:value.textDirection})};}
