import type { CSSProperties } from 'react';
import type { PublicationLayout,PublicationLayoutFrame,PublicationPreview } from '../../domain/publication';
import { readCaption } from './captionDraft';
import type { CaptionAlign } from './captionDraft';
import type { PageDraft } from './pageDraft';
import { zh } from '../../app/copy';
type Page=PublicationLayout['pages'][number];
export type PaperCaption={id:string;xMm:number;yMm:number;widthMm:number;heightMm:number;text:string;align:CaptionAlign};
export function PublicationPaper({page,previews,selectedFrame,selectedText,editing,editingCaption,editingPage,disabled,onSelect,onSelectText,label}:{page:Page;previews:PublicationPreview[];selectedFrame?:string;selectedText?:string;editing?:PublicationLayoutFrame;editingCaption?:PaperCaption;editingPage?:PageDraft;disabled?:boolean;onSelect?:(id:string)=>void;onSelectText?:(id:string)=>void;label:string}){
  const sources=new Map(previews.map(p=>[p.frameId,p.thumbnailSrc]));
  // 纸张一改，页面比例和每个框的相对位置都按新的毫米数重画：改小到放不下是当场看得见的，不是等保存后被拒才知道。
  const width=editingPage?.widthMm??page.widthMm,height=editingPage?.heightMm??page.heightMm,margin=editingPage?.marginMm??page.marginMm;
  const rectangle=(box:{xMm:number;yMm:number;widthMm:number;heightMm:number}):CSSProperties=>({left:`${box.xMm/width*100}%`,top:`${box.yMm/height*100}%`,width:`${box.widthMm/width*100}%`,height:`${box.heightMm/height*100}%`});
  return <figure className="publication-studio-paper"><figcaption>{label} · {page.pageNumber===0?zh.publicationCover:zh.publicationPageNumber.replace('{number}',String(page.pageNumber))}</figcaption><div className="publication-paper" style={{aspectRatio:`${width}/${height}`}}>
    <div aria-hidden="true" className="publication-guide" style={rectangle({xMm:margin,yMm:margin,widthMm:width-margin*2,heightMm:height-margin*2})}/>
    {page.frames.map((saved,index)=>{const frame=editing?.id===saved.id?editing:saved,src=sources.get(frame.id);return <button type="button" key={frame.id} className={`publication-frame ${selectedFrame===frame.id?'is-selected':''}`} disabled={!onSelect||disabled} aria-pressed={selectedFrame===frame.id} aria-label={zh.publicationSelectFrame.replace('{number}',String(index+1))} style={rectangle(frame)} onClick={()=>onSelect?.(frame.id)}>
      {src?<img src={src} alt="" style={{objectFit:frame.fitMode==='fit'?'contain':frame.fitMode==='fill'?'cover':'fill',objectPosition:`${frame.cropX*100}% ${frame.cropY*100}%`,transform:`scale(${frame.cropZoom})`}}/>:<span>{zh.publicationPreviewUnavailable}</span>}
    </button>;})}
    {page.texts.map((saved,index)=>{const draft=editingCaption&&editingCaption.id===saved.id?editingCaption:undefined;const read=readCaption(saved);const shown={xMm:draft?.xMm??saved.xMm,yMm:draft?.yMm??saved.yMm,widthMm:draft?.widthMm??saved.widthMm,heightMm:draft?.heightMm??saved.heightMm,text:draft?.text??read.text,align:draft?.align??read.align};
      // 文字按纯文本渲染：版面里的内容 JSON 不是可信 HTML，不解析成标记。
      return <button type="button" key={saved.id} className={`publication-text ${selectedText===saved.id?'is-selected':''}`} disabled={!onSelectText||disabled} aria-pressed={selectedText===saved.id} aria-label={zh.publicationSelectCaption.replace('{number}',String(index+1))} style={{...rectangle(shown),textAlign:shown.align}} onClick={()=>onSelectText?.(saved.id)}>{shown.text}</button>;})}
    {/* 还没保存的新图注只作预览：版面里没有这条记录，点击不选中任何东西。 */}
    {editingCaption&&!page.texts.some(saved=>saved.id===editingCaption.id)&&<div className="publication-text is-preview" style={{...rectangle(editingCaption),textAlign:editingCaption.align}}>{editingCaption.text}</div>}
  </div></figure>;
}
