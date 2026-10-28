import { useRef,useState,type ReactNode } from 'react';
import type { CreatePublicationChapter,SetPublicationChapters } from '@betweenframes/contracts';
import { DndContext,KeyboardSensor,PointerSensor,closestCenter,useSensor,useSensors } from '@dnd-kit/core';
import { SortableContext,useSortable,verticalListSortingStrategy,sortableKeyboardCoordinates } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { Grip,ChevronUp,ChevronDown,Trash2 } from 'lucide-react';
import type { PublicationDocument,PublicationLayout } from '../../domain/publication';
import { publicationChapters,publicationChapterRanges } from '../../domain/publicationChapters';
import { publicationContinuousFrames } from '../../domain/publicationContinuous';
import { publicationLayerViews,publicationLayers } from '../../domain/publicationLayers';
import { publicationChaptersPreview,publicationMoveChapter,publicationReorderChapters,publicationChapterOpening,publicationChapterOpeningPreview } from './publicationChapterPlans';
import { publicationPageOrderPreview } from './publicationPageOrder';
import { publicationPageName } from './publicationSpreads';
import { PublicationPaper } from './PublicationPaper';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { zh } from '../../app/copy';
function ChapterShell({title,dirty,busy,pending,notice,onClose,dragging,children}:{title:string;dirty:boolean;busy:boolean;pending:boolean;notice:string;onClose:()=>void;dragging?:React.RefObject<boolean>;children:ReactNode}){
  const [confirm,setConfirm]=useState(false);
  return <Modal title={title} className="publication-chapter-dialog" onEscapeKeyDown={event=>{if(dragging?.current)event.preventDefault();}} onClose={()=>{if(busy)return;if(dirty&&!pending)setConfirm(true);else onClose();}}>
    {children}
    {notice&&<p role="status">{notice}</p>}
    {confirm&&<div className="publication-chapter-discard" role="group" aria-label={zh.publicationDiscardConfirm}><p>{zh.publicationDiscardConfirm}</p><Button onClick={onClose}>{zh.publicationDiscardChanges}</Button><Button onClick={()=>setConfirm(false)}>{zh.publicationBack}</Button></div>}
  </Modal>;
}
function ChapterRow({id,title,disabled,children}:{id:string;title:string;disabled:boolean;children:ReactNode}){
  const {attributes,listeners,setNodeRef,setActivatorNodeRef,transform,transition,isDragging}=useSortable({id,disabled});
  return <fieldset ref={setNodeRef} className={'publication-chapter-row'+(isDragging?' is-dragging':'')} style={{transform:CSS.Transform.toString(transform),transition}}>
    <legend>{title||zh.publicationNewChapterName}</legend>
    <button type="button" className="publication-chapter-drag" ref={setActivatorNodeRef} disabled={disabled} {...attributes} {...listeners} aria-label={zh.publicationDragChapter.replace('{name}',title||zh.publicationNewChapterName)}><Grip size={16}/></button>
    {children}
  </fieldset>;
}
export function PublicationChaptersDialog({document,busy,pending,notice,onClose,onSave,onCreate,onRemovePages}:{document:PublicationDocument;busy:boolean;pending:boolean;notice:string;onClose:()=>void;onSave:(data:SetPublicationChapters)=>Promise<boolean>;onCreate?:()=>void;onRemovePages?:(chapterId:string)=>void}){
  const [initial]=useState<SetPublicationChapters>(()=>({chapters:publicationChapters(document.layout),pageIds:document.layout.pages.map(p=>p.id)}));
  const [draft,setDraft]=useState(initial),[invalidMove,setInvalidMove]=useState(false),dragging=useRef(false);
  const dirty=JSON.stringify(draft)!==JSON.stringify(initial);
  const normalized={...draft,chapters:draft.chapters.map(c=>({...c,title:c.title.trim()}))};
  let preview:PublicationLayout|undefined;try{preview=publicationChaptersPreview(document.layout,normalized);}catch{/* 无效输入保留在表单，禁止提交。 */}
  const pages=publicationPageOrderPreview(document.layout.pages,draft.pageIds)??document.layout.pages;
  const ordered=draft.chapters.slice().sort((a,b)=>pages.findIndex(p=>p.id===a.startPageId)-pages.findIndex(p=>p.id===b.startPageId));
  const rights=new Set(publicationContinuousFrames(document.layout).map(item=>item.right.id)),eligible=pages.filter(p=>p.pageNumber>0&&!rights.has(p.id));
  const unused=eligible.find(p=>!draft.chapters.some(c=>c.startPageId===p.id));
  const ranges=preview?publicationChapterRanges(preview):[];
  const sensors=useSensors(useSensor(PointerSensor,{activationConstraint:{distance:6}}),useSensor(KeyboardSensor,{coordinateGetter:sortableKeyboardCoordinates}));
  const stageOrder=(ids:string[]|undefined)=>{if(ids){setDraft(old=>({...old,pageIds:ids}));setInvalidMove(false);}else setInvalidMove(true);};
  const edit=(id:string,value:{title?:string;startPageId?:string})=>{setInvalidMove(false);setDraft(old=>({...old,chapters:old.chapters.map(c=>c.id===id?{...c,...value}:c)}));};
  return <ChapterShell title={zh.publicationManageChapters} dirty={dirty} busy={busy} pending={pending} notice={notice} onClose={onClose} dragging={dragging}>
    <p className="micro">{zh.publicationChapterNamesHint}</p>
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragStart={()=>{dragging.current=true;}} onDragCancel={()=>{dragging.current=false;}} onDragEnd={({active,over})=>{
      dragging.current=false;if(busy||pending||!preview||!over||active.id===over.id)return;
      const ids=ordered.map(c=>c.id),from=ids.indexOf(String(active.id)),to=ids.indexOf(String(over.id));if(from<0||to<0)return;ids.splice(to,0,ids.splice(from,1)[0]);stageOrder(publicationReorderChapters(preview,ids));
    }} accessibility={{screenReaderInstructions:{draggable:zh.publicationChapterDragKeyboard},announcements:{onDragStart:({active})=>zh.publicationChapterPicked.replace('{name}',ordered.find(c=>c.id===active.id)?.title??''),onDragOver:({over})=>over?zh.publicationChapterOver.replace('{name}',ordered.find(c=>c.id===over.id)?.title??''):zh.publicationPageDragOutside,onDragEnd:()=>zh.publicationChapterOrderPreview,onDragCancel:()=>zh.publicationPageDragCancelled}}}>
      <SortableContext items={ordered.map(c=>c.id)} strategy={verticalListSortingStrategy}>
        <div className="publication-chapter-list">{!ordered.length&&<p>{zh.publicationNoChapters}</p>}
          {ordered.map((chapter,index)=>{
            const range=ranges.find(c=>c.id===chapter.id),first=range?.pages[0],last=range?.pages[range.pages.length-1];
            return <ChapterRow key={chapter.id} id={chapter.id} title={chapter.title} disabled={busy||pending||!preview}>
              <label>{zh.publicationChapterName}<input maxLength={120} value={chapter.title} disabled={busy||pending} onChange={event=>edit(chapter.id,{title:event.target.value})}/></label>
              <label>{zh.publicationChapterStart}<select value={chapter.startPageId} disabled={busy||pending} onChange={event=>edit(chapter.id,{startPageId:event.target.value})}>{eligible.filter(p=>p.id===chapter.startPageId||!draft.chapters.some(c=>c.startPageId===p.id)).map(p=><option key={p.id} value={p.id}>{publicationPageName(p.pageNumber)}</option>)}</select></label>
              {first&&last&&<p className="publication-chapter-range">{publicationPageName(first.pageNumber)}{first.id!==last.id?' — '+publicationPageName(last.pageNumber):''} · {zh.publicationChapterPageCount.replace('{count}',String(range!.pages.length))}</p>}
              <div className="publication-chapter-row-actions"><Button size="small" aria-label={zh.publicationChapterEarlier.replace('{name}',chapter.title)} disabled={busy||pending||!preview||index===0} onClick={()=>stageOrder(preview?publicationMoveChapter(preview,chapter.id,-1):undefined)}><ChevronUp size={14}/>{zh.publicationMoveChapterEarlier}</Button>
                <Button size="small" aria-label={zh.publicationChapterLater.replace('{name}',chapter.title)} disabled={busy||pending||!preview||index===ordered.length-1} onClick={()=>stageOrder(preview?publicationMoveChapter(preview,chapter.id,1):undefined)}><ChevronDown size={14}/>{zh.publicationMoveChapterLater}</Button>
                {onRemovePages&&<Button size="small" disabled={busy||pending||dirty} title={dirty?zh.publicationSaveChaptersFirst:undefined} aria-label={zh.publicationRemoveChapterPagesNamed.replace('{name}',chapter.title)} onClick={()=>onRemovePages(chapter.id)}>{zh.publicationRemoveChapterPages}</Button>}
                <Button size="small" aria-label={zh.publicationRemoveChapterMarker.replace('{name}',chapter.title)} disabled={busy||pending} onClick={()=>{setInvalidMove(false);setDraft(old=>({...old,chapters:old.chapters.filter(c=>c.id!==chapter.id)}));}}><Trash2 size={14}/>{zh.publicationRemoveChapter}</Button></div>
            </ChapterRow>;
          })}</div>
      </SortableContext>
    </DndContext>
    {invalidMove&&<p role="alert">{zh.publicationChapterOrderInvalid}</p>}
    {!preview&&<p role="alert">{zh.publicationChapterInvalid}</p>}
    <p className="micro">{zh.publicationChapterRemoveHint}</p>
    <div className="publication-chapter-add"><Button disabled={busy||pending||!unused||draft.chapters.length>=64} onClick={()=>{if(unused)setDraft(old=>({...old,chapters:[...old.chapters,{id:crypto.randomUUID(),title:zh.publicationNewChapterName,startPageId:unused.id}]}));}}>{zh.publicationMarkChapter}</Button>
      {onCreate&&<Button disabled={busy||pending||dirty||draft.chapters.length>=64} title={dirty?zh.publicationSaveChaptersFirst:undefined} onClick={onCreate}>{zh.publicationCreateChapterPages}</Button>}</div>
    <div className="dialog-actions"><span>{dirty?zh.publicationChapterOrderPreview:zh.publicationChangesSaved}</span><Button variant="primary" disabled={busy||pending||!dirty||!preview} onClick={()=>{if(preview)void onSave(normalized).then(saved=>{if(saved)onClose();});}}>{zh.publicationSaveChapters}</Button><Button disabled={busy} onClick={onClose}>{dirty&&!pending?zh.publicationDiscardChanges:zh.publicationBack}</Button></div>
  </ChapterShell>;
}
export function PublicationChapterOpeningDialog({document,afterPageId,busy,pending,notice,onClose,onSave}:{document:PublicationDocument;afterPageId:string|null;busy:boolean;pending:boolean;notice:string;onClose:()=>void;onSave:(data:CreatePublicationChapter)=>Promise<boolean>}){
  const [ids]=useState(()=>({chapterId:crypto.randomUUID(),pageId:crypto.randomUUID(),textId:crypto.randomUUID(),companionId:crypto.randomUUID()}));
  const [settings,setSettings]=useState({title:'',afterPageId,count:2 as 1|2,startOnRight:true});
  const [initial]=useState(settings);
  const dirty=JSON.stringify(settings)!==JSON.stringify(initial);
  let data:CreatePublicationChapter|undefined,preview:PublicationLayout|undefined;
  try{data=publicationChapterOpening(document.layout,{...ids,...settings,title:settings.title.trim()||zh.publicationChapterTitlePlaceholder});preview=publicationChapterOpeningPreview(document.layout,data);}catch{/* 改位置或插页数量后重新计算，不偷偷改用户选择。 */}
  const rights=new Set(publicationContinuousFrames(document.layout).map(item=>item.left.id));
  const pages=preview?.pages.filter(p=>data?.pages.some(add=>add.pageId===p.id))??[];
  const layers=preview?publicationLayerViews(publicationLayers(preview)):[];
  return <ChapterShell title={zh.publicationCreateChapterPages} dirty={dirty} busy={busy} pending={pending} notice={notice} onClose={onClose}>
    <div className="publication-chapter-opening">
      <div>
        <label>{zh.publicationChapterName}<input autoComplete="off" maxLength={120} placeholder={zh.publicationChapterTitleExample} value={settings.title} disabled={busy||pending} onChange={event=>setSettings(old=>({...old,title:event.target.value}))}/></label>
        <label>{zh.publicationChapterInsertAfter}<select disabled={busy||pending} value={settings.afterPageId??''} onChange={event=>setSettings(old=>({...old,afterPageId:event.target.value||null}))}><option value="">{zh.publicationChapterBookEnd}</option>{document.layout.pages.filter(p=>p.pageNumber>=0&&!rights.has(p.id)).map(p=><option key={p.id} value={p.id}>{publicationPageName(p.pageNumber)}</option>)}</select></label>
        <label>{zh.publicationInsertCount}<select disabled={busy||pending} value={settings.count} onChange={event=>setSettings(old=>({...old,count:Number(event.target.value) as 1|2}))}><option value="1">{zh.publicationInsertOne}</option><option value="2">{zh.publicationInsertPair}</option></select></label>
        <label className="publication-editor-checkbox"><input type="checkbox" checked={settings.startOnRight} disabled={busy||pending} onChange={event=>setSettings(old=>({...old,startOnRight:event.target.checked}))}/>{zh.publicationChapterRightStart}</label>
        <p className="micro">{zh.publicationChapterOpeningHint}</p>
        {!preview&&<p role="alert">{zh.publicationChapterOpeningInvalid}</p>}
      </div>
      <div className="publication-chapter-opening-preview" aria-label={zh.publicationChapterOpeningPreview}>
        {pages.map(p=><PublicationPaper key={p.id} page={p} previews={[]} layers={layers} label={p.id===data?.chapter.startPageId?zh.publicationChapterTitlePage:zh.publicationChapterBlankPage}/>)}
      </div>
    </div>
    <div className="dialog-actions"><Button variant="primary" disabled={busy||pending||!data||!preview||!settings.title.trim()} onClick={()=>{if(data&&settings.title.trim())void onSave(data).then(saved=>{if(saved)onClose();});}}>{zh.publicationCreateChapterConfirm}</Button><Button disabled={busy} onClick={onClose}>{dirty&&!pending?zh.publicationDiscardChanges:zh.publicationBack}</Button></div>
  </ChapterShell>;
}
