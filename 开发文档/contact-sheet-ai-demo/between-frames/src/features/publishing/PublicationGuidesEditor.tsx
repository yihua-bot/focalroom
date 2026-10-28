import { useState } from 'react';
import { zh } from '../../app/copy';
import type { PublicationLayout,PublicationPreview } from '../../domain/publication';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { PublicationDimensionInput } from './PublicationDimensionInput';
import { PublicationPaper } from './PublicationPaper';
import { publicationLayers,publicationLayerViews } from '../../domain/publicationLayers';
import { publicationContinuousFrames } from '../../domain/publicationContinuous';
import { publicationPageGuides,publicationGuidesPreview,publicationDefaultGuideView,type PublicationPageGuides } from './publicationGuides';
import { PublicationGuidesError } from '../../domain/publicationGuides';
type Page=PublicationLayout['pages'][number];
const fields=['columns','rows','columnGapMm','rowGapMm'] as const;
export function PublicationGuidesEditor({page,layout,previews,busy,pending,notice,onClose,onSave}:{page:Page;layout:PublicationLayout;previews:PublicationPreview[];busy:boolean;pending:boolean;notice:string;onClose:()=>void;onSave:(settings:PublicationPageGuides)=>Promise<boolean>}){
  const [initial]=useState(()=>publicationPageGuides(page));
  const [values,setValues]=useState(()=>Object.fromEntries(fields.map(field=>[field,String(initial[field])])) as Record<typeof fields[number],string>);
  const [vertical,setVertical]=useState(()=>initial.verticalGuidesMm.map(String)),[horizontal,setHorizontal]=useState(()=>initial.horizontalGuidesMm.map(String));
  const number=(value:string)=>value.trim()?Number(value):NaN;
  const settings={...Object.fromEntries(fields.map(field=>[field,number(values[field])])),verticalGuidesMm:vertical.map(number),horizontalGuidesMm:horizontal.map(number)} as PublicationPageGuides;
  let layoutJson:string|undefined,validation=zh.publicationGuidesInvalid;try{layoutJson=publicationGuidesPreview(page,settings);}catch(error){if(error instanceof PublicationGuidesError&&error.reason==='capacity')validation=zh.publicationGuidesCapacity;}
  const nextLine=(values:string[],length:number)=>{const candidates=[length/2,...Array.from({length:33},(_,i)=>Math.round(length*(i+1)/34*100)/100)];return String(candidates.find(value=>!values.map(number).includes(value))!);};
  return <Modal title={zh.publicationEditGuides} description={zh.publicationGuidesEditHint} className="publication-guides-editor" onClose={()=>{if(!busy)onClose();}}>
    <form onSubmit={event=>{event.preventDefault();if(layoutJson&&!busy&&!pending)void onSave(settings).then(saved=>{if(saved)onClose();});}}>
      <div className="publication-guides-editor-layout"><div>
        <fieldset><legend>{zh.publicationGridSettings}</legend><div className="publication-guides-grid-fields">{fields.map(field=><label key={field}>{zh['publicationGuide_'+field]}<PublicationDimensionInput value={values[field]} onChange={value=>setValues(old=>({...old,[field]:value}))} disabled={busy||pending} min={field==='columns'||field==='rows'?1:0} max={field==='columns'||field==='rows'?24:100} step={field==='columns'||field==='rows'?1:'any'}/></label>)}</div></fieldset>
        {(['vertical','horizontal'] as const).map(axis=>{const lines=axis==='vertical'?vertical:horizontal,update=axis==='vertical'?setVertical:setHorizontal,length=axis==='vertical'?page.widthMm:page.heightMm;return <fieldset key={axis}><legend>{zh['publicationGuide_'+axis]}</legend><div className="publication-custom-guide-list">{lines.map((value,index)=><div key={index}><PublicationDimensionInput aria-label={zh['publicationGuide_'+axis+'Position'].replace('{number}',String(index+1))} min={0} max={length} step="any" disabled={busy||pending} value={value} onChange={next=>update(old=>old.map((item,i)=>i===index?next:item))}/><Button size="small" disabled={busy||pending} aria-label={zh['publicationGuide_'+axis+'Remove'].replace('{number}',String(index+1))} onClick={()=>update(old=>old.filter((_,i)=>i!==index))}>{zh.publicationGuideRemove}</Button></div>)}</div><Button size="small" disabled={busy||pending||lines.length>=32} onClick={()=>update(old=>[...old,nextLine(old,length)])}>{zh['publicationGuide_'+axis+'Add']}</Button></fieldset>;})}
        {!layoutJson&&<p role="alert">{validation}</p>}
      </div><div className="publication-guide-editor-preview"><PublicationPaper page={layoutJson?{...page,layoutJson}:page} previews={previews} continuous={publicationContinuousFrames(layout)} layers={publicationLayerViews(publicationLayers(layout))} assist={{...publicationDefaultGuideView,rulers:true,grid:true,margins:true}} label={zh.publicationGuidesPreview}/></div></div>
      <div className="dialog-actions"><Button type="submit" disabled={busy||pending||!layoutJson}>{zh.publicationSaveGuides}</Button><Button disabled={busy} onClick={onClose}>{zh.publicationBack}</Button></div>
      {notice&&<p role="status">{notice}</p>}
    </form>
  </Modal>;
}
