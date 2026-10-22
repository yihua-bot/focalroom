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
export interface CanvasRepository {
  compareDraft?(projectId: string, boardId: string): Promise<{ draftRevision: number; sourceBoardId: string | null; sourceChanged: boolean; baselineNodes: CanvasPhoto[]; changes: Array<{ id: string; nodeId: string; kind: 'add' | 'change' | 'remove'; nodeType: string }> }>;
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
