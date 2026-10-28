import { PublicationTextPreview } from './PublicationTextPreview';
import { publicationPageCover } from '../../domain/publicationCover';
import type { PublicationLayerView } from '../../domain/publicationLayers';
import { useState,useId,useRef,useEffect } from 'react';
import type { ReactNode } from 'react';
import type { PublicationLayout,PublicationPageUpdate,PublicationTextUpdate,PublicationPreview } from '../../domain/publication';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { zh } from '../../app/copy';
import { publicationContinuousFrames,publicationOrdinaryFrames,type ContinuousFrame } from '../../domain/publicationContinuous';
import { publicationTextValues,publicationTextStyle } from './publicationText';

type Page=PublicationLayout['pages'][number];
type Text=Page['texts'][number];
const dimensions=['widthMm','heightMm','marginMm','bleedMm'] as const;
const rectangle=['xMm','yMm','widthMm','heightMm'] as const;
const numeric=(value:string)=>value.trim()===''?NaN:Number(value);
function EditDialog({title,dirty,busy,pending,notice,onClose,children,className='',feedbackAtTop=false}:{feedbackAtTop?:boolean;className?:string;title:string;dirty:boolean;busy:boolean;pending:boolean;notice:string;onClose:()=>void;children:ReactNode}){
  const [confirm,setConfirm]=useState(false);
  const previousFocus=useRef<HTMLElement|null>(null),confirmation=useRef<HTMLDivElement>(null);
  useEffect(()=>{if(confirm&&!pending)confirmation.current?.focus();},[confirm,pending]);
  const resume=()=>{setConfirm(false);const target=previousFocus.current;requestAnimationFrame(()=>{if(target?.isConnected)target.focus();});};
  return <Modal title={title} className={['publication-edit-dialog',className].filter(Boolean).join(' ')} onClose={()=>{if(busy)return;if(dirty&&!pending){if(!confirm)previousFocus.current=document.activeElement instanceof HTMLElement?document.activeElement:null;setConfirm(true);}else onClose();}}>
    {feedbackAtTop&&notice&&<p role="status" className="publication-edit-feedback">{notice}</p>}
    {confirm&&!pending&&<div ref={confirmation} tabIndex={-1} className="publication-edit-confirm" role="group" aria-label={zh.publicationDiscardConfirm}><p>{zh.publicationDiscardConfirm}</p><Button disabled={busy} onClick={onClose}>{zh.publicationDiscardChanges}</Button><Button disabled={busy} onClick={resume}>{zh.publicationBack}</Button></div>}
    {children}
    {!feedbackAtTop&&notice&&<p role="status">{notice}</p>}
  </Modal>;
}
export function PublicationPageEditor({page,isNew,busy,pending,notice,onClose,onSave,onRemove,peer,layout,singleAllowed=true,pairAllowed=false,linkedRemovalBlocked=false,chapterStartTitle,onRemoveLinked}:{onRemoveLinked?:()=>void;chapterStartTitle?:string;peer?:Page;layout?:PublicationLayout;singleAllowed?:boolean;pairAllowed?:boolean;linkedRemovalBlocked?:boolean;page?:Page;isNew:boolean;busy:boolean;pending:boolean;notice:string;onClose:()=>void;onSave:(value:Omit<PublicationPageUpdate,'pageId'>,count?:1|2)=>Promise<boolean>;onRemove?:()=>Promise<boolean>}){
  const [initial]=useState(()=>Object.fromEntries(dimensions.map(key=>[key,String(page?.[key]??({widthMm:210,heightMm:297,marginMm:15,bleedMm:3})[key])])) as Record<typeof dimensions[number],string>);
  const [values,setValues]=useState(initial),[confirmRemove,setConfirmRemove]=useState(false),[count,setCount]=useState<1|2>(singleAllowed?1:2);
  const cover=page?publicationPageCover(page):undefined;
  const dirty=JSON.stringify(values)!==JSON.stringify(initial);
  const data=Object.fromEntries(dimensions.map(key=>[key,numeric(values[key])])) as Omit<PublicationPageUpdate,'pageId'>;
  let pairValid=true;
  if(peer&&page&&layout){try{const preview={...layout,pages:layout.pages.map(item=>item.id===page.id?{...item,...data}:item.id===peer.id?{...item,widthMm:data.widthMm,heightMm:data.heightMm,bleedMm:data.bleedMm}:item)};publicationContinuousFrames(preview);pairValid=[...publicationOrdinaryFrames(peer),...peer.texts].every(item=>item.xMm>=-data.bleedMm&&item.yMm>=-data.bleedMm&&item.xMm+item.widthMm<=data.widthMm+data.bleedMm&&item.yMm+item.heightMm<=data.heightMm+data.bleedMm)&&peer.marginMm*2<Math.min(data.widthMm,data.heightMm);}catch{pairValid=false;}}
  const valid=pairValid&&(!isNew||(count===1?singleAllowed:pairAllowed))&&Object.values(data).every(Number.isFinite)&&data.widthMm>=50&&data.widthMm<=2000&&data.heightMm>=50&&data.heightMm<=2000&&data.marginMm>=0&&data.marginMm*2<Math.min(data.widthMm,data.heightMm)&&data.bleedMm>=0&&data.bleedMm<=25&&(isNew||!page||[...publicationOrdinaryFrames(page),...page.texts].every(item=>item.xMm>=-data.bleedMm&&item.yMm>=-data.bleedMm&&item.xMm+item.widthMm<=data.widthMm+data.bleedMm&&item.yMm+item.heightMm<=data.heightMm+data.bleedMm));
  return <EditDialog title={isNew?zh.publicationAddPage:zh.publicationPageSettings} dirty={dirty} busy={busy} pending={pending} notice={notice} onClose={onClose}>
    <form onSubmit={event=>{event.preventDefault();if(valid&&!busy&&!pending)void onSave(data,count).then(saved=>{if(saved)onClose();});}}>
      {isNew&&pairAllowed&&<label>{zh.publicationInsertCount}<select value={count} disabled={busy||pending} onChange={event=>setCount(Number(event.target.value) as 1|2)}><option value="1" disabled={!singleAllowed}>{zh.publicationInsertOne}</option><option value="2">{zh.publicationInsertPair}</option></select></label>}
      {isNew&&!singleAllowed&&<p>{zh.publicationContinuousInsertionHint}</p>}
      {peer&&<p>{zh.publicationContinuousSizeHint}</p>}
      {cover&&<p>{zh.publicationCoverPageSizeHint}</p>}
      <fieldset className="publication-editor-fields"><legend>{zh.publicationPageSize}</legend>{dimensions.map(key=><label key={key}>{zh[`publicationPageField_${key}`]}<input type="number" step="0.1" value={values[key]} disabled={busy||pending||!!cover&&key==='widthMm'} onChange={event=>setValues(old=>({...old,[key]:event.target.value}))}/></label>)}</fieldset>
      {!valid&&<p role="alert">{zh.publicationInvalidPage}</p>}
      <div className="dialog-actions"><Button variant="primary" type="submit" disabled={busy||pending||!valid||(!isNew&&!dirty)}>{isNew?zh.publicationAddPage:zh.publicationSaveChanges}</Button><Button disabled={busy} onClick={onClose}>{dirty&&!pending?zh.publicationDiscardChanges:zh.publicationBack}</Button></div>
    </form>
    {!isNew&&peer&&onRemoveLinked&&<div className="publication-remove-content"><p>{zh.publicationRemoveLinkedPagesHint}</p><Button disabled={busy||pending||dirty} title={dirty?zh.publicationSaveBeforeNavigate:undefined} onClick={onRemoveLinked}>{zh.publicationRemoveLinkedPages}</Button></div>}
    {!isNew&&linkedRemovalBlocked&&<p>{zh.publicationContinuousPageLocked}</p>}
    {!isNew&&chapterStartTitle&&<p>{zh.publicationChapterStartProtected.replace('{name}',chapterStartTitle)}</p>}
    {!isNew&&onRemove&&!linkedRemovalBlocked&&!chapterStartTitle&&<div className="publication-remove-content">{confirmRemove?<div role="group" aria-label={zh.publicationRemovePageConfirm}><p>{zh.publicationRemovePageHint}</p><Button disabled={busy||pending} onClick={()=>void onRemove().then(saved=>{if(saved)onClose();})}>{zh.publicationRemovePageConfirm}</Button><Button disabled={busy} onClick={()=>setConfirmRemove(false)}>{zh.publicationBack}</Button></div>:<Button disabled={busy||pending} onClick={()=>setConfirmRemove(true)}>{zh.publicationRemovePage}</Button>}</div>}
  </EditDialog>;
}
export function PublicationTextEditor({page,text,textId,kind,busy,pending,notice,previews,onClose,onSave,onRemove,continuous=[],layers=[]}:{page:Page;text?:Text;textId:string;kind:'text'|'page_number';busy:boolean;pending:boolean;notice:string;previews:PublicationPreview[];continuous?:ContinuousFrame[];layers?:PublicationLayerView[];onClose:()=>void;onSave:(value:PublicationTextUpdate)=>Promise<boolean>;onRemove?:()=>Promise<boolean>}){
  const formId=useId();
  const [initial]=useState(()=>publicationTextValues(page,text,textId,kind));
  const [values,setValues]=useState(initial),[confirmRemove,setConfirmRemove]=useState(false);
  const [numbers,setNumbers]=useState(()=>Object.fromEntries([...rectangle,'fontSizePt','lineHeight'].map(key=>[key,String(initial[key as keyof PublicationTextUpdate])])) as Record<typeof rectangle[number]|'fontSizePt'|'lineHeight',string>);
  const data={...values,...Object.fromEntries(Object.entries(numbers).map(([key,value])=>[key,numeric(value)]))};
  const valid=Object.values(numbers).every(value=>Number.isFinite(numeric(value)))&&data.xMm>=-page.bleedMm&&data.yMm>=-page.bleedMm&&data.widthMm>0&&data.heightMm>0&&data.xMm+data.widthMm<=page.widthMm+page.bleedMm&&data.yMm+data.heightMm<=page.heightMm+page.bleedMm&&data.fontSizePt>=4&&data.fontSizePt<=300&&data.lineHeight>=1&&data.lineHeight<=3;
  const dirty=JSON.stringify(data)!==JSON.stringify(initial);
  const directions:Record<NonNullable<PublicationTextUpdate['textDirection']>,string>={horizontal:zh.publicationTextDirection_horizontal,'vertical-rl':zh['publicationTextDirection_vertical-rl'],'vertical-lr':zh['publicationTextDirection_vertical-lr'],'rotate-90':zh['publicationTextDirection_rotate-90'],'rotate-270':zh['publicationTextDirection_rotate-270']};
  const previewText:Text={id:data.textId,xMm:data.xMm,yMm:data.yMm,widthMm:data.widthMm,heightMm:data.heightMm,contentJson:JSON.stringify({kind:data.kind,text:data.text}),styleJson:JSON.stringify(publicationTextStyle(data))};
  const previewPage={...page,texts:[...page.texts.filter(item=>item.id!==data.textId),...(valid?[previewText]:[])]};
  return <EditDialog className="publication-text-editor" feedbackAtTop title={text?zh.publicationEditText:kind==='page_number'?zh.publicationAddPageNumber:zh.publicationAddText} dirty={dirty} busy={busy} pending={pending} notice={notice} onClose={onClose}>
    <div className="publication-object-editor"><form id={formId} onSubmit={event=>{event.preventDefault();if(valid&&!busy&&!pending)void onSave(data).then(saved=>{if(saved)onClose();});}}>
      {data.kind==='text'&&<label>{zh.publicationTextContent}<textarea maxLength={10000} value={values.text} disabled={busy||pending} onChange={event=>setValues(old=>({...old,text:event.target.value}))}/></label>}
      {data.kind==='page_number'&&<p>{zh.publicationAutomaticPageNumber}</p>}
      <fieldset className="publication-editor-fields"><legend>{zh.publicationPositionSize}</legend>{rectangle.map(key=><label key={key}>{zh[`publicationField_${key}`]}<input type="number" step="0.1" value={numbers[key]} disabled={busy||pending} onChange={event=>setNumbers(old=>({...old,[key]:event.target.value}))}/></label>)}</fieldset>
      <fieldset className="publication-editor-fields"><legend>{zh.publicationTextStyle}</legend>
        <label>{zh.publicationTextDirection}<select value={values.textDirection??'horizontal'} disabled={busy||pending} onChange={event=>setValues(old=>({...old,textDirection:event.target.value as PublicationTextUpdate['textDirection']}))}>{Object.entries(directions).map(([direction,label])=><option key={direction} value={direction}>{label}</option>)}</select></label>
        <label>{zh.publicationFontFamily}<select value={values.fontFamily} disabled={busy||pending} onChange={event=>setValues(old=>({...old,fontFamily:event.target.value as PublicationTextUpdate['fontFamily']}))}>{(['serif','sans-serif','monospace'] as const).map(family=><option key={family} value={family}>{zh[`publicationFont_${family}`]}</option>)}</select></label>
        {(['fontSizePt','lineHeight'] as const).map(key=><label key={key}>{zh[`publicationTextField_${key}`]}<input type="number" step="0.1" disabled={busy||pending} value={numbers[key]} onChange={event=>setNumbers(old=>({...old,[key]:event.target.value}))}/></label>)}
        <label>{zh.publicationTextAlign}<select disabled={busy||pending} value={values.align} onChange={event=>setValues(old=>({...old,align:event.target.value as PublicationTextUpdate['align']}))}>{(['left','center','right'] as const).map(align=><option key={align} value={align}>{zh[`publicationAlign_${align}`]}</option>)}</select></label>
        <label>{zh.publicationTextColor}<input type="color" value={values.color} disabled={busy||pending} onChange={event=>setValues(old=>({...old,color:event.target.value}))}/></label>
        <label className="publication-editor-checkbox"><input type="checkbox" checked={values.bold} disabled={busy||pending} onChange={event=>setValues(old=>({...old,bold:event.target.checked}))}/>{zh.publicationTextBold}</label>
      </fieldset>
      {!valid&&<p role="alert">{zh.publicationInvalidText}</p>}
    </form><PublicationTextPreview page={previewPage} textId={data.textId} previews={previews} continuous={continuous} layers={layers}/></div>
    <div className="dialog-actions"><Button variant="primary" type="submit" form={formId} disabled={busy||pending||!valid||(!!text&&!dirty)}>{zh.publicationSaveChanges}</Button><Button disabled={busy} onClick={onClose}>{dirty&&!pending?zh.publicationDiscardChanges:zh.publicationBack}</Button></div>
    {text&&onRemove&&<div className="publication-remove-content">{confirmRemove?<div role="group" aria-label={zh.publicationRemoveTextConfirm}><p>{zh.publicationRemoveTextHint}</p><Button disabled={busy||pending} onClick={()=>void onRemove().then(saved=>{if(saved)onClose();})}>{zh.publicationRemoveTextConfirm}</Button><Button disabled={busy} onClick={()=>setConfirmRemove(false)}>{zh.publicationBack}</Button></div>:<Button disabled={busy||pending} onClick={()=>setConfirmRemove(true)}>{zh.publicationRemoveText}</Button>}</div>}
  </EditDialog>;
}

export { EditDialog as PublicationEditDialog };
