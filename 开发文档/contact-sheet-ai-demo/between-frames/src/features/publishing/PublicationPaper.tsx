import type { CSSProperties } from 'react';
import type { PublicationLayout,PublicationLayoutFrame,PublicationPreview } from '../../domain/publication';
import { zh } from '../../app/copy';
type Page=PublicationLayout['pages'][number];
export function PublicationPaper({page,previews,selectedFrame,editing,disabled,onSelect,label}:{page:Page;previews:PublicationPreview[];selectedFrame?:string;editing?:PublicationLayoutFrame;disabled?:boolean;onSelect?:(id:string)=>void;label:string}){
  const sources=new Map(previews.map(p=>[p.frameId,p.thumbnailSrc]));
  const rectangle=(frame:{xMm:number;yMm:number;widthMm:number;heightMm:number}):CSSProperties=>({left:`${frame.xMm/page.widthMm*100}%`,top:`${frame.yMm/page.heightMm*100}%`,width:`${frame.widthMm/page.widthMm*100}%`,height:`${frame.heightMm/page.heightMm*100}%`});
  return <figure className="publication-studio-paper"><figcaption>{label} · {page.pageNumber===0?zh.publicationCover:zh.publicationPageNumber.replace('{number}',String(page.pageNumber))}</figcaption><div className="publication-paper" style={{aspectRatio:`${page.widthMm}/${page.heightMm}`}}>
    {page.frames.map((saved,index)=>{const frame=editing?.id===saved.id?editing:saved,src=sources.get(frame.id);return <button type="button" key={frame.id} className={`publication-frame ${selectedFrame===frame.id?'is-selected':''}`} disabled={!onSelect||disabled} aria-pressed={selectedFrame===frame.id} aria-label={zh.publicationSelectFrame.replace('{number}',String(index+1))} style={rectangle(frame)} onClick={()=>onSelect?.(frame.id)}>
      {src?<img src={src} alt="" style={{objectFit:frame.fitMode==='fit'?'contain':frame.fitMode==='fill'?'cover':'fill',objectPosition:`${frame.cropX*100}% ${frame.cropY*100}%`,transform:`scale(${frame.cropZoom})`}}/>:<span>{zh.publicationPreviewUnavailable}</span>}
    </button>;})}
    {page.texts.map(text=>{let content='';try{const value=JSON.parse(text.contentJson);content=typeof value.text==='string'?value.text:'';}catch{/* 非法文字记录由领域校验拒绝；这里不执行或渲染HTML。 */}return <div key={text.id} className="publication-text" style={rectangle(text)}>{content}</div>;})}
  </div></figure>;
}
