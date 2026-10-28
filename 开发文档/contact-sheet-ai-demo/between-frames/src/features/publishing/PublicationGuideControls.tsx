import { useEffect,useRef,useState,type RefObject } from 'react';
import { Ruler } from 'lucide-react';
import { zh } from '../../app/copy';
import { Button } from '../../components/shared/Button';
import { publicationDefaultGuideView,type PublicationGuideView } from './publicationGuides';

export function usePublicationGuideView(projectId:string,publicationId:string){
  const key='focalroom.publication-guide-view.v1:'+projectId+':'+publicationId;
  const [view,setView]=useState<PublicationGuideView>(()=>{try{const saved=JSON.parse(window.localStorage.getItem(key)??'null');if(saved&&Object.keys(publicationDefaultGuideView).every(flag=>typeof saved[flag]==='boolean'))return Object.fromEntries(Object.keys(publicationDefaultGuideView).map(flag=>[flag,saved[flag]])) as unknown as PublicationGuideView;}catch{/* 无法读取偏好时保持默认查看方式。 */}return {...publicationDefaultGuideView};});
  return [view,(next:PublicationGuideView)=>{setView(next);try{window.localStorage.setItem(key,JSON.stringify(next));}catch{/* 查看功能不依赖可写存储。 */}}] as const;
}
export function closePublicationGuideControls(element:HTMLDetailsElement|null,restoreFocus=true){if(!element?.open)return false;element.open=false;if(restoreFocus)element.querySelector('summary')?.focus();return true;}
export function PublicationGuideControls({view,onChange,canEdit,onEdit,controlRef}:{view:PublicationGuideView;onChange:(view:PublicationGuideView)=>void;canEdit:boolean;onEdit:()=>void;controlRef?:RefObject<HTMLDetailsElement|null>}){
  const local=useRef<HTMLDetailsElement>(null),details=controlRef??local;
  useEffect(()=>{const element=details.current;if(!element)return;const outside=(event:Event)=>{if(event.target instanceof Node&&!element.contains(event.target))closePublicationGuideControls(element,false);};const document=element.ownerDocument;document.addEventListener('pointerdown',outside,true);document.addEventListener('focusin',outside);return()=>{document.removeEventListener('pointerdown',outside,true);document.removeEventListener('focusin',outside);};},[details]);
  return <details className="publication-guide-controls" ref={details} onKeyDown={event=>{if(event.key==='Escape'&&closePublicationGuideControls(details.current)){event.preventDefault();event.stopPropagation();}}}>
    <summary><Ruler size={15}/>{zh.publicationAssist}</summary>
    <div className="publication-guide-options" role="group" aria-label={zh.publicationAssist}>
      {(['rulers','margins','grid','guides','snap'] as const).map(flag=><label key={flag}><input type="checkbox" checked={view[flag]} onChange={event=>onChange({...view,[flag]:event.target.checked})}/>{zh['publicationAssist_'+flag]}</label>)}
      <p className="micro">{zh.publicationAssistHint}</p><p className="micro">{zh.publicationSnapHint}</p>
      <Button size="small" disabled={!canEdit} onClick={()=>{closePublicationGuideControls(details.current);onEdit();}}>{zh.publicationEditGuides}</Button>
    </div>
  </details>;
}
