import { PublicationCoverDialog } from './PublicationCoverDialog';
import { useCallback,useEffect,useMemo,useRef,useState,type DragEvent } from 'react';
import type { PublicationDocument,PublicationInfo,PublicationRepository,PublicationVersion,PublicationVersionInfo } from '../../domain/publication';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { userFacingError } from '../../services/userFacingError';
import { zh } from '../../app/copy';
import { PublicationDimensionInput } from './PublicationDimensionInput';
import { PublicationPaper } from './PublicationPaper';
import { PublicationPages } from './PublicationPages';
import { PublicationPageEditor,PublicationTextEditor } from './PublicationEditDialogs';
import type { Photo } from '../../domain/models';
import type { PublicationLayout,PublicationPreview } from '../../domain/publication';
import { PublicationPhotoPicker } from './PublicationPhotoPicker';
import { publicationPhotoLayout } from './publicationPhotoLayout';
import { publicationPageName,publicationSpread,publicationAdjacentPage } from './publicationSpreads';
import { publicationMasters,publicationMasterLink } from './publicationMasterData';
import { PublicationSaveMaster,PublicationApplyMaster } from './PublicationMasterDialogs';
import { publicationTextValues } from './publicationText';
import { publicationEffectivePpi } from './publicationQuality';
import { PublicationVersionPicker } from './PublicationVersionPicker';
import type { PublicationLayoutFrame } from '../../domain/publication';
import { publicationRectangleValid,type PublicationRectangle,type PublicationObject } from './publicationGeometry';
import { PublicationArrangementInspector } from './PublicationArrangementInspector';
import { publicationArrange,publicationPageObjects,publicationObjectKey,publicationSelectionObjects,publicationArrangementPreview,publicationArrangementUpdates,type PublicationObjectGeometry,type PublicationSelectionIntent,type PublicationTransformObject,type PublicationArrangeAction } from './publicationArrangement';
import { PublicationMaterials } from './PublicationMaterials';
import { publicationPageOrderPreview,publicationMovePage,publicationInsertionAllowed } from './publicationPageOrder';
import { publicationPlacement,type PublicationPlacement } from './publicationPlacement';
import { publicationContinuousFrames,publicationContinuousBounds,publicationContinuousPair,publicationVisibleFrames,publicationVisiblePhotoIds } from '../../domain/publicationContinuous';
import { PublicationContinuousDialog,type ContinuousEditor } from './PublicationContinuousDialog';
import { publicationFrameNumbers as numbers,publicationFrameValues as values,publicationParseFrame,type PublicationFrameValues as Values } from './publicationFrameForm';
import { publicationLayers,publicationLayerViews,publicationLayerMembers,publicationLayerObjects,publicationChangeLayers,publicationLayerPreview,type PublicationLayers,type PublicationLayerNode } from '../../domain/publicationLayers';
import { PublicationLayersPanel } from './PublicationLayers';
import { publicationGroupRemoval,type PublicationGroupRemoval } from './publicationRemoval';
import { PublicationGroupRemovalDialog } from './PublicationGroupRemoval';
import { PublicationGuideControls,usePublicationGuideView,closePublicationGuideControls } from './PublicationGuideControls';
import { PublicationGuidesEditor } from './PublicationGuidesEditor';
import { PublicationPageRemovalDialog,type PublicationPageRemovalScope } from './PublicationPageRemovalDialog';
import { publicationChapterRemovalIds } from './publicationPageRemoval';
import { PublicationChaptersDialog,PublicationChapterOpeningDialog } from './PublicationChapterDialogs';
import { publicationChapterRanges,publicationChapterChanges } from '../../domain/publicationChapters';
import { PublicationPageOverview } from './PublicationPageOverview';
import './publication-studio.css';

