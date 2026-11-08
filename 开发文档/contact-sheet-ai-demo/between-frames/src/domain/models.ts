import type { DraftRepository } from './agent';
export type PhotoStatus = 'undecided' | 'keep' | 'reject';
export type PhotoFilter = 'all' | PhotoStatus;
export type WorkspaceView = 'grid' | 'sequence' | 'canvas' | 'publication';
export type InspectorPanel = 'photo' | 'memory';
export interface InspectionImage { src: string; kind: 'original' | 'derived_preview' | 'embedded_preview' | 'display_managed' }

export interface Photo {
  id: string;
  index: number;
  name: string;
  src: string;
  status: PhotoStatus;
  note: string;
  rating: number;
  bookMark?: 'spread' | 'full' | null;
  previewSrc?: string;
  originalPath?: string;
  isMissing?: boolean;
  width?: number;
  height?: number;
  loadOriginal?: () => Promise<InspectionImage>;
  /** 色彩管理开启后 100% 检视用的全分辨率显示器渲染；没有这条通道（网页版）时不提供。 */
  loadManagedOriginal?: () => Promise<InspectionImage>;
}

export interface PreviewEvent {
  id: string;
  type: string;
  photoId?: string;
  description: string;
  at: string;
  /** 操作者：user=人工命令；system=导入等系统动作；Agent 草案经人工采纳后记 user，类型以 draft_ 前缀区分。 */
  actor: 'user' | 'system';
  previous?: unknown;
  next?: unknown;
  context?: unknown;
}

export interface Project {
  id: string;
  archived?: boolean;
  title: string;
  brief: string;
  tags?: string[];
  photos: Photo[];
  sequence: string[];
  events: PreviewEvent[];
  revision?: number;
  updatedAt?: string;
  history?: { undoGroupId: string | null; redoGroupId: string | null; undoCount: number; redoCount: number };
  versions?: SequenceVersion[];
}

export interface SequenceVersion {
  id: string;
  label: string;
  basedOnRevision: number;
  createdAt: string;
  photoIds: string[];
  photos: { photoId: string; status: PhotoStatus; rating: number; note: string }[];
}

export interface ProjectRepository {
  readonly mode?: 'preview' | 'desktop';
  list(archived?: boolean): Promise<Project[]>;
  get(id: string): Promise<Project>;
  create(title: string, brief: string): Promise<Project>;
  edit(id: string, title: string, brief: string): Promise<Project>;
  decide(id: string, photoId: string, status: PhotoStatus): Promise<Project>;
  decideMany(id: string, photoIds: string[], status: PhotoStatus): Promise<Project>;
  rate(id: string, photoIds: string[], rating: number): Promise<Project>;
  setBookMark(projectId: string, photoId: string, mark: 'spread' | 'full' | null): Promise<Project>;
  note(id: string, photoId: string, text: string): Promise<Project>;
  reorder(id: string, photoIds: string[]): Promise<Project>;
  compare(id: string, leftId: string, rightId: string, preferred: string | null, reason: string): Promise<Project>;
  undo(id: string): Promise<Project>;
  redo(id: string): Promise<Project>;
  saveVersion(id: string, label: string): Promise<Project>;
  restoreVersion(id: string, versionId: string): Promise<Project>;
  setTags(projectId: string, tags: string[]): Promise<void>;
}

/** 同一来源路径、内容指纹已不同：外部编辑器就地保存回原文件就是这个形状。 */
export interface ImportChange {
  photoId: string;
  path: string;
  filename: string;
  previousSha256: string;
  incomingSha256: string;
  previousWidth: number;
  previousHeight: number;
  previousSize: number;
  incomingWidth: number;
  incomingHeight: number;
  incomingSize: number;
}

/** 外部软件写在原图旁边的判断：本地能落地的与只能报出来的分得很清楚。 */
export interface SidecarProposal {
  photoId: string;
  filename: string;
  sidecarPath: string;
  sidecarRating: number | null;
  localRating: number;
  appliesRating: boolean;
  sidecarPick: boolean | null;
  localStatus: 'undecided' | 'keep' | 'reject';
  appliesStatus: boolean;
  colorLabel: number | null;
  ambiguousRating: boolean;
  /** 一键回填够得着的那部分：本地这条还空着，填进去不会盖掉人工判断。 */
  batchSafe: boolean;
}

/** 一键回填的回执：落成几条、跳过几条；跳过不是失败，是"这期间你自己改过了"。 */
export interface SidecarBatchOutcome {
  applied: number;
  skipped: number;
}

export interface ImportJob {
  jobId: string;
  projectId: string;
  status: 'running' | 'completed' | 'cancelled';
  completed: number;
  total: number;
  imported: number;
  skipped: number;
  changes: ImportChange[];
  failures: { path: string; code: string; message: string }[];
}

export interface ExportReceipt { path: string; sha256: string; bytes: number }

