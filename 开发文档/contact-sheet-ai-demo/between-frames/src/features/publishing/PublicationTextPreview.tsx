import { useCallback,useEffect,useRef,useState } from 'react';
import type { PublicationLayout,PublicationPreview } from '../../domain/publication';
import type { PublicationLayerView } from '../../domain/publicationLayers';
import type { ContinuousFrame } from '../../domain/publicationContinuous';
import { PublicationPaper } from './PublicationPaper';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
/** 细节沿用实际Paper和字体测量，只调整视口，不写入文字或版面。 */
export function PublicationTextPreview({page,textId,previews,continuous,layers}:{page:PublicationLayout['pages'][number];textId:string;previews:PublicationPreview[];continuous:ContinuousFrame[];layers:PublicationLayerView[]}){
  const [detail,setDetail]=useState(true),[overflow,setOverflow]=useState(false),viewport=useRef<HTMLDivElement>(null);
  const selected=page.texts.find(text=>text.id===textId),signature=JSON.stringify(selected);
  const report=useCallback((id:string,value:boolean)=>{if(id===textId)setOverflow(value);},[textId]);
  useEffect(()=>{
    if(!detail)return;const node=viewport.current;if(!node)return;let frame:number|undefined,active=true;
    const focus=()=>{
      frame=undefined;if(!active||!node.clientWidth||!node.clientHeight)return;
      const object=Array.from(node.querySelectorAll<HTMLElement>('[data-publication-kind="text"]')).find(element=>element.dataset.publicationId===textId),body=object?.querySelector('.publication-text-content');
      if(!object||!body)return;const physical=object.getBoundingClientRect(),range=document.createRange();range.selectNodeContents(body);
      const glyph=typeof range.getBoundingClientRect==='function'?range.getBoundingClientRect():physical;
      let left=Math.max(physical.left,glyph.left),right=Math.min(physical.right,glyph.right),top=Math.max(physical.top,glyph.top),bottom=Math.min(physical.bottom,glyph.bottom);
      if(right<=left||bottom<=top){left=physical.left;right=physical.right;top=physical.top;bottom=physical.bottom;}
      const view=node.getBoundingClientRect(),w=node.clientWidth,h=node.clientHeight;
      const x=right-left>w?left+w/2:(left+right)/2,y=bottom-top>h?(body.getAttribute('data-text-direction')==='rotate-270'?bottom-h/2:top+h/2):(top+bottom)/2;
      node.scrollLeft+=x-view.left-node.clientLeft-w/2;node.scrollTop+=y-view.top-node.clientTop-h/2;
    };
    const schedule=()=>{if(active&&frame===undefined)frame=requestAnimationFrame(focus);};
    const resize=typeof ResizeObserver==='undefined'?undefined:new ResizeObserver(schedule);resize?.observe(node);
    const mutation=new MutationObserver(schedule);mutation.observe(node,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:['style']});
    void document.fonts?.ready.then(schedule);schedule();
    return()=>{active=false;if(frame!==undefined)cancelAnimationFrame(frame);resize?.disconnect();mutation.disconnect();};
  },[detail,signature,textId]);
  return <div className="publication-object-preview publication-text-preview">
    <div className="publication-text-preview-modes" role="group" aria-label={zh.publicationTextPreviewMode}><Button size="small" aria-pressed={detail} onClick={()=>setDetail(true)}>{zh.publicationTextDetailPreview}</Button><Button size="small" aria-pressed={!detail} onClick={()=>setDetail(false)}>{zh.publicationTextPagePreview}</Button></div>
    {detail&&overflow&&selected&&<p className="publication-text-preview-overflow" role="status">{zh.publicationTextOverflow.replace('{count}','1')}</p>}
    <div ref={viewport} className={'publication-text-preview-viewport'+(detail?' is-detail':' is-page')} aria-label={zh.publicationPreview} tabIndex={0}>
      <PublicationPaper page={page} previews={previews} continuous={continuous} layers={layers} selectedText={textId} label={zh.publicationPreview} zoom={detail?100:undefined} onTextOverflow={report}/>
    </div>
  </div>;
}