const positions=['xMm','yMm','widthMm','heightMm'] as const;
type TextValues=Record<typeof positions[number],string>;
const textValues=(text:PublicationRectangle):TextValues=>Object.fromEntries(positions.map(key=>[key,String(text[key])])) as TextValues;
function title(version:PublicationVersionInfo){if(version.kind==='saved')return version.label;return ({'创建出版物':zh.publicationAutoCreate,'从作品序列生成':zh.publicationAutoGenerate,'调整版面':zh.publicationAutoEdit,'调整章节':zh.publicationChapterHistoryEdit,'调整封面与书脊':zh.publicationCoverSettings,'移除章节页面':zh.publicationRemoveChapterPages,'移除整组页面':zh.publicationRemovePageGroup,'新建章节页':zh.publicationCreateChapterPages,'开始记录版本':zh.publicationAutoBaseline} as Record<string,string>)[version.label]??(version.label.startsWith('恢复：')?zh.publicationAutoRestore:zh.publicationAutoEdit);}
type Props={repository:PublicationRepository;projectId:string;publication:PublicationInfo;readOnly:boolean;photos?:Photo[];sequencePhotoIds?:string[];selectedPhotoIds?:string[];onClose:()=>void;onChanged:()=>Promise<void>};
type Page=PublicationDocument['layout']['pages'][number];
export function PublicationStudio(props:Props){return <PublicationStudioBody key={`${props.projectId}:${props.publication.id}`} {...props}/>;}
function PublicationStudioBody({repository,projectId,publication,readOnly,photos=[],sequencePhotoIds=[],selectedPhotoIds=[],onClose,onChanged}:Props){
  const [document,setDocument]=useState<PublicationDocument>();
  const [guideView,setGuideView]=usePublicationGuideView(projectId,publication.id);
  const guideControl=useRef<HTMLDetailsElement>(null);
  const [guidesEditor,setGuidesEditor]=useState<{page:Page;document:PublicationDocument}>();
  const [history,setHistory]=useState<PublicationVersionInfo[]>([]),[hasMore,setHasMore]=useState(false);
  const [selectedVersion,setSelectedVersion]=useState<PublicationVersion>();
  const [pageId,setPageId]=useState(''),[frameId,setFrameId]=useState(''),[textId,setTextId]=useState('');
  const [textEdit,setTextEdit]=useState<TextValues>();
  const [multi,setMulti]=useState<{pageId:string;objects:PublicationObject[]}>(),[multiDraft,setMultiDraft]=useState<PublicationObjectGeometry[]>();
  const [sidebar,setSidebar]=useState<'pages'|'versions'|'photos'|'layers'>('pages');
  const [layerDraft,setLayerDraft]=useState<PublicationLayers>(),[layerId,setLayerId]=useState(''),[memberId,setMemberId]=useState('');
  const [pageOrder,setPageOrder]=useState<string[]>(),[placement,setPlacement]=useState<PublicationPlacement&{revision:number}>();
  const pageDragActive=useRef(false);
  const setPageDragging=useCallback((active:boolean)=>{pageDragActive.current=active;},[]);
  const materialDrag=useRef<{photoId:string;token:string}|undefined>(undefined),[dragPhotoId,setDragPhotoId]=useState<string>();
  const [loading,setLoading]=useState(true);
  const [edit,setEdit]=useState<Values>(),[label,setLabel]=useState('');
  const [compare,setCompare]=useState(false),[confirmRestore,setConfirmRestore]=useState(false),[confirmClose,setConfirmClose]=useState(false);
  const [busy,setBusy]=useState(false),[notice,setNotice]=useState('');
  const [pageDraft,setPageDraft]=useState<{isNew:boolean;page?:Page;id:string}>();
  const [textDraft,setTextDraft]=useState<{page:Page;text?:Page['texts'][number];id:string;kind:'text'|'page_number'}>();
  const [photoPicker,setPhotoPicker]=useState<{page:Page;replacement?:string}>();
  const [frameRemoval,setFrameRemoval]=useState<string>();
  const [groupRemoval,setGroupRemoval]=useState<{plan:PublicationGroupRemoval;document:PublicationDocument}>();
  const [photoVersionFrame,setPhotoVersionFrame]=useState<{frame:PublicationLayoutFrame;revision:number}>();
  const [spread,setSpread]=useState(false),[zoom,setZoom]=useState<number>();
  const [continuousEditor,setContinuousEditor]=useState<ContinuousEditor>();
  const [coverDialog,setCoverDialog]=useState<PublicationDocument>();
  const [pageRemoval,setPageRemoval]=useState<PublicationPageRemovalScope>();
  const [chapterDialog,setChapterDialog]=useState<{type:'manage'|'create';document:PublicationDocument;afterPageId:string|null}>();
  const [chapterOrderConsent,setChapterOrderConsent]=useState('');
  const [masterDialog,setMasterDialog]=useState<{type:'save'|'apply';existing?:string}>();
  const overviewHeading=useRef<HTMLHeadingElement>(null);
  const lock=useRef(false),alive=useRef(true),readGeneration=useRef(0);
  const sourceLayout=selectedVersion?.layout??document?.layout;
  const layout=useMemo(()=>{const pages=!selectedVersion&&sourceLayout&&pageOrder?publicationPageOrderPreview(sourceLayout.pages,pageOrder):undefined;const base=pages?{...sourceLayout!,pages}:sourceLayout;return base&&layerDraft&&!selectedVersion?publicationLayerPreview(base,layerDraft):base;},[sourceLayout,pageOrder,selectedVersion,layerDraft]);
  const chapters=useMemo(()=>layout?publicationChapterRanges(layout):[],[layout]);
  const chapterOrderChanges=useMemo(()=>document&&layout&&pageOrder?publicationChapterChanges(document.layout,layout.pages):[],[document,layout,pageOrder]);
  const chapterOrderKey=JSON.stringify([document?.revision,pageOrder]);
  const layerData=useMemo(()=>layout?publicationLayers(layout):{nodes:[]},[layout]);
  const layerViews=useMemo(()=>publicationLayerViews(layerData),[layerData]);
  const layerObjects=useMemo(()=>layout?publicationLayerObjects(layout):new Map(),[layout]);
  const continuous=useMemo(()=>layout?publicationContinuousFrames(layout):[],[layout]);
  const page=continuous.find(item=>item.frame.id===frameId)?.left??layout?.pages.find(p=>p.id===pageId)??layout?.pages[0];
  const frame=page?.frames.find(f=>f.id===frameId);
  const linkedFrame=continuous.find(item=>item.frame.id===frameId);
  const visibleFrames=page?publicationVisibleFrames(page,continuous):[];
  const facing=layout&&page?publicationSpread(layout.pages,page):[];
  const eligiblePair=(()=>{try{if(layout&&facing[0]&&facing[1]&&layout.publication.pubType==='photobook')return publicationContinuousPair(layout.pages,facing[0].id,facing[1].id);}catch{/* 只有实体、同开本的相邻左右页可连续展开。 */}return undefined;})();
  const selectedText=page?.texts.find(t=>t.id===textId);
  const textBaseline=selectedText?textValues(selectedText):undefined;
  const masterDefinitions=useMemo(()=>layout?publicationMasters(layout):[],[layout]);
  const masterLink=page?publicationMasterLink(page):undefined;
  const masterState=document?.masterStates?.find(state=>state.pageId===page?.id);
  const linkedMaster=masterDefinitions.find(master=>master.id===masterLink?.id);
  const baseline=frame?values(frame):undefined;
  const frameDirty=Boolean(edit&&baseline&&JSON.stringify(edit)!==JSON.stringify(baseline));
  const textDirty=Boolean(textEdit&&textBaseline&&JSON.stringify(textEdit)!==JSON.stringify(textBaseline));
  const activeMulti=multi?.pageId===page?.id?multi:undefined;
  const selectionBlocked=activeMulti?.objects.some(object=>{const state=layerViews.find(node=>node.id===object.id);return state?.effectiveLocked||state?.effectiveVisible===false;});
  const multiPlan=page&&activeMulti&&multiDraft?publicationArrangementUpdates(page,multiDraft):undefined;
  const multiDirty=!!multiPlan&&(multiPlan.frames.length+multiPlan.texts.length)>0;
  const objectDirty=frameDirty||textDirty||multiDirty;
  const dirty=objectDirty||!!pageOrder||!!placement||!!layerDraft;
  const navigationLocked=busy||objectDirty||!!placement;
  const pending=repository.pendingCommands?.(projectId).some(r=>r.publicationId===publication.id);
  const writable=!readOnly&&!selectedVersion&&document?.layout.publication.state==='draft'&&!pending;
  const selectedLayer=layerViews.find(item=>item.id===(frame?.id??selectedText?.id));
  const canEditSelected=writable&&!layerDraft&&selectedLayer?.effectiveVisible!==false&&!selectedLayer?.effectiveLocked;
  const selectedBlocked=selectedLayer?.effectiveLocked||selectedLayer?.effectiveVisible===false;
  const canRestore=!readOnly&&Boolean(selectedVersion)&&Boolean(document?.revision??publication.revision)&&publication.state==='draft'&&!pending;
  const read=useCallback(()=>Promise.allSettled([Promise.resolve().then(()=>repository.document!(projectId,publication.id)),Promise.resolve().then(()=>repository.versions!(projectId,publication.id))]),[repository,projectId,publication.id]);
  const applyRead=useCallback(([current,versions]:Awaited<ReturnType<typeof read>>)=>{
    setLoading(false);
    if(current.status==='fulfilled'){setDocument(current.value);setMulti(old=>{if(!old)return;const target=current.value.layout.pages.find(p=>p.id===old.pageId);return target&&publicationSelectionObjects(target,old.objects)?old:undefined;});}else{setDocument(undefined);setNotice(userFacingError(current.reason,zh.publicationLoadFailed));}
    if(versions.status==='fulfilled'){setHistory(versions.value);setHasMore(versions.value.length===30);}else setNotice(userFacingError(versions.reason,zh.publicationHistoryFailed));
    return current.status==='fulfilled'&&versions.status==='fulfilled';
  },[setMulti,setNotice]);
  const reload=async()=>{const generation=++readGeneration.current;const result=await read();if(alive.current&&generation===readGeneration.current)return applyRead(result);};
  useEffect(()=>{alive.current=true;let active=true;const generation=++readGeneration.current;void read().then(result=>{if(active&&generation===readGeneration.current)applyRead(result);});return()=>{active=false;alive.current=false;};},[read,applyRead]);
  const run=async(operation:()=>Promise<void>,success:string,pane:'pages'|'versions'|'photos'|'layers'='pages')=>{
    if(lock.current||readOnly)return false;lock.current=true;setBusy(true);setNotice('');readGeneration.current++;
    try{await operation();if(!alive.current)return false;setSelectedVersion(undefined);setConfirmRestore(false);setEdit(undefined);setTextEdit(undefined);setMultiDraft(undefined);setLayerDraft(undefined);setPageOrder(undefined);setChapterOrderConsent('');setPlacement(undefined);setLabel('');setSidebar(pane);const loaded=await reload();if(!alive.current)return false;setNotice(loaded?success:zh.publicationSavedReloadFailed);try{await onChanged();}catch{setNotice(zh.publicationSavedReloadFailed);}return true;}
    catch(error){if(alive.current)setNotice(userFacingError(error,zh.publicationSaveFailed));return false;}
    finally{lock.current=false;if(alive.current)setBusy(false);}
  };
  const openVersion=async(version:PublicationVersionInfo)=>{
    if(lock.current||dirty)return;lock.current=true;setBusy(true);setNotice('');const generation=++readGeneration.current;
    try{const result=await repository.version!(projectId,publication.id,version.id);if(alive.current&&generation===readGeneration.current){setSelectedVersion(result);setLayerId('');setMemberId('');setMulti(undefined);setMultiDraft(undefined);setFrameId('');setTextId('');setTextEdit(undefined);setEdit(undefined);setConfirmRestore(false);}}
    catch(error){if(alive.current&&generation===readGeneration.current)setNotice(userFacingError(error,zh.publicationHistoryFailed));}
    finally{lock.current=false;if(alive.current)setBusy(false);}
  };
  const layerName=(node:PublicationLayerNode)=>node.name??(node.kind==='group'?zh.publicationGroup:node.kind==='text'?(()=>{const text=layout?.pages.flatMap(item=>item.texts).find(item=>item.id===node.id);try{return JSON.parse(text?.contentJson??'{}').kind==='page_number'?zh.publicationAutomaticPageNumber:JSON.parse(text?.contentJson??'{}').text?.slice(0,32)||zh.publicationTextObject;}catch{return zh.publicationTextObject;}})():photos.find(photo=>photo.id===document?.previews.find(preview=>preview.frameId===node.id)?.photoId)?.name??zh.publicationFrame);
  const scenePages=page&&layout?(spread?publicationSpread(layout.pages,page).filter((item):item is Page=>!!item):[page]):[];
  const sceneIds=scenePages.flatMap(item=>[...publicationVisibleFrames(item,continuous).map(frame=>frame.id),...item.texts.map(text=>text.id)]);
  const canGroup=!!repository.setLayers&&!!activeMulti&&activeMulti.objects.length>=2&&!activeMulti.objects.some(object=>layerViews.find(node=>node.id===object.id)?.effectiveLocked)&&!continuous.some(item=>activeMulti.objects.some(object=>object.id===item.frame.id))&&activeMulti.objects.every(object=>layerData.nodes.find(node=>node.id===object.id)?.parentId===layerData.nodes.find(node=>node.id===activeMulti.objects[0].id)?.parentId)&&layerViews.find(node=>node.id===layerId)?.kind!=='group';
  const stageLayers=(type:'visible'|'locked'|'front'|'back'|'forward'|'backward'|'group'|'ungroup'|'name',id:string,name?:string)=>{
    if(!layout||!repository.setLayers||!writable||busy||objectDirty||pageOrder||placement)return;
    try{const action=type==='group'?{type,id:crypto.randomUUID(),ids:activeMulti?.objects.map(item=>item.id)??[],name:zh.publicationGroup}:type==='name'?{type,id,name:name??''}:['front','back','forward','backward'].includes(type)?{type:type as 'front'|'back'|'forward'|'backward',id,sceneIds}:{type:type as 'visible'|'locked'|'ungroup',id};const next=publicationChangeLayers(layout,layerData,action);setLayerDraft(next);setSidebar('layers');setLayerId(type==='group'?action.id:type==='ungroup'?'':id);setNotice('');setMulti(undefined);setMultiDraft(undefined);setFrameId('');setTextId('');setEdit(undefined);setTextEdit(undefined);setMemberId('');}catch(error){setNotice(userFacingError(error,zh.publicationLayerInvalid));}
  };
  const requestGroupRemoval=(id:string)=>{
    if(!document||!writable||busy||dirty||!repository.removeObjects)return;
    try{setGroupRemoval({plan:publicationGroupRemoval(document.layout,id),document});setNotice('');}catch(error){setNotice(userFacingError(error,zh.publicationLayerLockedHint));}
  };
  const selectLayer=(node:PublicationLayerNode,additive=false)=>{
    if(busy||objectDirty||pageOrder||placement)return;
    const members=publicationLayerMembers(layerData,node.id,layerViews),owner=members[0]?layout?.pages.find(item=>item.id===layerObjects.get(members[0].id)?.pageId):undefined;if(!owner)return;
    if(continuous.some(item=>members.some(member=>member.id===item.frame.id)))setSpread(true);
    if(additive&&node.kind!=='group'){chooseObject(owner,{kind:node.kind,id:node.id},{additive:true,member:true});return;}
    setPageId(owner.id);setMemberId(node.kind==='group'?'':node.id);setLayerId(node.id);setFrameId(members.length===1&&members[0].kind==='frame'?members[0].id:'');setTextId(members.length===1&&members[0].kind==='text'?members[0].id:'');setEdit(undefined);setTextEdit(undefined);setMultiDraft(undefined);setMulti(members.length>1?{pageId:owner.id,objects:members.map(member=>({kind:member.kind as 'frame'|'text',id:member.id}))}:undefined);
  };
  const groupSelection=(item:Page,object:PublicationObject)=>{
    if(object.id===memberId)return;const group=layerViews.find(node=>node.id===object.id)?.rootGroup;if(!group)return;const members=publicationLayerMembers(layerData,group,layerViews);if(members.some(member=>{const state=layerViews.find(value=>value.id===member.id);return state?.effectiveLocked||state?.effectiveVisible===false;}))return;return publicationSelectionObjects(item,members.map(node=>({kind:node.kind as 'frame'|'text',id:node.id})));
  };
  const close=()=>{if(lock.current)return;if(dirty&&!pending)setConfirmClose(true);else onClose();};
  const proposed=(()=>{if(!frame||!edit||!page)return undefined;const geometry=publicationParseFrame(edit),bounds=linkedFrame?publicationContinuousBounds(linkedFrame.left,linkedFrame.right):page;if(!geometry||!publicationRectangleValid(bounds,geometry)||linkedFrame&&(geometry.xMm>=linkedFrame.left.widthMm||geometry.xMm+geometry.widthMm<=linkedFrame.left.widthMm))return undefined;return {...frame,...geometry};})();
  const proposedText=(()=>{if(!selectedText||!page||!textEdit)return undefined;const geometry=Object.fromEntries(positions.map(key=>[key,textEdit[key].trim()===''?NaN:Number(textEdit[key])])) as unknown as PublicationRectangle;return publicationRectangleValid(page,geometry)?{...selectedText,...geometry}:undefined;})();
  const currentPage=document?.layout.pages.find(p=>p.id===page?.id);
  const editorPair=continuous.find(item=>item.left.id===pageDraft?.id||item.right.id===pageDraft?.id);
  const editorPeer=editorPair?(editorPair.left.id===pageDraft?.id?editorPair.right:editorPair.left):undefined;
  const insertionAfter=pageDraft?.page?.side==='back'?null:pageDraft?.page?.id??null;
  const canInsertSingle=!document||publicationInsertionAllowed(document.layout.pages,insertionAfter,1);
  const pageIndex=layout?.pages.findIndex(p=>p.id===page?.id)??-1;
  const previousPage=layout&&page?publicationAdjacentPage(layout.pages,page,spread,-1):undefined;
  const nextPage=layout&&page?publicationAdjacentPage(layout.pages,page,spread,1):undefined;

  const choosePage=(id:string)=>{if(navigationLocked)return;setLayerId('');setMemberId('');setPageId(id);setMulti(undefined);setMultiDraft(undefined);setFrameId('');setTextId('');setTextEdit(undefined);setEdit(undefined);};
  const openPagePhotos=()=>{if(page){setNotice('');setPhotoPicker({page});}};
  const openPageText=()=>{if(page){setNotice('');setTextDraft({page,id:crypto.randomUUID(),kind:'text'});}};
  const openPageSettings=()=>{if(page){setNotice('');setPageDraft({isNew:false,page,id:page.id});}};
  const pagePair=continuous.find(item=>item.left.id===page?.id||item.right.id===page?.id);
  const moveTarget=(delta:number)=>pagePair?layout!.pages.findIndex(item=>item.id===pagePair.left.id)+(delta>0?2:-1):pageIndex+delta;
  const canMove=(delta:number)=>!!layout&&!!page&&!!layout.pages[moveTarget(delta)]&&!!publicationMovePage(layout.pages,page.id,layout.pages[moveTarget(delta)].id);
  const movePage=(delta:number)=>{if(!layout||!document||busy||objectDirty||layerDraft||placement||!writable||!repository.reorderPages)return;const target=moveTarget(delta);if(target<0||target>=layout.pages.length||page!.pageNumber<=0||layout.pages[target].pageNumber<=0)return;const ids=publicationMovePage(layout.pages,page!.id,layout.pages[target].id);if(ids)stagePageOrder(ids,page!.id);else setNotice(zh.publicationContinuousOrderInvalid);};
  const stagePageOrder=(ids:string[],activeId:string)=>{if(!document||!writable||busy||objectDirty||layerDraft||placement||!publicationPageOrderPreview(document.layout.pages,ids))return;setPageId(activeId);setFrameId('');setTextId('');setEdit(undefined);setTextEdit(undefined);setMulti(undefined);setMultiDraft(undefined);setNotice('');setChapterOrderConsent('');setPageOrder(ids.every((id,index)=>id===document.layout.pages[index]?.id)?undefined:ids);};
  const canPlaceMaterial=!!document&&!!writable&&!busy&&!dirty&&!!repository.arrangeFrames;
  const placedOnPage=(target:Page)=>publicationVisiblePhotoIds(target,continuous,document?.previews??[]);
  const placeMaterial=(photoId:string,target:Page,point:{xMm:number;yMm:number},emptyFrameId?:string)=>{
    if(!canPlaceMaterial||!document)return;const photo=photos.find(value=>value.id===photoId);if(!photo)return;if(placedOnPage(target).includes(photoId)){setNotice(zh.publicationPhotoPlaced);return;}
    const plan=publicationPlacement(target,photo,point,()=>crypto.randomUUID(),emptyFrameId);if(!plan){setNotice(zh.publicationPlacementUnavailable);return;}setNotice('');setPageId(target.id);setFrameId('');setTextId('');setMulti(undefined);setPlacement({...plan,revision:document.revision});
  };
  const endMaterialDrag=()=>{materialDrag.current=undefined;setDragPhotoId(undefined);};
  const startMaterialDrag=(photo:Photo,event:DragEvent<HTMLElement>)=>{if(!canPlaceMaterial){event.preventDefault();return;}const token=crypto.randomUUID();materialDrag.current={photoId:photo.id,token};event.dataTransfer.setData('application/x-focalroom-photo',token);event.dataTransfer.effectAllowed='copy';setDragPhotoId(photo.id);};
  const dropMaterial=(target:Page,event:DragEvent<HTMLDivElement>,emptyFrameId?:string)=>{const drag=materialDrag.current;try{if(!drag||event.dataTransfer.getData('application/x-focalroom-photo')!==drag.token||!canPlaceMaterial)return;const bounds=event.currentTarget.getBoundingClientRect();if(bounds.width<=0||bounds.height<=0)return;placeMaterial(drag.photoId,target,{xMm:(event.clientX-bounds.left)/bounds.width*target.widthMm,yMm:(event.clientY-bounds.top)/bounds.height*target.heightMm},emptyFrameId);}finally{endMaterialDrag();}};
  const chooseObject=(item:Page,object:PublicationObject,intent:PublicationSelectionIntent={})=>{
    if(busy||objectDirty||pageOrder||placement)return;
    if(intent.preserve&&activeMulti?.objects.some(value=>publicationObjectKey(value)===publicationObjectKey(object)))return;
    const member=intent.member||intent.preserve&&memberId===object.id,node=layerViews.find(value=>value.id===object.id);
    if(node?.rootGroup&&!member&&!intent.additive){const group=layerData.nodes.find(value=>value.id===node.rootGroup);if(group){selectLayer(group);return;}}
    setLayerId(object.id);setMemberId(member?object.id:'');
    const projected=object.kind==='frame'?continuous.find(value=>value.frame.id===object.id):undefined;
    if(intent.additive&&(projected||linkedFrame)){setNotice(zh.publicationContinuousSeparate);return;}
    if(projected){item=projected.left;setSpread(true);}
    if(intent.preserve&&activeMulti?.objects.some(value=>publicationObjectKey(value)===publicationObjectKey(object)))return;
    if(intent.additive){
      setLayerId('');
      if(page?.id!==item.id){setNotice(zh.publicationArrangeSamePage);return;}
      const existing=activeMulti?.objects??(frame?[{kind:'frame' as const,id:frame.id}]:selectedText?[{kind:'text' as const,id:selectedText.id}]:[]);
      const index=existing.findIndex(value=>publicationObjectKey(value)===publicationObjectKey(object));const next=index>=0?existing.filter((_,i)=>i!==index):[...existing,object];
      if(next.length>100){setNotice(zh.publicationArrangeLimit);return;}
      setNotice('');setEdit(undefined);setTextEdit(undefined);setMultiDraft(undefined);
      if(next.length>1){setMulti({pageId:item.id,objects:next});setFrameId('');setTextId('');}
      else {setMulti(undefined);setFrameId(next[0]?.kind==='frame'?next[0].id:'');setTextId(next[0]?.kind==='text'?next[0].id:'');}return;
    }
    setNotice('');setPageId(item.id);setMulti(undefined);setMultiDraft(undefined);setFrameId(object.kind==='frame'?object.id:'');setTextId(object.kind==='text'?object.id:'');setEdit(undefined);setTextEdit(undefined);
  };
  const selectFrame=(item:Page,id:string,intent?:PublicationSelectionIntent)=>chooseObject(item,{kind:'frame',id},intent);
  const selectText=(item:Page,id:string,intent?:PublicationSelectionIntent)=>chooseObject(item,{kind:'text',id},intent);
  const selectAllObjects=(item:Page)=>{if(busy||dirty)return;const objects=publicationPageObjects(item);if(objects.length>100){setNotice(zh.publicationArrangeLimit);return;}if(objects.length===1){chooseObject(item,objects[0]);return;}setPageId(item.id);setFrameId('');setTextId('');setEdit(undefined);setTextEdit(undefined);setMultiDraft(undefined);setMulti(objects.length?{pageId:item.id,objects:objects.map(({kind,id})=>({kind,id}))}:undefined);};
  const arrangeSelection=(action:PublicationArrangeAction,reference:'selection'|'page')=>{
    if(!page||!activeMulti||!writable||busy||!repository.arrangeObjects)return;const preview=publicationArrangementPreview(page,multiDraft),objects=publicationSelectionObjects(preview,activeMulti.objects);const result=objects?publicationArrange(preview,objects,action,reference):undefined;if(!result){setNotice(zh.publicationArrangeSpaceHint);return;}setNotice('');setMultiDraft(result);
  };
  const openTextEditor=(item:Page,id:string)=>{if(busy||dirty||!writable||!repository.editText)return;const text=item.texts.find(t=>t.id===id);if(text){setNotice('');setTextDraft({page:item,text,id,kind:'text'});}};
  const transformObject=(item:Page,object:PublicationTransformObject,rectangle:PublicationRectangle,members?:PublicationObjectGeometry[])=>{
    const projected=object.kind==='frame'?continuous.find(value=>value.frame.id===object.id):undefined;
    if(projected)item=projected.left;
    if(busy||!writable||layerDraft||object.kind!=='selection'&&layerViews.find(node=>node.id===object.id)?.effectiveLocked||!publicationRectangleValid(projected?publicationContinuousBounds(projected.left,projected.right):item,rectangle)||projected&&(rectangle.xMm>=projected.left.widthMm||rectangle.xMm+rectangle.widthMm<=projected.left.widthMm))return;setNotice('');
    if(object.kind==='selection'){
      const selection=activeMulti??(members&&layerViews.some(node=>node.kind==='group'&&publicationLayerMembers(layerData,node.id,layerViews).map(value=>value.id).sort().join()===members.map(value=>value.id).sort().join())?{pageId:item.id,objects:members.map(({kind,id})=>({kind,id}))}:undefined);
      if(!selection||selection.pageId!==item.id||object.id!==item.id||!repository.arrangeObjects||!members||members.length!==selection.objects.length||members.some(member=>{const state=layerViews.find(node=>node.id===member.id);return state?.effectiveLocked||state?.effectiveVisible===false;}))return;
      if(!activeMulti)setMulti(selection);
      const keys=new Set(selection.objects.map(publicationObjectKey));if(members.some(value=>!keys.has(publicationObjectKey(value)))||!publicationArrangementUpdates(item,members))return;setMultiDraft(members);return;
    }
    if(multiDirty||memberId!==object.id&&layerViews.find(node=>node.id===object.id)?.rootGroup)return;
    if(object.kind==='frame'){if(textDirty||frameDirty&&frameId!==object.id)return;const saved=item.frames.find(f=>f.id===object.id);if(!saved||!repository.editFrame||layerViews.find(node=>node.id===object.id)?.effectiveLocked)return;const base=frame?.id===object.id&&proposed?proposed:saved;setPageId(item.id);setFrameId(object.id);setTextId('');setTextEdit(undefined);setEdit(values({...base,...rectangle}));}
    else {if(frameDirty||textDirty&&textId!==object.id||!repository.editTextGeometry)return;const saved=item.texts.find(t=>t.id===object.id);if(!saved)return;setPageId(item.id);setTextId(object.id);setFrameId('');setEdit(undefined);setTextEdit(textValues(rectangle));}
  };
  const renderPages=(versionLayout:PublicationLayout,activePage:Page,previews:PublicationPreview[],caption:string,interactive:boolean)=>{
    const projections=publicationContinuousFrames(versionLayout),painting=publicationLayerViews(publicationLayers(versionLayout));
    const paper=(saved:Page)=>{const item=interactive&&placement?.pageId===saved.id?placement.preview:interactive&&activeMulti?.pageId===saved.id?publicationArrangementPreview(saved,multiDraft):saved;const images=interactive&&placement?.pageId===saved.id?[...previews.filter(value=>value.frameId!==placement.frameId),{frameId:placement.frameId,thumbnailSrc:photos.find(photo=>photo.id===placement.photoId)?.src}]:previews;return <PublicationPaper assist={guideView} layers={painting} selectionFor={interactive?object=>groupSelection(saved,object):undefined} continuous={projections} key={(interactive?'editing-':'reference-')+item.id} zoom={zoom} page={interactive&&proposedText&&item.texts.some(t=>t.id===proposedText.id)?{...item,texts:item.texts.map(t=>t.id===proposedText.id?proposedText:t)}:item} previews={images} selectedText={interactive?textId:undefined} selectedFrame={interactive?frameId:undefined} editing={interactive?proposed:undefined} disabled={busy||dirty} active={spread&&interactive&&item.id===page?.id}
      onActivate={spread&&interactive?()=>choosePage(item.id):undefined}
      selectedObjects={interactive&&activeMulti?.pageId===item.id?activeMulti.objects:undefined}
      onSelectAll={interactive?()=>selectAllObjects(item):undefined}
      onSelect={interactive?(id,intent)=>selectFrame(item,id,intent):undefined}
      onSelectText={interactive?(id,intent)=>selectText(item,id,intent):undefined}
      onEditText={interactive?id=>{if(!layerViews.find(node=>node.id===id)?.effectiveLocked)openTextEditor(item,id);}:undefined}
      transformOnly={dirty?(multiDirty?{kind:'selection',id:activeMulti!.pageId}:textDirty?{kind:'text',id:textId}:{kind:'frame',id:frameId}):undefined}
      acceptPhotoDrop={interactive&&canPlaceMaterial&&!!dragPhotoId} onPhotoDrop={(event,empty)=>dropMaterial(saved,event,empty)}
      onTransform={interactive&&writable&&!busy&&!layerDraft&&!selectionBlocked&&!pageOrder&&!placement&&(!activeMulti||repository.arrangeObjects)&&(!selectedText||repository.editTextGeometry)&&(!frameDirty||proposed)&&(!textDirty||proposedText)?(object,rectangle,members)=>transformObject(saved,object,rectangle,members):undefined} ariaLabel={compare&&selectedVersion?caption+' · '+publicationPageName(item.pageNumber):undefined} label={caption}/>;};
    return spread?<div className={`publication-spread${projections.some(value=>value.left.id===activePage.id||value.right.id===activePage.id)?' is-continuous':''}`} role="group" aria-label={caption+' · '+zh.publicationSpreadView}>{publicationSpread(versionLayout.pages,activePage).map((item,index)=>item?paper(item):<div key={'blank-'+index} className="publication-spread-gap" style={{aspectRatio:`${activePage.widthMm}/${activePage.heightMm}`,...(zoom?{width:activePage.widthMm*96/25.4*zoom/100,flex:'0 0 auto',maxWidth:'none'}:{})}}><span>{zh.publicationNoFacingPage}</span></div>)}</div>:paper(activePage);
  };
  return <Modal title={zh.publicationStudio} className="publication-studio" onClose={close} onEscapeKeyDown={event=>{if(pageDragActive.current||materialDrag.current){event.preventDefault();if(materialDrag.current)endMaterialDrag();return;}if(closePublicationGuideControls(guideControl.current))event.preventDefault();}}>
      <header className="publication-studio-header"><div><strong>{publication.title}</strong><span>{selectedVersion?`${zh.publicationVersion} ${selectedVersion.version.revision}`:`${zh.publicationCurrent} · ${document?.revision??publication.revision??'—'}`}{dirty&&<b className="publication-unsaved">{zh.publicationUnsaved}</b>}</span></div>
      <label className="publication-zoom-control">{zh.publicationPageZoom}<select value={zoom??'fit'} disabled={busy} onChange={event=>setZoom(event.target.value==='fit'?undefined:Number(event.target.value))}><option value="fit">{zh.publicationZoomFit}</option>{[25,50,75,100,125,150,200,300,400].map(value=><option key={value} value={value}>{value}%</option>)}</select></label>
      {selectedVersion&&<><Button size="small" disabled={busy} onClick={()=>{readGeneration.current++;setSelectedVersion(undefined);setConfirmRestore(false);setEdit(undefined);}}>{zh.publicationBackToCurrent}</Button><Button size="small" aria-pressed={compare} disabled={busy} onClick={()=>setCompare(value=>!value)}>{zh.publicationCompare}</Button></>}
      {pending&&<Button size="small" disabled={busy||readOnly} onClick={()=>void run(()=>repository.retryPending!(projectId,publication.id),zh.publicationConfirmed)}>{zh.publicationConfirmPrevious}</Button>}
    </header>
    {(repository.addPage||repository.editPage||repository.reorderPages||repository.addText||repository.arrangeFrames||repository.document)&&<div className="publication-content-tools" role="group" aria-label={zh.publicationEditContent}>
      <PublicationGuideControls controlRef={guideControl} view={guideView} onChange={setGuideView} canEdit={!!writable&&!busy&&!dirty&&!!page&&!!repository.setPageGuides} onEdit={()=>{if(page&&document){setNotice('');setGuidesEditor({page,document});}}}/>
      {repository.setCover&&sourceLayout?.publication.pubType==='photobook'&&<Button size="small" disabled={busy||dirty||!writable||loading} onClick={()=>{if(document){setNotice('');setCoverDialog(document);}}}>{zh.publicationCoverSettings}</Button>}
      {repository.addPage&&<Button size="small" disabled={busy||dirty||!writable||loading} onClick={()=>{setNotice('');setPageDraft({isNew:true,page:continuous.find(item=>item.left.id===page?.id)?.right??page,id:crypto.randomUUID()});}}>{zh.publicationAddPage}</Button>}
      {repository.editPage&&<Button size="small" disabled={busy||dirty||!writable||!page} onClick={openPageSettings}>{zh.publicationPageSettings}</Button>}
      {repository.reorderPages&&<><Button size="small" disabled={busy||objectDirty||!!layerDraft||!!placement||!writable||!page||!canMove(-1)} onClick={()=>movePage(-1)}>{pagePair?zh.publicationMoveSpreadEarlier:zh.publicationMovePageEarlier}</Button><Button size="small" disabled={busy||objectDirty||!!layerDraft||!!placement||!writable||!page||!canMove(1)} onClick={()=>movePage(1)}>{pagePair?zh.publicationMoveSpreadLater:zh.publicationMovePageLater}</Button></>}
      {repository.addText&&(['text','page_number'] as const).map(kind=><Button key={kind} size="small" disabled={busy||dirty||!writable||!page} onClick={()=>{setNotice('');setTextDraft({page:page!,id:crypto.randomUUID(),kind});}}>{kind==='text'?zh.publicationAddText:zh.publicationAddPageNumber}</Button>)}
      {repository.arrangeFrames&&<Button size="small" disabled={busy||dirty||!writable||!page||!photos.length||(page.frames.length>=100&&!page.frames.some(f=>!f.assetId))} onClick={openPagePhotos}>{zh.publicationAddPhotos}</Button>}
    </div>}
    {!selectedVersion&&writable&&<p className="publication-direct-help">{zh.publicationDirectHint}</p>}
    <div className="publication-studio-layout" onKeyDown={event=>{if(['Delete','Backspace'].includes(event.key)&&!event.metaKey&&!event.ctrlKey&&!event.altKey&&!event.shiftKey&&(event.target as HTMLElement).closest('.publication-layer-select,[data-publication-kind]')&&layerViews.find(node=>node.id===layerId)?.kind==='group'){event.preventDefault();event.stopPropagation();requestGroupRemoval(layerId);return;}if((event.metaKey||event.ctrlKey)&&event.key.toLowerCase()==='g'&&!(event.target as HTMLElement).closest('input,textarea,select,[contenteditable=true]')){event.preventDefault();event.stopPropagation();if(event.shiftKey&&layerViews.find(node=>node.id===layerId)?.kind==='group')stageLayers('ungroup',layerId);else if(canGroup)stageLayers('group','');}}}><aside className="publication-studio-history">
      <div className="publication-sidebar-tabs" role="tablist" aria-label={zh.publicationBrowse} onKeyDown={event=>{if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;event.preventDefault();const tabs=['pages','photos','layers','versions'] as const;const index=tabs.indexOf(sidebar);const next=tabs[event.key==='Home'?0:event.key==='End'?3:(index+(event.key==='ArrowLeft'?3:1))%4];setSidebar(next);event.currentTarget.querySelector<HTMLButtonElement>(`#publication-${next}-tab`)?.focus();}}>
        <button type="button" role="tab" id="publication-pages-tab" tabIndex={sidebar==='pages'?0:-1} aria-controls="publication-sidebar-panel" aria-selected={sidebar==='pages'} onClick={()=>setSidebar('pages')}>{zh.publicationPages}</button>
        <button type="button" role="tab" id="publication-photos-tab" aria-label={zh.publicationMaterials} tabIndex={sidebar==='photos'?0:-1} aria-controls="publication-sidebar-panel" aria-selected={sidebar==='photos'} onClick={()=>setSidebar('photos')}>{zh.publicationSidebarPhotos}</button>
        <button type="button" role="tab" id="publication-layers-tab" tabIndex={sidebar==='layers'?0:-1} aria-controls="publication-sidebar-panel" aria-selected={sidebar==='layers'} onClick={()=>setSidebar('layers')}>{zh.publicationLayers}</button>
        <button type="button" role="tab" id="publication-versions-tab" aria-label={zh.publicationVersions} tabIndex={sidebar==='versions'?0:-1} aria-controls="publication-sidebar-panel" aria-selected={sidebar==='versions'} onClick={()=>setSidebar('versions')}>{zh.publicationSidebarVersions}</button>
      </div>
      <div id="publication-sidebar-panel" role="tabpanel" aria-labelledby={'publication-'+sidebar+'-tab'}>
      {sidebar==='pages'?<><div className="publication-chapter-navigation">
        {chapters.length>0&&<label>{zh.publicationChapters}<select aria-label={zh.publicationJumpChapter} disabled={navigationLocked} value={chapters.find(c=>c.pages.some(p=>p.id===page?.id))?.id??''} onChange={event=>{const chapter=chapters.find(c=>c.id===event.target.value);if(chapter)choosePage(chapter.startPageId);}}><option value="" disabled>{zh.publicationChapterPrelude}</option>{chapters.map(c=><option key={c.id} value={c.id}>{c.title} · {publicationPageName(c.pages[0].pageNumber)}</option>)}</select></label>}
        {repository.setChapters&&!selectedVersion&&<Button size="small" disabled={busy||dirty||!writable||!document?.layout.pages.length} onClick={()=>{if(document){setNotice('');setChapterDialog({type:'manage',document,afterPageId:page?.side==='back'?null:continuous.find(item=>item.left.id===page?.id)?.right.id??page?.id??null});}}}>{zh.publicationManageChapters}</Button>}
      </div><PublicationPages key={selectedVersion?.version.id??'current'} pages={layout?.pages??[]} layers={layerViews} previews={selectedVersion?.previews??document?.previews??[]} activeId={page?.id} disabled={navigationLocked} onSelect={choosePage} onReorder={writable&&!layerDraft&&repository.reorderPages?stagePageOrder:undefined} onDragActiveChange={setPageDragging} onInvalidReorder={()=>setNotice(zh.publicationContinuousOrderInvalid)}/></>:sidebar==='photos'?<PublicationMaterials photos={photos} sequencePhotoIds={sequencePhotoIds} selectedPhotoIds={selectedPhotoIds} placedPhotoIds={page?placedOnPage(page):[]} disabled={!canPlaceMaterial||!page} onDragStart={startMaterialDrag} onDragEnd={endMaterialDrag} onPlace={photo=>{if(page){const empty=page.frames.filter(frame=>!frame.assetId&&!layerViews.find(node=>node.id===frame.id)?.effectiveLocked&&layerViews.find(node=>node.id===frame.id)?.effectiveVisible!==false&&!layerViews.find(node=>node.id===frame.id)?.parentId).sort((a,b)=>a.yMm-b.yMm||a.xMm-b.xMm)[0];placeMaterial(photo.id,page,{xMm:page.widthMm/2,yMm:page.heightMm/2},empty?.id);}}}/>:sidebar==='layers'?<PublicationLayersPanel layers={layerData} sceneIds={sceneIds} selectedIds={activeMulti?.objects.map(item=>item.id)??(frame?[frame.id]:selectedText?[selectedText.id]:[])} activeId={layerId} label={layerName} disabled={busy||objectDirty||!!pageOrder||!!placement||!writable||!repository.setLayers} dirty={!!layerDraft} canGroup={!!canGroup} onRemove={repository.removeObjects?requestGroupRemoval:undefined} onSelect={selectLayer} onAction={stageLayers} onSave={()=>{if(document&&layerDraft&&writable&&repository.setLayers)void run(()=>repository.setLayers!(projectId,publication.id,document.revision,layerDraft),zh.publicationLayerSaved,'layers');}} onDiscard={()=>{setLayerDraft(undefined);setLayerId('');}}/>:<>
      {!history.length&&<p>{zh.publicationNoVersions}</p>}
      {history.map(version=><button type="button" key={version.id} disabled={busy||dirty} aria-pressed={selectedVersion?.version.id===version.id} onClick={()=>void openVersion(version)}><strong>{title(version)}</strong><span>{zh.publicationVersion} {version.revision} · {new Date(version.createdAt).toLocaleString()}</span></button>)}
      {hasMore&&<Button size="small" disabled={busy||dirty} onClick={()=>{if(lock.current)return;lock.current=true;setBusy(true);void repository.versions!(projectId,publication.id,history[history.length-1].revision).then(rows=>{if(alive.current){setHistory(old=>[...old,...rows]);setHasMore(rows.length===30);}}).catch(error=>{if(alive.current)setNotice(userFacingError(error,zh.publicationHistoryFailed));}).finally(()=>{lock.current=false;if(alive.current)setBusy(false);});}}>{zh.publicationMoreVersions}</Button>}
      </>}</div>
    </aside><main className="publication-studio-preview" tabIndex={0} aria-label={zh.publicationPagePreview} onKeyDown={event=>{
      if(!['PageUp','PageDown'].includes(event.key)||event.altKey||event.ctrlKey||event.metaKey||event.shiftKey||(event.target as HTMLElement).closest('input,textarea,select,[contenteditable=true]'))return;
      const target=event.key==='PageUp'?previousPage:nextPage;
      event.preventDefault();event.stopPropagation();if(target&&!navigationLocked)choosePage(target.id);
    }}>
      {pageOrder&&<div className="publication-staged-edit" role="group" aria-label={zh.publicationOrderPreview}><strong>{zh.publicationOrderPreview}</strong><Button size="small" variant="primary" disabled={busy||!writable||!!chapterOrderChanges.length&&chapterOrderConsent!==chapterOrderKey} onClick={()=>{if(document&&repository.reorderPages)void run(()=>repository.reorderPages!(projectId,publication.id,document.revision,pageOrder,...(chapterOrderChanges.length&&chapterOrderConsent===chapterOrderKey?[true] as const:[])),zh.publicationPageOrderSaved);}}>{zh.publicationSaveOrder}</Button><Button size="small" disabled={busy} onClick={()=>setPageOrder(undefined)}>{zh.publicationDiscardOrder}</Button></div>}
      {pageOrder&&chapterOrderChanges.length>0&&<section className="publication-chapter-order-impact" aria-label={zh.publicationChapterImpact}>
        <p>{zh.publicationChapterOrderImpact.replace('{count}',String(chapterOrderChanges.length))}</p>
        <details><summary>{zh.publicationChapterImpactDetails}</summary><ul>{chapterOrderChanges.map(change=><li key={change.pageId}>{publicationPageName(change.pageNumber)} · {change.before?.title??zh.publicationChapterPrelude} → {change.after?.title??zh.publicationChapterPrelude}</li>)}</ul></details>
        <label><input type="checkbox" checked={chapterOrderConsent===chapterOrderKey} disabled={busy} onChange={event=>setChapterOrderConsent(event.target.checked?chapterOrderKey:'')}/>{zh.publicationChapterConfirmChanges}</label>
      </section>}
      {placement&&<div className="publication-staged-edit" role="group" aria-label={zh.publicationPlacementPreview}><strong>{zh.publicationPlacementPreview} · {photos.find(photo=>photo.id===placement.photoId)?.name}</strong><Button size="small" variant="primary" disabled={busy||!writable} onClick={()=>void run(()=>repository.arrangeFrames!(projectId,publication.id,placement.revision,[],placement.additions,placement.assignments),zh.publicationPhotosPlaced,'photos')}>{zh.publicationConfirmPlacement}</Button><Button size="small" disabled={busy} onClick={()=>setPlacement(undefined)}>{zh.publicationCancelPlacement}</Button></div>}
      {!!layout?.pages.length&&<div className="publication-view-toggle" role="group" aria-label={zh.publicationPreviewMode}><Button size="small" aria-pressed={!spread} disabled={busy} onClick={()=>setSpread(false)}>{zh.publicationSinglePageView}</Button><Button size="small" aria-pressed={spread} disabled={busy} onClick={()=>setSpread(true)}>{zh.publicationSpreadView}</Button></div>}
      {!!layout?.pages.length&&<div className="publication-page-navigation"><Button size="small" aria-label={spread?zh.publicationPreviousSpread:zh.publicationPreviousPage} title={spread?zh.publicationPreviousSpread:zh.publicationPreviousPage} disabled={navigationLocked||!previousPage} onClick={()=>{if(previousPage)choosePage(previousPage.id);}}>←</Button><select aria-label={zh.publicationSelectPage} value={page?.id??''} disabled={navigationLocked} onChange={event=>choosePage(event.target.value)}>{layout.pages.map(p=><option key={p.id} value={p.id}>{publicationPageName(p.pageNumber)}</option>)}</select><span aria-live="polite" aria-label={scenePages.map(item=>publicationPageName(item.pageNumber)).join(' · ')}>{spread&&scenePages.length>1?Math.min(...scenePages.map(item=>layout.pages.findIndex(value=>value.id===item.id)))+1+'–'+(Math.max(...scenePages.map(item=>layout.pages.findIndex(value=>value.id===item.id)))+1):pageIndex+1} / {layout.pages.length}</span><Button size="small" aria-label={spread?zh.publicationNextSpread:zh.publicationNextPage} title={spread?zh.publicationNextSpread:zh.publicationNextPage} disabled={navigationLocked||!nextPage} onClick={()=>{if(nextPage)choosePage(nextPage.id);}}>→</Button></div>}
      {loading&&<p role="status">{zh.publicationLoading}</p>}
      {!page&&!loading&&<div className="publication-empty-page"><p>{zh.publicationNoPages}</p>{repository.addPage&&writable&&<Button variant="primary" disabled={busy||dirty} onClick={()=>setPageDraft({isNew:true,id:crypto.randomUUID()})}>{zh.publicationCreateFirstPage}</Button>}</div>}
      <div className={`publication-studio-papers ${compare&&selectedVersion?'is-comparing':''} ${zoom?'is-zoomed':''}`}>
        {compare&&selectedVersion&&(currentPage?renderPages(document!.layout,currentPage,document!.previews,zh.publicationCurrent,false):<div className="publication-comparison-empty"><p>{zh.publicationPageAbsentCurrent}</p></div>)}
        {page&&layout&&renderPages(layout,page,selectedVersion?.previews??document?.previews??[],selectedVersion?title(selectedVersion.version):zh.publicationCurrent,true)}
      </div>
    </main><aside className="publication-studio-inspector">
      <h3 ref={overviewHeading} tabIndex={-1}>{placement?zh.publicationPlacementPreview:pageOrder?zh.publicationOrderPreview:selectedVersion?zh.publicationSavedView:activeMulti?zh.publicationArrangeTitle:selectedText?zh.publicationTextObject:linkedFrame?zh.publicationContinuousPhoto:frame?zh.publicationFrame:zh.publicationPageOverview}</h3>
      {placement||pageOrder?<><p>{zh.publicationStagedNext}</p>{page&&<p>{publicationPageName(page.pageNumber)}</p>}{placement&&<strong>{photos.find(photo=>photo.id===placement.photoId)?.name}</strong>}</>:<>
      {selectedVersion&&<p>{zh.publicationSavedHint}</p>}
      {page&&!activeMulti&&(visibleFrames.length>0||page.texts.length>0)&&<label className="publication-object-picker">{zh.publicationPageObjects}<select value={frame?'frame:'+frame.id:selectedText?'text:'+selectedText.id:''} disabled={busy||dirty} onChange={event=>{const value=event.target.value;if(value.startsWith('frame:'))selectFrame(page,value.slice(6));else if(value.startsWith('text:'))selectText(page,value.slice(5));}}><option value="" disabled>{zh.publicationChooseObject}</option>{visibleFrames.map((item,index)=><option key={item.id} value={'frame:'+item.id}>{continuous.some(value=>value.frame.id===item.id)?zh.publicationContinuousPhoto:zh.publicationSelectFrame.replace('{number}',String(index+1))}</option>)}{page.texts.map((item,index)=><option key={item.id} value={'text:'+item.id}>{zh.publicationSelectText.replace('{number}',String(index+1))}</option>)}</select></label>}

      {page&&!frame&&!selectedText&&!activeMulti&&<PublicationPageOverview page={page} photoCount={visibleFrames.filter(item=>!!item.assetId).length} emptyFrames={visibleFrames.filter(item=>!item.assetId).length} textCount={page.texts.length} disabled={busy||dirty||!writable}
        onPhotos={!selectedVersion&&repository.arrangeFrames&&photos.length?openPagePhotos:undefined} onText={!selectedVersion&&repository.addText?openPageText:undefined} onSettings={!selectedVersion&&repository.editPage?openPageSettings:undefined}/> }
      {(frame||selectedText||activeMulti)&&<Button size="small" disabled={busy||dirty} title={dirty?zh.publicationSaveBeforeNavigate:undefined} onClick={()=>{if(page){choosePage(page.id);overviewHeading.current?.focus({preventScroll:true});}}}>{zh.publicationReturnToPage}</Button>}
      {selectionBlocked&&<p>{activeMulti?.objects.some(object=>layerViews.find(node=>node.id===object.id)?.effectiveLocked)?zh.publicationLayerLockedHint:zh.publicationLayerHiddenHint}</p>}
      {selectedBlocked&&<p>{selectedLayer?.effectiveLocked?zh.publicationLayerLockedHint:zh.publicationLayerHiddenHint}</p>}
      {linkedFrame&&<p className="publication-continuous-hint">{zh.publicationContinuousSelected}</p>}
      {frame&&<form onSubmit={event=>{event.preventDefault();if(!canEditSelected||!proposed||!document)return;void run(()=>repository.editFrame!(projectId,publication.id,document.revision,{frameId:frame.id,xMm:proposed.xMm,yMm:proposed.yMm,widthMm:proposed.widthMm,heightMm:proposed.heightMm,fitMode:proposed.fitMode,cropX:proposed.cropX,cropY:proposed.cropY,cropZoom:proposed.cropZoom,assetId:frame.assetId,renditionId:frame.renditionId}),zh.publicationSaved);}}>
        {frame.assetId&&<p className="publication-ppi">{zh.publicationImageClarity} · {publicationEffectivePpi(proposed??frame)!==undefined?`${Math.round(publicationEffectivePpi(proposed??frame)!)} ppi`:zh.publicationImageDimensionsUnknown}</p>}
        <fieldset><legend>{zh.publicationPositionSize}</legend>{numbers.slice(0,4).map(key=><label key={key}>{zh[`publicationField_${key}`]}<PublicationDimensionInput step="0.1" disabled={busy||!canEditSelected} value={(edit??baseline!)[key]} onChange={value=>setEdit(old=>({...old??baseline!,[key]:value}))}/></label>)}</fieldset>
        <fieldset><legend>{zh.publicationCrop}</legend>{numbers.slice(4).map(key=><label key={key}>{zh[`publicationField_${key}`]}<input type="number" step={key==='cropZoom'?0.1:1} min={key==='cropZoom'?1:0} max={key==='cropZoom'?20:100} disabled={busy||!canEditSelected} value={(edit??baseline!)[key]} onChange={event=>setEdit(old=>({...old??baseline!,[key]:event.target.value}))}/></label>)}</fieldset>
        <label>{zh.publicationFit}<select disabled={busy||!canEditSelected} value={(edit??baseline!).fitMode} onChange={event=>setEdit(old=>({...old??baseline!,fitMode:event.target.value as Values['fitMode']}))}>{(['fit','fill','stretch'] as const).map(mode=><option key={mode} value={mode}>{zh[`publicationFit_${mode}`]}</option>)}</select></label>
        {dirty&&!proposed&&<p role="alert" className="publication-validation">{linkedFrame?zh.publicationContinuousInvalid:zh.publicationInvalidFrame}</p>}
        <div className="publication-studio-actions"><span>{dirty?zh.publicationSaveBeforeNavigate:zh.publicationChangesSaved}</span><Button variant="primary" type="submit" disabled={busy||!canEditSelected||!dirty||!proposed}>{busy?zh.publicationSaving:zh.publicationSaveChanges}</Button><Button disabled={busy||!dirty} onClick={()=>setEdit(undefined)}>{zh.publicationDiscardChanges}</Button></div>
      </form>}
      {selectedText&&page&&<form onSubmit={event=>{event.preventDefault();if(!canEditSelected||!proposedText||!document||!repository.editTextGeometry)return;void run(()=>repository.editTextGeometry!(projectId,publication.id,document.revision,{textId:selectedText.id,pageId:page.id,xMm:proposedText.xMm,yMm:proposedText.yMm,widthMm:proposedText.widthMm,heightMm:proposedText.heightMm}),zh.publicationSaved);}}>
        <p className="publication-selected-text">{publicationTextValues(page,selectedText,selectedText.id,'text').kind==='page_number'?zh.publicationAutomaticPageNumber:publicationTextValues(page,selectedText,selectedText.id,'text').text.slice(0,120)}</p>
        <fieldset><legend>{zh.publicationPositionSize}</legend>{positions.map(key=><label key={key}>{zh[`publicationField_${key}`]}<PublicationDimensionInput step="0.1" disabled={busy||!canEditSelected||!repository.editTextGeometry} value={(textEdit??textBaseline!)[key]} onChange={value=>setTextEdit(old=>({...old??textBaseline!,[key]:value}))}/></label>)}</fieldset>
        {textDirty&&!proposedText&&<p role="alert" className="publication-validation">{zh.publicationInvalidTextGeometry}</p>}
        <div className="publication-studio-actions"><span>{dirty?zh.publicationSaveBeforeNavigate:zh.publicationChangesSaved}</span><Button variant="primary" type="submit" disabled={busy||!canEditSelected||!textDirty||!proposedText||!repository.editTextGeometry}>{busy?zh.publicationSaving:zh.publicationSaveChanges}</Button><Button disabled={busy||!textDirty} onClick={()=>setTextEdit(undefined)}>{zh.publicationDiscardChanges}</Button></div>
        {repository.editText&&<Button type="button" size="small" disabled={busy||dirty||!canEditSelected} onClick={()=>openTextEditor(page,selectedText.id)}>{zh.publicationEditText}</Button>}
      </form>}
      {frame&&!selectedVersion&&(repository.removeFrame||repository.replaceFramePhoto||repository.photoVersions)&&<div className="publication-frame-actions">
        {frame.assetId&&eligiblePair&&repository.setSpreadFrame&&!linkedFrame&&<Button size="small" disabled={busy||dirty||!canEditSelected||!!selectedLayer?.parentId} title={selectedLayer?.parentId?zh.publicationLayerUngroupFirst:undefined} onClick={()=>{setNotice('');setContinuousEditor({mode:'link',layout:document!.layout,previews:document!.previews,revision:document!.revision,frameId:frame.id,leftId:eligiblePair[0].id,rightId:eligiblePair[1].id});}}>{zh.publicationExpandContinuous}</Button>}
        {linkedFrame&&repository.unsetSpreadFrame&&<Button size="small" disabled={busy||dirty||!canEditSelected} onClick={()=>{setNotice('');setContinuousEditor({mode:'unlink',layout:document!.layout,previews:document!.previews,revision:document!.revision,frameId:frame.id,leftId:linkedFrame.left.id,rightId:linkedFrame.right.id});}}>{zh.publicationUnlinkContinuous}</Button>}
        {repository.photoVersions&&frame.assetId&&<Button size="small" disabled={busy||dirty||!canEditSelected} onClick={()=>{setNotice('');setPhotoVersionFrame({frame,revision:document!.revision});}}>{zh.publicationPhotoVersions}</Button>}
        {repository.replaceFramePhoto&&<Button size="small" disabled={busy||dirty||!canEditSelected||!photos.length} onClick={()=>{setNotice('');setPhotoPicker({page:page!,replacement:frame.id});}}>{frame.assetId?zh.publicationReplacePhoto:zh.publicationFillFrame}</Button>}
        {repository.removeFrame&&<Button size="small" disabled={busy||dirty||!canEditSelected} onClick={()=>{setNotice('');setFrameRemoval(frame.id);}}>{zh.publicationRemoveFrame}</Button>}
      </div>}
      {page&&repository.arrangeObjects&&publicationPageObjects(page).length>0&&<PublicationArrangementInspector key={page.id+':'+(selectedVersion?.version.id??'current')} page={publicationArrangementPreview(page,multiDraft)} selected={activeMulti?.objects??(frame?[{kind:'frame',id:frame.id}]:selectedText?[{kind:'text',id:selectedText.id}]:[])} busy={busy} dirty={dirty} canWrite={!!writable&&!layerDraft&&!selectionBlocked} changes={multiPlan?multiPlan.frames.length+multiPlan.texts.length:0} onToggle={object=>chooseObject(page,object,{additive:true})} onAll={()=>selectAllObjects(page)} onClear={()=>{if(busy||dirty)return;setMulti(undefined);setFrameId('');setTextId('');setMultiDraft(undefined);}} onArrange={arrangeSelection} onDiscard={()=>setMultiDraft(undefined)} onSave={()=>{if(!document||!writable||!multiPlan)return;void run(()=>repository.arrangeObjects!(projectId,publication.id,document.revision,multiPlan.frames,multiPlan.texts),zh.publicationSaved);}}/>}
      {!selectedVersion&&repository.saveMaster&&page&&<section className="publication-master-controls"><h4>{zh.publicationMasters}</h4>
        {masterLink&&<p>{linkedMaster?.name??zh.publicationMasterUnavailable} · {zh.publicationMasterLinked}</p>}
        {masterState?.locallyChanged&&<p className="publication-master-change-hint">{zh.publicationMasterPageChanged}</p>}
        {masterState?.currentRevision&&masterState.currentRevision>masterState.appliedRevision&&<p className="publication-master-change-hint">{zh.publicationMasterHasUpdate}</p>}
        <div><Button size="small" disabled={busy||dirty||!writable} onClick={()=>{setNotice('');setMasterDialog({type:'save'});}}>{zh.publicationSaveMaster}</Button>
        {linkedMaster&&<Button size="small" disabled={busy||dirty||!writable} onClick={()=>{setNotice('');setMasterDialog({type:'save',existing:linkedMaster.id});}}>{zh.publicationUpdateMaster}</Button>}
        {repository.applyMaster&&repository.previewMaster&&<Button size="small" disabled={busy||dirty||!writable||!masterDefinitions.length} onClick={()=>{setNotice('');setMasterDialog({type:'apply'});}}>{zh.publicationApplyMaster}</Button>}</div>
      </section>}
      {!selectedVersion&&<form onSubmit={event=>{event.preventDefault();if(!writable||!document||!label.trim()||dirty)return;void run(()=>repository.saveVersion!(projectId,publication.id,document.revision,label.trim()),zh.publicationVersionSaved,'versions');}}><h4>{zh.publicationSaveVersion}</h4><input aria-label={zh.publicationVersionName} placeholder={zh.publicationVersionName} maxLength={120} value={label} disabled={busy||!writable||dirty} onChange={event=>setLabel(event.target.value)}/><Button type="submit" size="small" disabled={busy||!writable||dirty||!label.trim()}>{zh.publicationSaveVersion}</Button></form>}
      {selectedVersion&&canRestore&&!confirmRestore&&<Button disabled={busy} onClick={()=>setConfirmRestore(true)}>{zh.publicationRestoreVersion}</Button>}
      {confirmRestore&&selectedVersion&&<div role="group" aria-label={zh.publicationRestoreConfirm}><p>{zh.publicationRestoreHint}</p><Button disabled={busy||!canRestore} onClick={()=>void run(()=>repository.restoreVersion!(projectId,publication.id,document?.revision??publication.revision!,selectedVersion.version.id),zh.publicationRestored)}>{zh.publicationRestoreConfirm}</Button><Button disabled={busy} onClick={()=>setConfirmRestore(false)}>{zh.publicationBack}</Button></div>}
    </>} </aside></div>
    <div className="publication-studio-status-slot">{notice&&<p role="status" aria-label={zh.publicationFeedback} className="publication-studio-notice">{notice}</p>}</div>
    {confirmClose&&<div role="group" aria-label={zh.publicationDiscardConfirm}><p>{zh.publicationDiscardConfirm}</p><Button onClick={onClose}>{zh.publicationDiscardChanges}</Button><Button onClick={()=>setConfirmClose(false)}>{zh.publicationBack}</Button></div>}
    {chapterDialog?.type==='manage'&&<PublicationChaptersDialog key={chapterDialog.document.revision} document={chapterDialog.document} busy={busy} pending={!!pending||!writable} notice={notice} onClose={()=>setChapterDialog(undefined)}
      onSave={data=>{if(!writable||!repository.setChapters)return Promise.resolve(false);return run(()=>repository.setChapters!(projectId,publication.id,chapterDialog.document.revision,data),zh.publicationChaptersSaved);}}
      onRemovePages={repository.removePages?chapterId=>{try{setNotice('');setPageRemoval({document:chapterDialog.document,pageIds:publicationChapterRemovalIds(chapterDialog.document.layout,chapterId),chapterId});setChapterDialog(undefined);}catch(error){setNotice(userFacingError(error,zh.publicationRemovePagesInvalid));}}:undefined}
      onCreate={repository.createChapter?()=>{setNotice('');setChapterDialog({...chapterDialog,type:'create'});}:undefined}/>}
    {chapterDialog?.type==='create'&&<PublicationChapterOpeningDialog key={chapterDialog.document.revision} document={chapterDialog.document} afterPageId={chapterDialog.afterPageId} busy={busy} pending={!!pending||!writable} notice={notice} onClose={()=>setChapterDialog(undefined)}
      onSave={data=>{if(!writable||!repository.createChapter)return Promise.resolve(false);return run(()=>repository.createChapter!(projectId,publication.id,chapterDialog.document.revision,data),zh.publicationChapterCreated).then(saved=>{if(saved)setPageId(data.chapter.startPageId);return saved;});}}/>}
    {coverDialog&&<PublicationCoverDialog key={coverDialog.revision} document={coverDialog} busy={busy} pending={!!pending||!writable} stale={!!document&&document.revision!==coverDialog.revision} notice={notice} onClose={()=>setCoverDialog(undefined)} onSave={data=>{if(!writable||!repository.setCover)return Promise.resolve(false);return run(()=>repository.setCover!(projectId,publication.id,coverDialog.revision,data),zh.publicationCoverSaved).then(saved=>{if(saved){setPageId(data.pageId);setFrameId('');setTextId('');setMulti(undefined);}return saved;});}}/>}
    {pageRemoval&&<PublicationPageRemovalDialog key={pageRemoval.document.revision+':'+pageRemoval.pageIds.join(',')} scope={pageRemoval} busy={busy} pending={!!pending||!writable} stale={!!document&&document.revision!==pageRemoval.document.revision} notice={notice} onClose={()=>setPageRemoval(undefined)}
      onConfirm={(data,focusId)=>{if(!writable||!repository.removePages)return Promise.resolve(false);return run(()=>repository.removePages!(projectId,publication.id,pageRemoval.document.revision,data),zh.publicationPagesRemoved).then(saved=>{if(saved){setPageId(focusId??'');setFrameId('');setTextId('');setMulti(undefined);}return saved;});}}/>}
    {pageDraft&&<PublicationPageEditor key={pageDraft.id} page={pageDraft.page} isNew={pageDraft.isNew} peer={!pageDraft.isNew?editorPeer:undefined} layout={document?.layout} onRemoveLinked={!pageDraft.isNew&&editorPair&&document&&repository.removePages?()=>{setNotice('');setPageRemoval({document,pageIds:[editorPair.left.id,editorPair.right.id],chapterId:null});setPageDraft(undefined);}:undefined} chapterStartTitle={!pageDraft.isNew?chapters.find(c=>c.startPageId===pageDraft.id)?.title:undefined} singleAllowed={canInsertSingle} pairAllowed={!!repository.addPages} linkedRemovalBlocked={!pageDraft.isNew&&!!document&&!publicationPageOrderPreview(document.layout.pages.filter(item=>item.id!==pageDraft.id),document.layout.pages.filter(item=>item.id!==pageDraft.id).map(item=>item.id))} busy={busy} pending={!!pending} notice={notice} onClose={()=>{if(!busy)setPageDraft(undefined);}}
      onSave={(data,count)=>{if(!document||!writable)return Promise.resolve(false);return run(()=>pageDraft.isNew?(count===2&&repository.addPages?repository.addPages(projectId,publication.id,document.revision,[{...data,pageId:pageDraft.id,afterPageId:insertionAfter},{...data,pageId:crypto.randomUUID(),afterPageId:pageDraft.id}]):repository.addPage!(projectId,publication.id,document.revision,{...data,pageId:pageDraft.id,afterPageId:insertionAfter})):(editorPeer&&repository.editPages?repository.editPages(projectId,publication.id,document.revision,[{...data,pageId:pageDraft.id},{...data,pageId:editorPeer.id,marginMm:editorPeer.marginMm}]):repository.editPage!(projectId,publication.id,document.revision,{...data,pageId:pageDraft.id})),zh.publicationPageSaved).then(saved=>{if(saved&&pageDraft.isNew)setPageId(pageDraft.id);return saved;});}}
      onRemove={!pageDraft.isNew&&pageDraft.page?.pageNumber!==0&&repository.removePage?()=>{if(!document||!writable)return Promise.resolve(false);return run(()=>repository.removePage!(projectId,publication.id,document.revision,pageDraft.id),zh.publicationPageRemoved).then(saved=>{if(saved){setPageId('');setFrameId('');}return saved;});}:undefined}/>}
    {textDraft&&<PublicationTextEditor key={textDraft.id} page={textDraft.page} text={textDraft.text} textId={textDraft.id} kind={textDraft.kind} busy={busy} pending={!!pending} notice={notice} previews={document?.previews??[]} continuous={continuous} layers={layerViews} onClose={()=>{if(!busy)setTextDraft(undefined);}}
      onSave={data=>{if(!document||!writable)return Promise.resolve(false);return run(()=>textDraft.text?repository.editText!(projectId,publication.id,document.revision,data):repository.addText!(projectId,publication.id,document.revision,data),zh.publicationTextSaved);}}
      onRemove={textDraft.text&&repository.removeText?()=>{if(!document||!writable)return Promise.resolve(false);return run(()=>repository.removeText!(projectId,publication.id,document.revision,textDraft.id),zh.publicationTextRemoved);}:undefined}/>}
    {photoPicker&&<PublicationPhotoPicker layers={layerViews} continuous={continuous} page={photoPicker.page} previews={document?.previews??[]} photos={photos} sequencePhotoIds={sequencePhotoIds} selectedPhotoIds={selectedPhotoIds} replacement={photoPicker.replacement} busy={busy} pending={!!pending} error={notice} onClose={()=>{if(!busy)setPhotoPicker(undefined);}}
      onSave={(ids,columns)=>{
        if(!document||!writable||!ids.length)return Promise.resolve(false);
        if(photoPicker.replacement)return run(()=>repository.replaceFramePhoto!(projectId,publication.id,document.revision,photoPicker.replacement!,ids[0]),zh.publicationPhotoReplaced);
        const chosen=ids.flatMap(id=>{const photo=photos.find(p=>p.id===id);return photo?[photo]:[];});if(chosen.length!==ids.length)return Promise.resolve(false);
        const plan=publicationPhotoLayout(photoPicker.page,chosen,columns,()=>crypto.randomUUID(),new Set(layerViews.filter(node=>node.effectiveLocked||!node.effectiveVisible||node.parentId).map(node=>node.id)));return run(()=>repository.arrangeFrames!(projectId,publication.id,document.revision,plan.updates,plan.additions,plan.assignments),zh.publicationPhotosPlaced);
      }}/>}
    {photoVersionFrame&&document&&<PublicationVersionPicker key={photoVersionFrame.frame.id+':'+photoVersionFrame.revision} repository={repository} projectId={projectId} publicationId={publication.id} revision={photoVersionFrame.revision} frame={photoVersionFrame.frame} busy={busy} pending={!!pending} notice={notice} onClose={()=>{if(!busy)setPhotoVersionFrame(undefined);}} onSave={renditionId=>{if(!writable)return Promise.resolve(false);const next=photoVersionFrame.frame;return run(()=>repository.editFrame!(projectId,publication.id,photoVersionFrame.revision,{frameId:next.id,assetId:next.assetId,renditionId,xMm:next.xMm,yMm:next.yMm,widthMm:next.widthMm,heightMm:next.heightMm,fitMode:next.fitMode,cropX:next.cropX,cropY:next.cropY,cropZoom:next.cropZoom}),zh.publicationPhotoVersionSaved);}}/>}
    {frameRemoval&&<Modal title={zh.publicationRemoveFrameConfirm} description={continuous.some(value=>value.frame.id===frameRemoval)?zh.publicationContinuousRemoveHint:zh.publicationRemoveFrameHint} onClose={()=>{if(!busy)setFrameRemoval(undefined);}}><div className="dialog-actions"><Button disabled={busy||!!pending||!writable} onClick={()=>{if(!document||!writable)return;void run(()=>repository.removeFrame!(projectId,publication.id,document.revision,frameRemoval),zh.publicationFrameRemoved).then(saved=>{if(saved){setFrameRemoval(undefined);setFrameId('');}});}}>{zh.publicationRemoveFrameConfirm}</Button><Button disabled={busy} onClick={()=>setFrameRemoval(undefined)}>{zh.publicationBack}</Button></div>{notice&&<p role="status">{notice}</p>}</Modal>}
    {groupRemoval&&<PublicationGroupRemovalDialog plan={groupRemoval.plan} document={groupRemoval.document} busy={busy} pending={!!pending||!writable} notice={notice} onClose={()=>setGroupRemoval(undefined)} onConfirm={()=>{
      if(!writable||!repository.removeObjects)return;const captured=groupRemoval;
      void run(()=>repository.removeObjects!(projectId,publication.id,captured.document.revision,captured.plan.objects),zh.publicationGroupRemoved,'layers').then(saved=>{if(saved){setGroupRemoval(undefined);setMulti(undefined);setFrameId('');setTextId('');setLayerId('');setMemberId('');}});
    }}/>}
    {continuousEditor&&<PublicationContinuousDialog editor={continuousEditor} busy={busy} pending={!!pending} canWrite={!!writable} notice={notice} onClose={()=>{if(!busy)setContinuousEditor(undefined);}}
      onLink={data=>writable&&repository.setSpreadFrame?run(()=>repository.setSpreadFrame!(projectId,publication.id,continuousEditor.revision,data),zh.publicationContinuousSaved).then(saved=>{if(saved){setPageId(data.leftPageId);setFrameId(data.frameId);setSpread(true);}return saved;}):Promise.resolve(false)}
      onUnlink={data=>writable&&repository.unsetSpreadFrame?run(()=>repository.unsetSpreadFrame!(projectId,publication.id,continuousEditor.revision,data),zh.publicationContinuousSaved).then(saved=>{if(saved){setPageId(data.targetPageId);setFrameId(data.frameId);}return saved;}):Promise.resolve(false)}/>}
    {guidesEditor&&<PublicationGuidesEditor page={guidesEditor.page} layout={guidesEditor.document.layout} previews={guidesEditor.document.previews} busy={busy} pending={!!pending||!writable} notice={notice} onClose={()=>setGuidesEditor(undefined)} onSave={settings=>writable&&repository.setPageGuides?run(()=>repository.setPageGuides!(projectId,publication.id,guidesEditor.document.revision,{pageId:guidesEditor.page.id,settings}),zh.publicationGuidesSaved).then(saved=>{if(saved)setGuideView({...guideView,grid:true,guides:true});return saved;}):Promise.resolve(false)}/>}
    {masterDialog?.type==='save'&&page&&document&&<PublicationSaveMaster page={page} masters={masterDefinitions} existing={masterDialog.existing} busy={busy} pending={!!pending} notice={notice} onClose={()=>{if(!busy)setMasterDialog(undefined);}} onSave={data=>writable?run(()=>repository.saveMaster!(projectId,publication.id,document.revision,data),zh.publicationMasterSaved):Promise.resolve(false)}/>}
    {masterDialog?.type==='apply'&&page&&document&&<PublicationApplyMaster projectId={projectId} publicationId={publication.id} document={document} masters={masterDefinitions} activePageId={page.id} repository={repository} busy={busy} pending={!!pending} notice={notice} onClose={()=>{if(!busy)setMasterDialog(undefined);}} onApply={data=>writable?run(()=>repository.applyMaster!(projectId,publication.id,document.revision,data),zh.publicationMasterApplied):Promise.resolve(false)}/>}
  </Modal>;
}
