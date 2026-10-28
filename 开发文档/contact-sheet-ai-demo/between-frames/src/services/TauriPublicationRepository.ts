import { invokeCommand as invoke } from './tauriCommand';
import { convertFileSrc } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { PublicationRepository } from '../domain/publication';
import type { PublicationFrameUpdate,PublicationFrameAdd,PublicationFramePhoto,PublicationPageUpdate,PublicationPageAdd,PublicationTextUpdate,PublicationTextGeometry,PublicationReceipt } from '@betweenframes/contracts';
import { PublicationCommandClient } from './PublicationCommandClient';
import { checkPublicationScope,publicationLayoutSchema,publicationPreviewSchema,publicationPreviews,publicationVersionSchema,publicationMasterStatesSchema } from './publicationSchemas';
import { zh } from '../app/copy';
import { parsePublicationPhotoVersions } from './publicationPhotoVersions';
import type { PublicationLayoutFrame,PublicationPhotoVersionCursor } from '../domain/publication';
import type { PublicationSpreadFrame,PublicationUnspreadFrame } from '../domain/publication';
import { applyPublicationMasterSchema,savePublicationMasterSchema,type ApplyPublicationMaster,type SavePublicationMaster } from '@betweenframes/contracts';

const publicationSchema = z.object({
  id: z.string(), project_id: z.string(), pub_type: z.enum(['photobook', 'exhibition', 'portfolio']),
  title: z.string(), state: z.enum(['draft', 'in_review', 'finalized', 'delivered']), created_at: z.string(),
  revision:z.number().int().positive().optional().default(1),
}).transform(value => ({
  id: value.id, projectId: value.project_id, pubType: value.pub_type,
  title: value.title, state: value.state, createdAt: value.created_at,
  revision:value.revision,
}));
const pageSchema = z.object({
  id: z.string(), pageNumber: z.number().int(), widthMm: z.number().finite().positive(), heightMm: z.number().finite().positive(),
  frames: z.array(z.object({
    id: z.string(), xMm: z.number().finite(), yMm: z.number().finite(),
    widthMm: z.number().finite().positive(), heightMm: z.number().finite().positive(),
    fitMode: z.enum(['fit', 'fill', 'stretch']), thumbnailPath: z.string().nullable(),
  }).transform(({ thumbnailPath, ...frame }) => ({
    ...frame, thumbnailSrc: thumbnailPath ? convertFileSrc(thumbnailPath) : undefined,
  }))),
});

