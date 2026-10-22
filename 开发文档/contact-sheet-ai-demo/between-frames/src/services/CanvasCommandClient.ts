import { parseCanvasReceipt, canvasRequestSchema, type CanvasAction, type CanvasCommandRequest } from '../domain/canvasCommands';

type Send = (request: CanvasCommandRequest) => Promise<unknown>;
const prefix = 'focalroom.canvas.pending.v3.';
const definitive = new Set(['REVISION_CONFLICT', 'VALIDATION_ERROR', 'NOT_FOUND', 'PERMISSION_DENIED', 'AUTHORITY_MISMATCH', 'CONFLICT', 'IDEMPOTENCY_CONFLICT']);
export class CanvasCommandClient {
  private busy = new Set<string>();
  constructor(private send: Send, private storage: Storage, private newId: () => string = () => crypto.randomUUID()) {}
  pending(projectId: string) {
    const found: CanvasCommandRequest[] = [];
    for (let i = 0; i < this.storage.length; i++) {
      const key = this.storage.key(i);
      if (!key?.startsWith(prefix)) continue;
      let value: unknown;
      try { value = JSON.parse(this.storage.getItem(key) ?? 'null'); } catch { continue; }
      const parsed = canvasRequestSchema.safeParse(value);
      if (parsed.success && parsed.data.projectId === projectId) found.push(parsed.data);
    }
    return found;
  }
  async execute(projectId: string, boardId: string, baseRevision: number, actions: CanvasAction[]) {
    const key = prefix + boardId;
    if (this.storage.getItem(key)) throw Object.assign(new Error('上次画布操作尚未确认，请先点击“确认上次操作”。'), { code: 'CANVAS_PENDING' });
    const parsed = canvasRequestSchema.safeParse({ schemaVersion: 3, commandId: this.newId(), idempotencyKey: this.newId(), projectId, boardId, baseRevision,
      operations: actions.map(action => ({ opId: this.newId(), action })) });
    if (!parsed.success) {
      const detail = parsed.error.issues.map(i => `${i.path.join('.')}: ${i.message}`).join('; ');
      throw new Error(`画布操作内容无效（${detail}）。请重新选择后重试。`);
    }
    const request = parsed.data;
    // 先落盘再发出；页面/应用退出后仍能重发完全相同的命令。
    try { this.storage.setItem(key, JSON.stringify(request)); }
    catch { throw new Error('无法保存这次画布操作，请检查可用存储空间后重试。'); }
    return this.submit(request);
  }
  async retry(projectId: string, boardId: string) {
    const request = canvasRequestSchema.parse(JSON.parse(this.storage.getItem(prefix + boardId) ?? 'null'));
    if (request.projectId !== projectId) throw new Error('这次操作不属于当前项目。');
    return this.submit(request);
  }
  private async submit(request: CanvasCommandRequest) {
    if (this.busy.has(request.boardId)) throw new Error('正在确认画布操作，请稍候。');
    this.busy.add(request.boardId);
    try {
      const receipt = parseCanvasReceipt(request, await this.send(request));
      this.storage.removeItem(prefix + request.boardId);
      return receipt;
    } catch (error) {
      if (definitive.has((error as { code?: string })?.code ?? '')) {
        this.storage.removeItem(prefix + request.boardId);
        throw error;
      }
      throw Object.assign(new Error('尚未确认画布操作结果，请点击“确认上次操作”。'), { cause: error, code: 'CANVAS_PENDING' });
    } finally { this.busy.delete(request.boardId); }
  }
}
