export interface PublicationInfo {
  id: string; projectId: string; pubType: string; title: string; state: string; createdAt: string;
  revision?:number;
}
// 类型与开本必填：后端一直支持展览/作品集和自定义尺寸，缺的是创建时那份明确的选择。
export type PublicationKind = 'photobook' | 'exhibition' | 'portfolio';
export interface PublicationSpec { pubType: PublicationKind; widthMm: number; heightMm: number }
export interface PublicationRepository {
  list(projectId: string): Promise<PublicationInfo[]>;
  create(projectId: string, title: string, spec: PublicationSpec): Promise<PublicationInfo>;
  generate(projectId: string, publicationId: string, photosPerPage?: number): Promise<number>;
  pages(projectId: string, publicationId: string): Promise<PublicationPage[]>;
  document?(projectId:string,publicationId:string):Promise<PublicationDocument>;
  versions?(projectId:string,publicationId:string,before?:number):Promise<PublicationVersionInfo[]>;
  version?(projectId:string,publicationId:string,versionId:string):Promise<PublicationVersion>;
  editFrame?(projectId:string,publicationId:string,revision:number,frame:PublicationFrameUpdate):Promise<void>;
  editPage?(projectId:string,publicationId:string,revision:number,page:PublicationPageUpdate):Promise<void>;
  // 一次命令改完整本纸张：与逐页改动同一条幂等通道，要么全部生效要么整笔回滚。
  editPages?(projectId:string,publicationId:string,revision:number,pages:PublicationPageUpdate[]):Promise<void>;
  // 审校状态单独一条命令：不在草稿状态时版面写命令会被服务端拒，这条是唯一还能走的写命令。
  changeState?(projectId:string,publicationId:string,revision:number,state:PublicationState):Promise<void>;
  // 展览规格（观看距离与悬挂中心线）：摄影书没有这件事，后端会直接拒。
  editSpec?(projectId:string,publicationId:string,revision:number,spec:PublicationSpecUpdate):Promise<void>;
  addText?(projectId:string,publicationId:string,revision:number,text:PublicationTextNew):Promise<void>;
  editText?(projectId:string,publicationId:string,revision:number,text:PublicationTextUpdate):Promise<void>;
  removeText?(projectId:string,publicationId:string,revision:number,textId:string):Promise<void>;
  saveVersion?(projectId:string,publicationId:string,revision:number,label:string):Promise<void>;
  restoreVersion?(projectId:string,publicationId:string,revision:number,versionId:string):Promise<void>;
  pendingCommands?(projectId:string):Array<{publicationId:string}>;
  retryPending?(projectId:string,publicationId:string):Promise<void>;
  preflight?(projectId:string,publicationId:string,minPpi?:number):Promise<PreflightReport>;
  // 交付出口：把整本版面导成作品清单落盘。用户在系统保存对话框里取消时返回 null，那不是失败。
  exportWorklist?(projectId:string,publicationId:string):Promise<PublicationExportReceipt|null>;
  // 协作审稿：意见锚在"这一本 + 提它时那一版"。未解决的意见会拦下定稿，处理必须逐条明确表态。
  review?(projectId:string,publicationId:string):Promise<PublicationReview>;
  addReview?(projectId:string,publicationId:string,body:string):Promise<PublicationReview>;
  resolveReview?(projectId:string,publicationId:string,threadId:string,resolved:boolean):Promise<PublicationReview>;
}
export interface PublicationReviewComment { id:number; body:string; author:string; createdAt:string }
export interface PublicationReviewThread {
  id:string; revision:number; state:'open'|'resolved'; comments:PublicationReviewComment[];
}
export interface PublicationReview {
  publicationId:string; revision:number; open:number; threads:PublicationReviewThread[];
}
export interface PublicationExportReceipt { path:string; sha256:string; bytes:number }
export type PreflightSeverity = 'blocking' | 'warning' | 'info';
export interface PreflightIssue {
  code: string; severity: PreflightSeverity;
  pageNumber: number | null; frameId: string | null; textId: string | null;
  measured: number | null; required: number | null;
}
export interface PreflightReport {
  minPpi: number; pages: number; frames: number; texts: number;
  blocking: number; warnings: number; infos: number; truncated: boolean;
  issues: PreflightIssue[];
}
export type { PublicationFrameUpdate, PublicationPageUpdate, PublicationSpecUpdate, PublicationState, PublicationTextNew, PublicationTextUpdate } from '@betweenframes/contracts';
import type { PublicationFrameUpdate, PublicationPageUpdate, PublicationSpecUpdate, PublicationState, PublicationTextNew, PublicationTextUpdate } from '@betweenframes/contracts';
export interface PublicationSource {
  assetId:string;sourceSha256:string;width:number;height:number;sourceProfileDigest:string|null;
  renditionId:string|null;variantId:string|null;renditionSha256:string|null;renditionWidth:number|null;renditionHeight:number|null;bitDepth:number|null;iccDigest:string|null;previewSha256:string|null;
}
export interface PublicationLayoutFrame {
  id:string;xMm:number;yMm:number;widthMm:number;heightMm:number;fitMode:'fit'|'fill'|'stretch';assetId:string|null;renditionId:string|null;cropX:number;cropY:number;cropZoom:number;source:PublicationSource|null;
}
export interface PublicationLayout {
  schemaVersion:1;publication:{id:string;projectId:string;title:string;pubType:string;specJson:string;state:string};
  pages:Array<{id:string;pageNumber:number;side:string;widthMm:number;heightMm:number;marginMm:number;bleedMm:number;layoutJson:string|null;frames:PublicationLayoutFrame[];texts:Array<{id:string;xMm:number;yMm:number;widthMm:number;heightMm:number;contentJson:string;styleJson:string|null}>}>;
}
export interface PublicationPreview {frameId:string;thumbnailSrc?:string}
export interface PublicationDocument {revision:number;layout:PublicationLayout;previews:PublicationPreview[]}
export interface PublicationVersionInfo {id:string;publicationId:string;revision:number;label:string;kind:'working'|'saved';snapshotSha256:string;createdAt:string}
export interface PublicationVersion {version:PublicationVersionInfo;layout:PublicationLayout;previews:PublicationPreview[]}
export interface PublicationPage {
  id: string; pageNumber: number; widthMm: number; heightMm: number;
  frames: Array<{
    id: string; xMm: number; yMm: number; widthMm: number; heightMm: number;
    fitMode: 'fit' | 'fill' | 'stretch'; thumbnailSrc?: string;
  }>;
}