export class TauriPublicationRepository implements PublicationRepository {
  private commands=new PublicationCommandClient(request=>invoke('pub_execute',{request}),window.localStorage);
  private revisions=new Map<string,{projectId:string;revision:number}>();
  private remember(receipt:PublicationReceipt){const old=this.revisions.get(receipt.publicationId);this.revisions.set(receipt.publicationId,{projectId:receipt.projectId,revision:Math.max(old?.revision??0,receipt.revision)});}
  pendingCommands(projectId:string){return this.commands.pending(projectId).map(r=>({publicationId:r.publicationId}));}
  async retryPending(projectId:string,publicationId:string){this.remember(await this.commands.retry(projectId,publicationId));}
  async document(projectId:string,publicationId:string){
    const result=z.object({revision:z.number().int().positive(),layout:publicationLayoutSchema,previews:publicationPreviewSchema,masterStates:publicationMasterStatesSchema.optional().default([])}).parse(await invoke('pub_layout',{projectId,publicationId}));
    checkPublicationScope(result.layout,projectId,publicationId);
    if(result.masterStates.some(s=>!result.layout.pages.some(p=>p.id===s.pageId)))throw new Error(zh.publicationUpdated);
    const old=this.revisions.get(publicationId);if(old?.projectId===projectId&&old.revision>result.revision)throw new Error(zh.publicationUpdated);
    this.revisions.set(publicationId,{projectId,revision:result.revision});
    return {...result,previews:publicationPreviews(result.layout,result.previews)};
  }
  async versions(projectId:string,publicationId:string,before?:number){
    const result=z.array(publicationVersionSchema).max(30).parse(await invoke('pub_versions',{projectId,publicationId,before:before??null}));
    if(new Set(result.map(v=>v.id)).size!==result.length||result.some((v,i)=>v.publicationId!==publicationId||(before!==undefined&&v.revision>=before)||(i>0&&v.revision>=result[i-1].revision)))throw new Error(zh.publicationHistoryFailed);return result;
  }
  async version(projectId:string,publicationId:string,versionId:string){
    const result=z.object({version:publicationVersionSchema,layout:publicationLayoutSchema,previews:publicationPreviewSchema}).parse(await invoke('pub_version',{projectId,publicationId,versionId}));
    checkPublicationScope(result.layout,projectId,publicationId);if(result.version.id!==versionId||result.version.publicationId!==publicationId)throw new Error(zh.publicationHistoryFailed);
    return {...result,previews:publicationPreviews(result.layout,result.previews)};
  }
  async editFrame(projectId:string,publicationId:string,revision:number,frame:PublicationFrameUpdate){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UpdateFrame',data:frame}]));}
  async arrangeObjects(projectId:string,publicationId:string,revision:number,frames:PublicationFrameUpdate[],texts:PublicationTextGeometry[]){
    const ids=[...frames.map(frame=>frame.frameId),...texts.map(text=>text.textId)];if(!ids.length||ids.length>100||new Set(ids).size!==ids.length)throw new Error(zh.publicationArrangeInvalid);
    this.remember(await this.commands.execute(projectId,publicationId,revision,[...frames.map(data=>({type:'UpdateFrame' as const,data})),...texts.map(data=>({type:'UpdateTextGeometry' as const,data}))]));
  }
  async arrangeFrames(projectId:string,publicationId:string,revision:number,updates:PublicationFrameUpdate[],additions:PublicationFrameAdd[],assignments:PublicationFramePhoto[]=[]){this.remember(await this.commands.execute(projectId,publicationId,revision,[...updates.map(data=>({type:'UpdateFrame' as const,data})),...additions.map(data=>({type:'AddFrame' as const,data})),...assignments.map(data=>({type:'SetFramePhoto' as const,data}))]));}
  async replaceFramePhoto(projectId:string,publicationId:string,revision:number,frameId:string,photoId:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'SetFramePhoto',data:{frameId,photoId,renditionId:null}}]));}
  async photoVersions(projectId:string,publicationId:string,revision:number,frame:PublicationLayoutFrame,cursor?:PublicationPhotoVersionCursor){return parsePublicationPhotoVersions(await invoke('pub_photo_versions',{projectId,publicationId,frameId:frame.id,baseRevision:revision,cursor:cursor??null}),revision,frame,cursor);}
  async removeFrame(projectId:string,publicationId:string,revision:number,frameId:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'RemoveFrame',data:{frameId}}]));}
  async removeObjects(projectId:string,publicationId:string,revision:number,objects:import('../domain/publication').PublicationObjectReference[]){
    if(!objects.length||objects.length>100||new Set(objects.map(object=>object.id)).size!==objects.length||objects.some(object=>!['frame','text'].includes(object.kind)))throw new Error(zh.publicationArrangeInvalid);
    this.remember(await this.commands.execute(projectId,publicationId,revision,objects.map(object=>object.kind==='frame'?{type:'RemoveFrame' as const,data:{frameId:object.id}}:{type:'RemoveText' as const,data:{textId:object.id}})));
  }
  async setPageGuides(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').SetPublicationPageGuides){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'SetPageGuides',data}]));}
  async setLayers(projectId:string,publicationId:string,revision:number,data:import('../domain/publication').PublicationLayers){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'SetPublicationLayers',data}]));}
  async setSpreadFrame(projectId:string,publicationId:string,revision:number,data:PublicationSpreadFrame){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'SetSpreadFrame',data}]));}
  async unsetSpreadFrame(projectId:string,publicationId:string,revision:number,data:PublicationUnspreadFrame){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UnsetSpreadFrame',data}]));}
  async saveMaster(projectId:string,publicationId:string,revision:number,data:SavePublicationMaster){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'SaveMaster',data:savePublicationMasterSchema.parse(data)}]));}
  async applyMaster(projectId:string,publicationId:string,revision:number,data:ApplyPublicationMaster){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'ApplyMaster',data:applyPublicationMasterSchema.parse(data)}]));}
  async previewMaster(projectId:string,publicationId:string,revision:number,data:ApplyPublicationMaster){
    const request=applyPublicationMasterSchema.parse(data);
    const result=z.object({revision:z.number().int().positive(),layout:publicationLayoutSchema,previews:publicationPreviewSchema,overriddenPageIds:z.array(z.string()).max(100)}).parse(await invoke('pub_master_preview',{projectId,publicationId,baseRevision:revision,data:request}));
    checkPublicationScope(result.layout,projectId,publicationId);
    if(result.revision!==revision||result.overriddenPageIds.some(id=>!request.pageIds.includes(id))||new Set(result.overriddenPageIds).size!==result.overriddenPageIds.length||request.pageIds.some(id=>!result.layout.pages.some(p=>p.id===id)))throw new Error(zh.publicationUpdated);
    return {...result,previews:publicationPreviews(result.layout,result.previews)};
  }
  async editPage(projectId:string,publicationId:string,revision:number,page:PublicationPageUpdate){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UpdatePage',data:page}]));}
  async editPages(projectId:string,publicationId:string,revision:number,pages:PublicationPageUpdate[]){this.remember(await this.commands.execute(projectId,publicationId,revision,pages.map(data=>({type:'UpdatePage' as const,data}))));}
  async addPages(projectId:string,publicationId:string,revision:number,pages:PublicationPageAdd[]){this.remember(await this.commands.execute(projectId,publicationId,revision,pages.map(data=>({type:'AddPage' as const,data}))));}
  async addPage(projectId:string,publicationId:string,revision:number,page:PublicationPageAdd){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'AddPage',data:page}]));}
  async reorderPages(projectId:string,publicationId:string,revision:number,pageIds:string[],chapterChangesConfirmed?:boolean){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'ReorderPages',data:{pageIds,...(chapterChangesConfirmed!==undefined?{chapterChangesConfirmed}:{})}}]));}
  async setCover(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').SetPublicationCover){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'SetCover',data}]));}
  async setChapters(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').SetPublicationChapters){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'SetChapters',data}]));}
  async createChapter(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').CreatePublicationChapter){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'CreateChapter',data}]));}
  async removePages(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').RemovePublicationPages){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'RemovePages',data}]));}
  async removePage(projectId:string,publicationId:string,revision:number,pageId:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'RemovePage',data:{pageId}}]));}
  async addText(projectId:string,publicationId:string,revision:number,text:PublicationTextUpdate){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'AddText',data:text}]));}
  async editTextGeometry(projectId:string,publicationId:string,revision:number,data:PublicationTextGeometry){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UpdateTextGeometry',data}]));}
  async editText(projectId:string,publicationId:string,revision:number,text:PublicationTextUpdate){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UpdateText',data:text}]));}
  async removeText(projectId:string,publicationId:string,revision:number,textId:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'RemoveText',data:{textId}}]));}
  async saveVersion(projectId:string,publicationId:string,revision:number,label:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'CaptureVersion',data:{label}}]));}
  async restoreVersion(projectId:string,publicationId:string,revision:number,versionId:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'RestoreVersion',data:{versionId}}]));}
  async pages(projectId: string, publicationId: string) {
    return z.array(pageSchema).parse(await invoke('pub_pages', { projectId, publicationId }));
  }
  async list(projectId: string) {
    const list=z.array(publicationSchema).parse(await invoke('pub_list',{projectId}));
    if(list.some(pub=>pub.projectId!==projectId))throw new Error(zh.publicationScopeMismatch);
    for(const pub of list){const old=this.revisions.get(pub.id);this.revisions.set(pub.id,{projectId,revision:Math.max(old?.revision??0,pub.revision)});}return list;
  }
  async create(projectId: string, title: string, pubType = 'photobook') {
    if(this.commands.pending(projectId).some(r=>r.operations.some(op=>op.action.type==='Create')))throw new Error(zh.publicationPending);
    const type=z.enum(['photobook','exhibition','portfolio']).parse(pubType),id=crypto.randomUUID();
    const receipt=await this.commands.execute(projectId,id,0,[{type:'Create',data:{title,pubType:type}}]);this.remember(receipt);return publicationSchema.parse(receipt.results[0]);
  }
  async generate(projectId: string, publicationId: string, photosPerPage = 1, replaceExisting = false) {
    let current=this.revisions.get(publicationId);if(current?.projectId!==projectId){await this.list(projectId);current=this.revisions.get(publicationId);}
    if(!current||current.projectId!==projectId)throw new Error(zh.publicationUpdated);
    const perPage=z.union([z.literal(1),z.literal(2),z.literal(4)]).parse(photosPerPage);
    const receipt=await this.commands.execute(projectId,publicationId,current.revision,[{type:'Generate',data:{photosPerPage:perPage,replaceExisting}}]);this.remember(receipt);return z.number().int().positive().parse(receipt.results[0]);
  }
}
