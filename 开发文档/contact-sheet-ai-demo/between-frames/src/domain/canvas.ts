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
  decision?: 'pending' | 'accepted' | 'rejected' | 'deferred' | 'partial';
  fields?: EditorialResolution['field'][]; acceptedFields?: EditorialResolution['field'][]; fieldDecisions?: Record<string, 'pending'|'accepted'|'rejected'|'deferred'>;
}
export interface CanvasComparison {
  draftRevision: number; sourceBoardId: string | null; sourceRevision?: number | null; currentSourceRevision?: number | null;
  sourceChanged: boolean; baselineNodes: CanvasPhoto[]; draftNodes?: CanvasPhoto[]; changes: EditorialChange[];
  currentSourceNodes?: CanvasPhoto[];
}
export interface EditorialResolution { changeId: string; field: 'position'|'size'|'rotation'|'order'|'content'|'style'|'children'|'rendition'; choice: 'keep_current'|'use_draft' }
export interface AttributeConflict { changeId: string; nodeId: string; field: EditorialResolution['field']; baseline: unknown; current: unknown; proposed: unknown }
export interface EditorialSelection {
  draftId: string; draftRevision: number; targetBoardId: string; expectedTargetRevision: number; selectedChangeIds: string[];
  resolutions?: EditorialResolution[];
  fieldsByChange?: Record<string, EditorialResolution['field'][]>;
}
export interface MergeSimulation {
  simulationId: string; draftRevision: number; targetRevision: number;
  effectiveChangeIds: string[]; autoIncludedChangeIds: string[]; conflicts: string[]; mergedNodes: CanvasPhoto[]; canApply: boolean;
  nodeIdMap?: Record<string, string>;
  attributeConflicts?: AttributeConflict[];
}
export interface EditorialHistoryEntry {
  cursor: number; commandId: string; draftId: string; changeCount: number; createdAt: string; undone: boolean; compensated: boolean; isCompensation: boolean; nodeTypes: string[];
}
export interface RevertSimulation {
  mergeCommandId: string; targetRevision: number; simulationId: string; currentNodes: CanvasPhoto[]; mergedNodes: CanvasPhoto[]; conflicts: string[]; canApply: boolean;
}
export interface EditorialMessage { id: string; threadId: string; changeId: string | null; body: string; rootBody: string; createdAt: string; resolved: boolean; isReply: boolean }
export interface EditorialDiscussion { draftId: string; draftRevision: number; cursor: number; messages: EditorialMessage[]; totalMessages: number; eventCount: number; nextOffset: number | null }
export interface EditorialRevisionContext {
  draftId: string; parent: { kind: 'board'; id: string; revision: number }; parentTitle: string; discussion: EditorialDiscussion;
  decisions: Array<{ changeId: string; nodeType: string; kind: string; decision: string; fieldDecisions: Record<string,string> }>;
  observations: Array<{ photoId: string; photoHash: string; modelId: string; promptVersion: string; taskId: string; callId: string; visibleFacts: string[]; interpretation: string; uncertainty: string; currentHashMatches: boolean }>;
  totalDecisions: number; totalObservations: number; nextOffset: number | null; hasEarlier: boolean;
}
export interface CanvasRepository {
  editorialRevisionContext?(projectId: string, draftId: string, offset?: number): Promise<EditorialRevisionContext | null>;
  editorialDiscussion?(projectId: string, draftId: string, revision: number, offset?: number, cursor?: number): Promise<EditorialDiscussion>;
  addEditorialComment?(projectId: string, draftId: string, revision: number, comment: { changeId: string | null; threadId: string | null; body: string }): Promise<void>;
  resolveEditorialThread?(projectId: string, draftId: string, revision: number, threadId: string, resolved: boolean): Promise<void>;
  editorialHistory?(projectId: string, boardId: string, before?: number): Promise<EditorialHistoryEntry[]>;
  simulateRevert?(projectId: string, boardId: string, commandId: string): Promise<RevertSimulation>;
  revertMerge?(projectId: string, boardId: string, simulation: RevertSimulation): Promise<void>;
  compareDraft?(projectId: string, boardId: string): Promise<CanvasComparison>;
  simulateMerge?(projectId: string, selection: EditorialSelection): Promise<MergeSimulation>;
  applyMerge?(projectId: string, selection: EditorialSelection, simulationId: string): Promise<void>;
  recordDecision?(projectId: string, selection: EditorialSelection, decision: 'pending'|'rejected'|'deferred'): Promise<void>;
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
