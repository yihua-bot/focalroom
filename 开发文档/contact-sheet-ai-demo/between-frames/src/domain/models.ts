import type { DraftRepository } from './agent';
export type PhotoStatus = 'undecided' | 'keep' | 'reject';
export type PhotoFilter = 'all' | PhotoStatus;
export type WorkspaceView = 'grid' | 'sequence';
export type InspectorPanel = 'photo' | 'ai' | 'memory';
export interface InspectionImage { src: string; kind: 'original' | 'derived_preview' | 'embedded_preview' }

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

export interface ImportJob {
  jobId: string;
  projectId: string;
  status: 'running' | 'completed' | 'cancelled';
  completed: number;
  total: number;
  imported: number;
  skipped: number;
  failures: { path: string; code: string; message: string }[];
}

export interface ExportReceipt { path: string; sha256: string; bytes: number }

export interface DesktopProjectRepository extends ProjectRepository, DraftRepository {
  sharing?: import('./sharing').SharingRepository;
  memory: import('./memory').MemoryRepository;
  diagnostics: import('./diagnostics').DiagnosticsRepository;
  readonly mode: 'desktop';
  subscribe(onChange: (projectId: string, revision?: number) => void): Promise<() => void>;
  archive(id: string): Promise<void>;
  restore(id: string): Promise<Project>;
  import(id: string, folder: boolean): Promise<string | null>;
  importJob(jobId: string): Promise<ImportJob>;
  cancelImport(jobId: string): Promise<void>;
  retryImport(jobId: string): Promise<string>;
  relink(id: string, photoId: string): Promise<Project | null>;
  export(id: string, format: 'json' | 'csv' | 'html' | 'print' | 'sidecar' | 'shared', includePaths?: boolean): Promise<ExportReceipt | null>;
}
