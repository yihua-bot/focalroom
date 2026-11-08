import { invokeCommand as invoke } from './tauriCommand';
import { convertFileSrc } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { PublicationRepository, PublicationSpec } from '../domain/publication';
import type { PublicationFrameUpdate,PublicationPageUpdate,PublicationReceipt,PublicationSpecUpdate,PublicationState,PublicationTextNew,PublicationTextUpdate } from '@betweenframes/contracts';
import { PublicationCommandClient } from './PublicationCommandClient';
import { checkPreflight,checkPublicationScope,preflightReportSchema,publicationLayoutSchema,publicationPreviewSchema,publicationPreviews,publicationVersionSchema } from './publicationSchemas';
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

const reviewCommentSchema = z.object({
  id: z.number().int().nonnegative(), body: z.string().min(1).max(5000),
  author: z.string().min(1).max(160), createdAt: z.string().min(1).max(80),
});
const reviewThreadSchema = z.object({
  id: z.string().min(1), revision: z.number().int().nonnegative(),
  state: z.enum(['open', 'resolved']), comments: z.array(reviewCommentSchema).min(1).max(200),
});
const reviewSchema = z.object({
  publicationId: z.string().min(1), revision: z.number().int().nonnegative(),
  open: z.number().int().nonnegative(), threads: z.array(reviewThreadSchema).max(60),
});
const reviewBody = z.string().trim().min(1).max(5000);

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
  async editPage(projectId:string,publicationId:string,revision:number,page:PublicationPageUpdate){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UpdatePage',data:page}]));}
  // 整本一次改完：一页一条操作，仍是一条幂等命令，所以服务端要么全改要么整笔回滚，不会留下半本新纸张。
  async editPages(projectId:string,publicationId:string,revision:number,pages:PublicationPageUpdate[]){this.remember(await this.commands.execute(projectId,publicationId,revision,pages.map(page=>({type:'UpdatePage' as const,data:page}))))}
  // 状态改动单独成条：契约不允许它和版面改动同批，服务端也只让这一条命令绕过"必须是草稿"的守卫。
  async changeState(projectId:string,publicationId:string,revision:number,state:PublicationState){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UpdateState',data:{state}}]))}
  async editSpec(projectId:string,publicationId:string,revision:number,spec:PublicationSpecUpdate){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UpdateSpec',data:spec}]))}
  async addText(projectId:string,publicationId:string,revision:number,text:PublicationTextNew){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'AddText',data:text}]));}
  async editText(projectId:string,publicationId:string,revision:number,text:PublicationTextUpdate){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'UpdateText',data:text}]));}
  async removeText(projectId:string,publicationId:string,revision:number,textId:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'DeleteText',data:{textId}}]));}
  async saveVersion(projectId:string,publicationId:string,revision:number,label:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'CaptureVersion',data:{label}}]));}
  async restoreVersion(projectId:string,publicationId:string,revision:number,versionId:string){this.remember(await this.commands.execute(projectId,publicationId,revision,[{type:'RestoreVersion',data:{versionId}}]));}
  async preflight(projectId:string,publicationId:string,minPpi?:number){
    const result=preflightReportSchema.parse(await invoke('pub_preflight',{projectId,publicationId,minPpi:minPpi??null}));
    checkPreflight(result);return result;
  }
  // 用户在保存对话框里取消时后端回 null：那是他改主意的地方，不能报成导出失败。
  async exportWorklist(projectId:string,publicationId:string){
    return z.object({path:z.string().min(1),sha256:z.string().length(64),bytes:z.number().int().nonnegative()}).nullable().parse(await invoke('pub_export',{projectId,publicationId}));
  }
  // 审稿回执是"谁说了什么、处理了没有"的凭据：条数与逐条对不上、意见不属于这本、版本号比当前还新，都不许上屏。
  private async readReview(command:'pub_review'|'pub_review_write',projectId:string,publicationId:string,extra:Record<string,unknown>={}){
    const result=reviewSchema.parse(await invoke(command,{projectId,publicationId,...extra}));
    if(result.publicationId!==publicationId)throw new Error(zh.publicationScopeMismatch);
    const open=result.threads.filter(thread=>thread.state==='open').length;
    if(new Set(result.threads.map(thread=>thread.id)).size!==result.threads.length
      ||open!==result.open
      ||result.threads.some(thread=>thread.revision>result.revision)
      ||result.threads.some(thread=>thread.comments.some(comment=>comment.body.trim()==='')))throw new Error(zh.publicationReviewFailed);
    return result;
  }
  async review(projectId:string,publicationId:string){return this.readReview('pub_review',projectId,publicationId);}
  async addReview(projectId:string,publicationId:string,body:string){
    return this.readReview('pub_review_write',projectId,publicationId,{request:{action:'add',data:{body:reviewBody.parse(body)}}});
  }
  async resolveReview(projectId:string,publicationId:string,threadId:string,resolved:boolean){
    return this.readReview('pub_review_write',projectId,publicationId,{request:{action:'resolve',data:{threadId:z.string().min(1).max(160).parse(threadId),resolved}}});
  }
  async pages(projectId: string, publicationId: string) {
    return z.array(pageSchema).parse(await invoke('pub_pages', { projectId, publicationId }));
  }
  async list(projectId: string) {
    const list=z.array(publicationSchema).parse(await invoke('pub_list',{projectId}));
    if(list.some(pub=>pub.projectId!==projectId))throw new Error(zh.publicationScopeMismatch);
    for(const pub of list){const old=this.revisions.get(pub.id);this.revisions.set(pub.id,{projectId,revision:Math.max(old?.revision??0,pub.revision)});}return list;
  }
  async create(projectId: string, title: string, spec: PublicationSpec) {
    if(this.commands.pending(projectId).some(r=>r.operations.some(op=>op.action.type==='Create')))throw new Error(zh.publicationPending);
    // 类型与开本原样交给契约校验：陌生类型、越界尺寸都出不了门，也不在这里补一个默认值。
    const id=crypto.randomUUID();
    const receipt=await this.commands.execute(projectId,id,0,[{type:'Create',data:{title,pubType:spec.pubType,widthMm:spec.widthMm,heightMm:spec.heightMm}}]);this.remember(receipt);return publicationSchema.parse(receipt.results[0]);
  }
  async generate(projectId: string, publicationId: string, photosPerPage = 1) {
    let current=this.revisions.get(publicationId);if(current?.projectId!==projectId){await this.list(projectId);current=this.revisions.get(publicationId);}
    if(!current||current.projectId!==projectId)throw new Error(zh.publicationUpdated);
    const perPage=z.union([z.literal(1),z.literal(2),z.literal(4)]).parse(photosPerPage);
    const receipt=await this.commands.execute(projectId,publicationId,current.revision,[{type:'Generate',data:{photosPerPage:perPage}}]);this.remember(receipt);return z.number().int().positive().parse(receipt.results[0]);
  }
}