/** 一次坏预览修复的实话：结局沿用后端的原因码，界面只负责把它说成人话。 */
export type PreviewRepairOutcome = { photoId: string; outcome: string; rebuilt: string[]; detail: string };
/** 整项目方案口径的欠账：这一轮补了多少、还差多少，按库里的事实报，不按"最近几份"截断。 */
export type ProjectDraftRecheck = {
  /** 项目里一共多少份方案。 */
  drafts: number;
  /** 这一轮点到多少份；比总数少就说明还有没点到的。 */
  draftsScanned: number;
  /** 点名时还欠一次重算的份数。 */
  recheckOwed: number;
  recheckedDrafts: number;
  recheckFailed: number;
  /** 这一轮之后还欠多少份：没算成的仍旧欠着。 */
  recheckRemaining: number;
};
/** 整项目派生预览的点名单：跨页统计加逐张原因。修复之后同一份结构里多出修好的计数与口径欠账。 */
export type ProjectPreviewAudit = {
  total: number;
  scanned: number;
  healthy: number;
  needsRepair: number;
  blocked: number;
  reasons: Record<string, number>;
  /** 名单要落到具体哪张照片上，所以带上在册文件名；后端最多给前 50 条。 */
  items: (PreviewRepairOutcome & { filename: string })[];
  repaired?: number;
  refused?: number;
  remaining?: number;
  recheck?: ProjectDraftRecheck;
  /** 文件已经修好、口径这一趟却没跑成：两件事分开说，不吞掉"修好几张"。 */
  recheckError?: { code: string; message: string };
};

/** 当年没记像素版本的历史观察点名：能证明的与补不了的分开报，各带原因码。 */
export type ObservationPixelAudit = {
  /** 库里还有多少条没记像素版本。 */
  rows: number;
  /** 这一轮点到多少条；比总数少就说明还有没点到的。 */
  scanned: number;
  photos: number;
  ready: number;
  refused: number;
  reasons: Record<string, number>;
  items: { observationId: number; photoId: string; filename: string; outcome: string }[];
  backfilled?: number;
  remaining?: number;
  /** 补过之后按名单重算的口径欠账：这几张照片所在的方案版本号没动，但画面依据已经变了。 */
  recheck?: ProjectDraftRecheck;
  /** 指纹已经写进库里、口径这一趟却没跑成：两件事分开说，不吞掉"补上几条"。 */
  recheckError?: { code: string; message: string };
};

export interface DesktopProjectRepository extends ProjectRepository, DraftRepository {
  canvas?: import('./canvas').CanvasRepository;
  publication?: import('./publication').PublicationRepository;
  cloudAuth?: import('./cloud-auth').CloudAuthRepository;
  billing?: import('./billing').BillingRepository;
  sharing?: import('./sharing').SharingRepository;
  memory: import('./memory').MemoryRepository;
  diagnostics: import('./diagnostics').DiagnosticsRepository;
  readonly mode: 'desktop';
  subscribe(onChange: (projectId: string, revision?: number) => void): Promise<() => void>;
  archive(id: string): Promise<void>;
  restore(id: string): Promise<Project>;
  delete(id: string): Promise<void>;
  import(id: string, folder: boolean): Promise<string | null>;
  importJob(jobId: string): Promise<ImportJob>;
  cancelImport(jobId: string): Promise<void>;
  retryImport(jobId: string): Promise<string>;
  editorVersions?: import('./editorVersions').EditorVersionsRepository;
  relink(id: string, photoId: string): Promise<Project | null>;
  /** 外部就地改写原图后，把这条记录换成文件现在的内容（只读这一条记录在册的文件）。 */
  applyPhotoChange?(id: string, photoId: string): Promise<Project>;
  /** 只读：列出"外部软件写着、本地还没有"的判断；不点确认就什么都不动。 */
  sidecarProposals?(id: string): Promise<SidecarProposal[]>;
  applySidecarJudgement?(id: string, photoId: string): Promise<Project>;
  /** 一键回填：只把"本地还空着"的那些填上，人工判过的一条不碰（要读的边车仍由台账自己推）。 */
  backfillSidecarJudgements?(id: string): Promise<SidecarBatchOutcome>;
  /** 图库那两份中间文件坏了时按在册原图重算：只写缓存，原图一个字节都不动，所以不占修订号。 */
  repairPreviews?(id: string, photoIds: string[]): Promise<PreviewRepairOutcome[]>;
  /** 只读体检这一张：与重算前那道门同一套判断（含原图内容比对），一个字节都不写。 */
  inspectPreview?(id: string, photoId: string): Promise<PreviewRepairOutcome>;
  /** 只读点名整个项目：这一页之外的坏预览也一次数清，数的时候一个字节都不改。 */
  auditPreviews?(id: string): Promise<ProjectPreviewAudit>;
  /** 一次修完全项目点得出的坏预览，并把各份方案的口径跟着重算（不问模型、不上传照片）。 */
  repairAllPreviews?(id: string): Promise<ProjectPreviewAudit>;
  /** 接着补整项目还欠着的方案口径：纯本地重算，不重新修图、不问模型、不上传照片。 */
  recheckProjectEvidence?(id: string): Promise<ProjectDraftRecheck>;
  /** 点名"当年没记像素版本"的历史观察：只读本地文件比对，一个字节都不改、也不问模型。 */
  auditObservationVersions?(id: string): Promise<ObservationPixelAudit>;
  /** 把能证明的那部分观察补上像素版本，并让收着这些照片的方案口径跟着重算（不上传、不花额度）。 */
  backfillObservationVersions?(id: string): Promise<ObservationPixelAudit>;
  export(id: string, format: 'json' | 'csv' | 'html' | 'print' | 'sidecar' | 'shared', includePaths?: boolean): Promise<ExportReceipt | null>;
}
