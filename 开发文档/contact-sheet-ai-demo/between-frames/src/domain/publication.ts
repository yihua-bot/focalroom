export interface PublicationInfo {
  id: string; projectId: string; pubType: string; title: string; state: string; createdAt: string;
  revision?:number;
}
export interface PublicationRepository {
  list(projectId: string): Promise<PublicationInfo[]>;
  create(projectId: string, title: string, pubType?: string): Promise<PublicationInfo>;
  generate(projectId: string, publicationId: string, photosPerPage?: number, replaceExisting?: boolean): Promise<number>;
  pages(projectId: string, publicationId: string): Promise<PublicationPage[]>;
  document?(projectId:string,publicationId:string):Promise<PublicationDocument>;
  versions?(projectId:string,publicationId:string,before?:number):Promise<PublicationVersionInfo[]>;
  version?(projectId:string,publicationId:string,versionId:string):Promise<PublicationVersion>;
  editFrame?(projectId:string,publicationId:string,revision:number,frame:PublicationFrameUpdate):Promise<void>;
  arrangeObjects?(projectId:string,publicationId:string,revision:number,frames:PublicationFrameUpdate[],texts:PublicationTextGeometry[]):Promise<void>;
  arrangeFrames?(projectId:string,publicationId:string,revision:number,updates:PublicationFrameUpdate[],additions:PublicationFrameAdd[],assignments?:PublicationFramePhoto[]):Promise<void>;
  replaceFramePhoto?(projectId:string,publicationId:string,revision:number,frameId:string,photoId:string):Promise<void>;
  photoVersions?(projectId:string,publicationId:string,revision:number,frame:PublicationLayoutFrame,cursor?:PublicationPhotoVersionCursor):Promise<PublicationPhotoVersions>;
  removeFrame?(projectId:string,publicationId:string,revision:number,frameId:string):Promise<void>;
  removeObjects?(projectId:string,publicationId:string,revision:number,objects:PublicationObjectReference[]):Promise<void>;
  setPageGuides?(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').SetPublicationPageGuides):Promise<void>;
  setLayers?(projectId:string,publicationId:string,revision:number,layers:PublicationLayers):Promise<void>;
  setSpreadFrame?(projectId:string,publicationId:string,revision:number,data:PublicationSpreadFrame):Promise<void>;
  unsetSpreadFrame?(projectId:string,publicationId:string,revision:number,data:PublicationUnspreadFrame):Promise<void>;
  saveMaster?(projectId:string,publicationId:string,revision:number,data:SavePublicationMaster):Promise<void>;
  previewMaster?(projectId:string,publicationId:string,revision:number,data:ApplyPublicationMaster):Promise<PublicationMasterPreview>;
  applyMaster?(projectId:string,publicationId:string,revision:number,data:ApplyPublicationMaster):Promise<void>;
  editPage?(projectId:string,publicationId:string,revision:number,page:PublicationPageUpdate):Promise<void>;
  editPages?(projectId:string,publicationId:string,revision:number,pages:PublicationPageUpdate[]):Promise<void>;
  addPages?(projectId:string,publicationId:string,revision:number,pages:PublicationPageAdd[]):Promise<void>;
  addPage?(projectId:string,publicationId:string,revision:number,page:PublicationPageAdd):Promise<void>;
  reorderPages?(projectId:string,publicationId:string,revision:number,pageIds:string[],chapterChangesConfirmed?:boolean):Promise<void>;
  setCover?(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').SetPublicationCover):Promise<void>;
  setChapters?(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').SetPublicationChapters):Promise<void>;
  createChapter?(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').CreatePublicationChapter):Promise<void>;
  removePages?(projectId:string,publicationId:string,revision:number,data:import('@betweenframes/contracts').RemovePublicationPages):Promise<void>;
  removePage?(projectId:string,publicationId:string,revision:number,pageId:string):Promise<void>;
  addText?(projectId:string,publicationId:string,revision:number,text:PublicationTextUpdate):Promise<void>;
  editTextGeometry?(projectId:string,publicationId:string,revision:number,text:PublicationTextGeometry):Promise<void>;
  editText?(projectId:string,publicationId:string,revision:number,text:PublicationTextUpdate):Promise<void>;
  removeText?(projectId:string,publicationId:string,revision:number,textId:string):Promise<void>;
  saveVersion?(projectId:string,publicationId:string,revision:number,label:string):Promise<void>;
  restoreVersion?(projectId:string,publicationId:string,revision:number,versionId:string):Promise<void>;
  pendingCommands?(projectId:string):Array<{publicationId:string}>;
  retryPending?(projectId:string,publicationId:string):Promise<void>;
}
export type { PublicationFrameUpdate,PublicationFrameAdd,PublicationFramePhoto,PublicationPageUpdate,PublicationPageAdd,PublicationTextUpdate,PublicationTextGeometry } from '@betweenframes/contracts';
export type { PublicationSpreadFrame,PublicationUnspreadFrame,PublicationFrameGeometry } from '@betweenframes/contracts';
import type { PublicationSpreadFrame,PublicationUnspreadFrame } from '@betweenframes/contracts';
import type { PublicationFrameUpdate,PublicationFrameAdd,PublicationFramePhoto,PublicationPageUpdate,PublicationPageAdd,PublicationTextUpdate,PublicationTextGeometry } from '@betweenframes/contracts';
export type { PublicationMaster,SavePublicationMaster,ApplyPublicationMaster,PublicationMasterLink } from '@betweenframes/contracts';
import type { SavePublicationMaster,ApplyPublicationMaster } from '@betweenframes/contracts';
export interface PublicationMasterPreview extends PublicationDocument {overriddenPageIds:string[]}
export interface PublicationSource {
  assetId:string;sourceSha256:string;width:number;height:number;sourceProfileDigest:string|null;
  renditionId:string|null;variantId:string|null;renditionSha256:string|null;renditionWidth:number|null;renditionHeight:number|null;bitDepth:number|null;iccDigest:string|null;previewSha256:string|null;
}
export interface PublicationObjectReference {kind:'frame'|'text';id:string}
export interface PublicationLayoutFrame {
  id:string;xMm:number;yMm:number;widthMm:number;heightMm:number;fitMode:'fit'|'fill'|'stretch';assetId:string|null;renditionId:string|null;cropX:number;cropY:number;cropZoom:number;source:PublicationSource|null;
}
export interface PublicationLayout {
  schemaVersion:1|2|3;publication:{id:string;projectId:string;title:string;pubType:string;specJson:string;state:string};
  pages:Array<{id:string;pageNumber:number;side:string;widthMm:number;heightMm:number;marginMm:number;bleedMm:number;layoutJson:string|null;frames:PublicationLayoutFrame[];texts:Array<{id:string;xMm:number;yMm:number;widthMm:number;heightMm:number;contentJson:string;styleJson:string|null}>}>;
}
export interface PublicationPreview {frameId:string;thumbnailSrc?:string;photoId?:string}
export interface PublicationPhotoVersionCursor {createdAt:string;id:string}
export interface PublicationPhotoVersion {renditionId:string|null;variantName:string|null;externalSoftware:string|null;createdAt:string|null;source:PublicationSource;thumbnailSrc?:string}
export interface PublicationPhotoVersions {revision:number;frameId:string;assetId:string;original:PublicationPhotoVersion;current:PublicationPhotoVersion;choices:PublicationPhotoVersion[];next:PublicationPhotoVersionCursor|null}
export interface PublicationMasterState {pageId:string;masterId:string;appliedRevision:number;currentRevision:number|null;locallyChanged:boolean}
export interface PublicationDocument {revision:number;layout:PublicationLayout;previews:PublicationPreview[];masterStates?:PublicationMasterState[]}
export interface PublicationVersionInfo {id:string;publicationId:string;revision:number;label:string;kind:'working'|'saved';snapshotSha256:string;createdAt:string}
export interface PublicationVersion {version:PublicationVersionInfo;layout:PublicationLayout;previews:PublicationPreview[]}
export interface PublicationPage {
  id: string; pageNumber: number; widthMm: number; heightMm: number;
  frames: Array<{
    id: string; xMm: number; yMm: number; widthMm: number; heightMm: number;
    fitMode: 'fit' | 'fill' | 'stretch'; thumbnailSrc?: string;
  }>;
}

export type { PublicationLayers,PublicationLayerNode } from '@betweenframes/contracts';
import type { PublicationLayers } from '@betweenframes/contracts';
