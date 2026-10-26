export interface PublicationInfo {
  id: string; projectId: string; pubType: string; title: string; state: string; createdAt: string;
  revision?:number;
}
export interface PublicationRepository {
  list(projectId: string): Promise<PublicationInfo[]>;
  create(projectId: string, title: string, pubType?: string): Promise<PublicationInfo>;
  generate(projectId: string, publicationId: string, photosPerPage?: number): Promise<number>;
  pages(projectId: string, publicationId: string): Promise<PublicationPage[]>;
  document?(projectId:string,publicationId:string):Promise<PublicationDocument>;
  versions?(projectId:string,publicationId:string,before?:number):Promise<PublicationVersionInfo[]>;
  version?(projectId:string,publicationId:string,versionId:string):Promise<PublicationVersion>;
  editFrame?(projectId:string,publicationId:string,revision:number,frame:PublicationFrameUpdate):Promise<void>;
  saveVersion?(projectId:string,publicationId:string,revision:number,label:string):Promise<void>;
  restoreVersion?(projectId:string,publicationId:string,revision:number,versionId:string):Promise<void>;
  pendingCommands?(projectId:string):Array<{publicationId:string}>;
  retryPending?(projectId:string,publicationId:string):Promise<void>;
}
export type { PublicationFrameUpdate } from '@betweenframes/contracts';
import type { PublicationFrameUpdate } from '@betweenframes/contracts';
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
