import { invokeCommand as invoke } from './tauriCommand';
import { convertFileSrc } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { PublicationRepository } from '../domain/publication';
import type { PublicationFrameUpdate,PublicationReceipt } from '@betweenframes/contracts';
import { PublicationCommandClient } from './PublicationCommandClient';
import { checkPublicationScope,publicationLayoutSchema,publicationPreviewSchema,publicationPreviews,publicationVersionSchema } from './publicationSchemas';
import { zh } from '../app/copy';

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
    const result=z.object({revision:z.number().int().positive(),layout:publicationLayoutSchema,previews:publicationPreviewSchema}).parse(await invoke('pub_layout',{projectId,publicationId}));
    checkPublicationScope(result.layout,projectId,publicationId);
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
  async generate(projectId: string, publicationId: string, photosPerPage = 1) {
    let current=this.revisions.get(publicationId);if(current?.projectId!==projectId){await this.list(projectId);current=this.revisions.get(publicationId);}
    if(!current||current.projectId!==projectId)throw new Error(zh.publicationUpdated);
    const perPage=z.union([z.literal(1),z.literal(2),z.literal(4)]).parse(photosPerPage);
    const receipt=await this.commands.execute(projectId,publicationId,current.revision,[{type:'Generate',data:{photosPerPage:perPage}}]);this.remember(receipt);return z.number().int().positive().parse(receipt.results[0]);
  }
}
