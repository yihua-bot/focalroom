import type { PublicationTextDirection } from '@betweenframes/contracts';
import { publicationTextContentStyle } from './publicationTextDirection';
import { useEffect,useRef,type CSSProperties,type MouseEvent } from 'react';
/** 检查真实字体布局，提示截断；不把浏览器测量当作字体嵌入或印厂预检。 */
export function PublicationTextBox({id,content,style,className,disabled,ariaLabel,onSelect,onOverflow,measure=true,selected,locked,direction,aspectRatio}:{id:string;content:string;style:CSSProperties;className:string;disabled?:boolean;ariaLabel?:string;onSelect?:(event:MouseEvent<HTMLButtonElement>)=>void;onOverflow:(id:string,value:boolean)=>void;measure?:boolean;selected?:boolean;locked?:boolean;direction?:PublicationTextDirection;aspectRatio?:number}){
  const outer=useRef<HTMLElement|null>(null),inner=useRef<HTMLSpanElement|null>(null);
  const signature=JSON.stringify([style,direction,aspectRatio]);
  useEffect(()=>{
    if(!measure)return;let active=true;const node=outer.current,body=inner.current;if(!node||!body)return;
    const read=()=>{if(active&&node.clientWidth>0&&node.clientHeight>0)onOverflow(id,body.scrollHeight>body.clientHeight+1||body.scrollWidth>body.clientWidth+1);};
    const frame=requestAnimationFrame(read);const observer=typeof ResizeObserver==='undefined'?undefined:new ResizeObserver(read);observer?.observe(node);observer?.observe(body);void document.fonts?.ready.then(read);
    return()=>{active=false;cancelAnimationFrame(frame);observer?.disconnect();};
  },[id,content,signature,onOverflow,measure]);
  const body=<span ref={inner} className="publication-text-content" data-text-direction={direction??'horizontal'} style={publicationTextContentStyle(direction,aspectRatio)}>{content}</span>;
  return onSelect?<button ref={node=>{outer.current=node;}} type="button" data-publication-kind="text" data-publication-id={id} data-publication-locked={locked||undefined} disabled={disabled} aria-label={ariaLabel} aria-pressed={selected} className={className} style={style} onClick={onSelect}>{body}</button>:<span ref={node=>{outer.current=node;}} data-publication-kind="text" data-publication-id={id} className={className} style={style}>{body}</span>;
}
