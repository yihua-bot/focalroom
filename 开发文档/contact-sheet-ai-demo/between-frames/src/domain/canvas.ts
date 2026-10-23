export interface BoardInfo { id: string; projectId: string; title: string; state: string; owningTaskId?: string | null; revision?: number }
export interface CanvasNodeInfo {
  id: string; boardId: string; nodeType: string;
  x: number; y: number; width: number; height: number;
  rotation: number; zOrder: number;
  assetId: string | null; renditionRef: string | null;
}
export interface CanvasPhoto extends CanvasNodeInfo {
  thumbnailPath?: string;
  contentJson?: string | null;
  styleJson?: string | null;
  childrenJson?: string | null;
  locked?: boolean;
  visible?: boolean;
}
export interface AnnotationInfo {
  id: string; boardId: string; anchorType: string;
  x: number | null; y: number | null;
  author: string; body: string; resolved: boolean; createdAt: string;
}
export interface EditorialChange {
  id: string; nodeId: string; sourceNodeId?: string | null; kind: 'add' | 'change' | 'remove'; nodeType: string;
  kinds?: string[]; before?: CanvasPhoto | null; after?: CanvasPhoto | null; requiredDependencies?: string[];
}
export interface CanvasComparison {
  draftRevision: number; sourceBoardId: string | null; sourceRevision?: number | null; currentSourceRevision?: number | null;
  sourceChanged: boolean; baselineNodes: CanvasPhoto[]; draftNodes?: CanvasPhoto[]; changes: EditorialChange[];
}
export interface EditorialSelection {
  draftId: string; draftRevision: number; targetBoardId: string; expectedTargetRevision: number; selectedChangeIds: string[];
}
export interface MergeSimulation {
  simulationId: string; draftRevision: number; targetRevision: number;
  effectiveChangeIds: string[]; autoIncludedChangeIds: string[]; conflicts: string[]; mergedNodes: CanvasPhoto[]; canApply: boolean;
  nodeIdMap?: Record<string, string>;
}
export interface CanvasRepository {
  compareDraft?(projectId: string, boardId: string): Promise<CanvasComparison>;
  simulateMerge?(projectId: string, selection: EditorialSelection): Promise<MergeSimulation>;
  applyMerge?(projectId: string, selection: EditorialSelection, simulationId: string): Promise<void>;
  getSnapshot?(boardId: string): Promise<{ board: BoardInfo; nodes: CanvasPhoto[]; annotations: AnnotationInfo[] }>;
  pendingCommands?(projectId: string): Array<{ boardId: string }>;
  retryPending?(projectId: string, boardId: string): Promise<void>;
  placePhotos?(projectId: string, boardId: string, photos: Array<{ assetId: string; x: number; y: number; width: number; height: number }>): Promise<void>;
  setNodesLocked?(projectId: string, boardId: string, nodeIds: string[], locked: boolean): Promise<void>;
  listBoards(projectId: string): Promise<BoardInfo[]>;
  createBoard(projectId: string, title: string): Promise<BoardInfo>;
  getNodes(boardId: string): Promise<CanvasPhoto[]>;
  placePhoto(projectId: string, boardId: string, assetId: string, x: number, y: number, w: number, h: number): Promise<CanvasNodeInfo>;
  moveNodes(projectId: string, boardId: string, nodeIds: string[], dx: number, dy: number): Promise<void>;
  addText(projectId: string, boardId: string, nodeType: string, x: number, y: number, w: number, h: number, content: string, color?: string): Promise<CanvasNodeInfo>;
  deleteNode(projectId: string, boardId: string, nodeId: string): Promise<void>;
  addAnnotation(projectId: string, boardId: string, x: number, y: number, body: string): Promise<void>;
  resolveAnnotation(projectId: string, annotationId: string, resolved: boolean): Promise<void>;
  getAnnotations(boardId: string): Promise<AnnotationInfo[]>;
  addArrow(projectId: string, boardId: string, fromNodeId: string, toNodeId: string, color?: string): Promise<void>;
  undo(projectId: string, boardId: string): Promise<boolean>;
  redo(projectId: string, boardId: string): Promise<boolean>;
  adoptBoard(projectId: string, boardId: string): Promise<void>;
  groupNodes(projectId: string, boardId: string, nodeIds: string[], label: string): Promise<void>;
  resizeNode(projectId: string, boardId: string, nodeId: string, width: number, height: number): Promise<void>;
  setNodeLocked(projectId: string, boardId: string, nodeId: string, locked: boolean): Promise<void>;
  addInk(projectId: string, boardId: string, x: number, y: number, w: number, h: number, points: Array<{x:number;y:number;pressure?:number}>, color?: string, strokeWidth?: number): Promise<void>;
}
