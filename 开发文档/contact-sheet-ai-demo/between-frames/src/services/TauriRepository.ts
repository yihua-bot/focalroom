import { TauriAgentRepository } from './TauriAgentRepository';
import { TauriMemoryRepository } from './TauriMemoryRepository';
import type { SelectionDraft } from '../domain/agent';
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core';
import { z } from 'zod';
import { listen } from '@tauri-apps/api/event';
import { PROJECT_CHANGED_EVENT, PHOTOS_IMPORTED_EVENT, type DomainCommand, type GuiCommandRequest } from '../domain/commands.generated';
import type { DesktopProjectRepository, PhotoStatus, Project, ProjectRepository } from '../domain/models';

const eventSchema = z.object({ id: z.string(), photoId: z.string().nullable(), actor: z.enum(['user', 'system']), kind: z.string(), previousJson: z.string().nullable(), nextJson: z.string().nullable(), reason: z.string().nullable(), contextJson: z.string(), at: z.string() });
const projectSchema = z.object({
  id: z.string(), archived: z.boolean(), title: z.string(), brief: z.string(), revision: z.number().int(), updatedAt: z.string(),
  photos: z.array(z.object({ id: z.string(), index: z.number().int(), name: z.string(), originalPath: z.string(), thumbnailPath: z.string(), previewPath: z.string(), contentSha256: z.string(), width: z.number(), height: z.number(), fileSize: z.number(), status: z.enum(['undecided', 'keep', 'reject']), note: z.string(), rating: z.number().int().min(0).max(5), isMissing: z.boolean() })),
  sequence: z.array(z.string()), events: z.array(eventSchema),
  history: z.object({ undoGroupId: z.string().nullable(), redoGroupId: z.string().nullable(), undoCount: z.number().int(), redoCount: z.number().int() }),
  versions: z.array(z.object({ id: z.string(), label: z.string(), basedOnRevision: z.number().int(), createdAt: z.string(), photoIds: z.array(z.string()), photos: z.array(z.object({ photoId: z.string(), status: z.enum(['keep','reject','undecided']), rating: z.number().int(), note: z.string() })) })),
});
const jobSchema = z.object({ jobId: z.string(), projectId: z.string(), status: z.enum(['running', 'completed', 'cancelled']), completed: z.number(), total: z.number(), imported: z.number(), skipped: z.number(), failures: z.array(z.object({ path: z.string(), code: z.string(), message: z.string() })) });

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try { return await invoke<T>(name, args); }
  catch (error) {
    const parsed = z.object({ code: z.string(), message: z.string(), retriable: z.boolean() }).safeParse(error);
    if (parsed.success) throw Object.assign(new Error(parsed.data.message), { code: parsed.data.code });
    throw error instanceof Error ? error : new Error(`本地操作失败：${String(error)}`);
  }
}

export class TauriRepository implements DesktopProjectRepository {
  readonly mode = 'desktop' as const;
  readonly agent = new TauriAgentRepository();
  readonly memory = new TauriMemoryRepository();
  private snapshots = new Map<string, Project>();

  private map(raw: unknown): Project {
    const parsed = projectSchema.parse(raw);
    const current = this.snapshots.get(parsed.id);
    if (current && (current.revision ?? 0) > parsed.revision) return current;
    const project: Project = { ...parsed, photos: parsed.photos.map(photo => ({ ...photo, src: convertFileSrc(photo.thumbnailPath), previewSrc: convertFileSrc(photo.previewPath) })),
      events: parsed.events.map(event => ({ id: event.id, type: event.kind, photoId: event.photoId ?? undefined, at: event.at,
        description: event.reason || ({ decision: '修改选片状态', note: '更新照片备注', compare: '记录两图偏好', reorder: '调整作品序列', import: '导入照片', relink: '重新定位照片', project: '更新项目意图', project_create: '创建项目', rating: '保存人工评分', undo: '撤销人工编辑', redo: '重做人工编辑', version_save: '保存作品版本', version_restore: '恢复作品版本', project_archive: '归档项目', project_restore: '恢复项目', draft_apply: '采纳草案', draft_rebase: '重新比较草案基线', draft_edit: '调整草案顺序', draft_discard: '丢弃草案' }[event.kind] ?? event.kind),
        previous: event.previousJson ? JSON.parse(event.previousJson) : null, next: event.nextJson ? JSON.parse(event.nextJson) : null, context: JSON.parse(event.contextJson) })),
    };
    this.snapshots.set(project.id, project);
    return project;
  }
  private revision(id: string) {
    const revision = this.snapshots.get(id)?.revision;
    if (revision === undefined) throw new Error('请先打开项目再编辑');
    return revision;
  }
  private async execute(id: string, operation: DomainCommand, revision = this.revision(id)) {
    const commandId = crypto.randomUUID();
    const request: GuiCommandRequest = { commandId, projectId: id, expectedProjectRevision: revision, causationId: commandId, idempotencyKey: commandId, command: operation };
    const replySchema = z.object({
      ok: z.boolean(), requestId: z.string(), revision: z.number().int().nullable(),
      data: z.object({ requestId: z.string(), revision: z.number().int(), changed: z.boolean(), transactionGroupId: z.string().nullable(), project: projectSchema.nullable() }).nullable(),
      error: z.object({ code: z.string(), message: z.string(), retryable: z.boolean(), currentRevision: z.number().int().nullable() }).nullable(),
    });
    // 仅通信失败才按同一 envelope 重试；业务冲突不能生成新 ID 强行重放。
    let raw: unknown;
    try { raw = await command('execute_gui_command', { request }); }
    catch (error) {
      if (error instanceof Error && 'code' in error) throw error;
      raw = await command('execute_gui_command', { request });
    }
    const reply = replySchema.parse(raw);
    if (!reply.ok || !reply.data?.project) throw Object.assign(new Error(reply.error?.message ?? '命令没有返回项目'), { code: reply.error?.code, currentRevision: reply.error?.currentRevision });
    const current = this.snapshots.get(id)?.revision ?? 0;
    if (reply.data.revision < current) return this.get(id);
    return this.map(reply.data.project);
  }
  async list(archived = false) { return z.array(projectSchema).parse(await command('list_projects', { archived })).map(project => this.map(project)); }
  async get(id: string) { return this.map(await command('get_project', { projectId: id })); }
  async create(title: string, brief: string) { return this.execute(crypto.randomUUID(), { type: 'CreateProject', title, brief }, 0); }
  async edit(id: string, title: string, brief: string) { return this.execute(id, { type: 'UpdateProject', title, brief }); }
  async decide(id: string, photoId: string, status: PhotoStatus) { return this.decideMany(id, [photoId], status); }
  async decideMany(id: string, photoIds: string[], status: PhotoStatus) { return this.execute(id, { type: 'SetPhotoDecision', photoIds, status }); }
  async rate(id: string, photoIds: string[], rating: number) { return this.execute(id, { type: 'SetPhotoRating', photoIds, rating }); }
  async note(id: string, photoId: string, note: string) { return this.execute(id, { type: 'SetPhotoNote', photoId, note }); }
  async reorder(id: string, orderedPhotoIds: string[]) { return this.execute(id, { type: 'ReorderSequence', orderedPhotoIds }); }
  async compare(id: string, leftId: string, rightId: string, preferred: string | null, reason: string) {
    return this.execute(id, { type: 'RecordComparison', leftId, rightId, preference: preferred === null ? 'uncertain' : preferred === leftId ? 'left' : 'right', reason });
  }
  async undo(id: string) {
    const transactionGroupId = this.snapshots.get(id)?.history?.undoGroupId;
    if (!transactionGroupId) throw new Error('没有可撤销的操作');
    return this.execute(id, { type: 'UndoCommandGroup', transactionGroupId });
  }
  async redo(id: string) {
    const transactionGroupId = this.snapshots.get(id)?.history?.redoGroupId;
    if (!transactionGroupId) throw new Error('没有可重做的操作');
    return this.execute(id, { type: 'RedoCommandGroup', transactionGroupId });
  }
  async saveVersion(id: string, label: string) { return this.execute(id, { type: 'SaveSequenceVersion', label }); }
  async restoreVersion(id: string, versionId: string) { return this.execute(id, { type: 'RestoreSequenceVersion', versionId }); }
  async applyDraft(id: string, draft: SelectionDraft, approvedPhotoIds: string[], mode: 'partial' | 'full') { return this.execute(id, { type: 'ApplyDraft', draftId: draft.id, expectedDraftRevision: draft.revision, approvedPhotoIds, mode }); }
  async rebaseDraft(id: string, draft: SelectionDraft) { return this.execute(id, { type: 'RebaseDraft', draftId: draft.id, expectedDraftRevision: draft.revision }); }
  async updateDraft(id: string, draft: SelectionDraft, orderedPhotoIds: string[]) { return this.execute(id, { type: 'UpdateDraft', draftId: draft.id, expectedDraftRevision: draft.revision, orderedPhotoIds }); }
  async discardDraft(id: string, draft: SelectionDraft) { return this.execute(id, { type: 'DiscardDraft', draftId: draft.id, expectedDraftRevision: draft.revision }); }
  async archive(id: string) { await this.execute(id, { type: 'ArchiveProject' }); this.snapshots.delete(id); }
  async restore(id: string) { return this.execute(id, { type: 'RestoreProject' }); }
  async subscribe(onChange: (projectId: string, revision?: number) => void) {
    const changed = z.object({ projectId: z.string(), revision: z.number().int().optional() });
    const handle = (payload: unknown) => { const event = changed.safeParse(payload); if (event.success) onChange(event.data.projectId, event.data.revision); };
    const first = await listen(PROJECT_CHANGED_EVENT, event => handle(event.payload));
    try {
      const second = await listen(PHOTOS_IMPORTED_EVENT, event => handle(event.payload));
      return () => { first(); second(); };
    } catch (error) { first(); throw error; }
  }
  async import(id: string, folder: boolean) {
    const selectionToken = await command<string | null>('choose_import', { projectId: id, folder });
    return selectionToken ? command<string>('import_photos', { projectId: id, selectionToken }) : null;
  }
  async importJob(jobId: string) { return jobSchema.parse(await command('get_import_job', { jobId })); }
  async cancelImport(jobId: string) { await command('cancel_import', { jobId }); }
  async retryImport(jobId: string) { return command<string>('retry_import', { jobId }); }
  async relink(id: string, photoId: string) { const project = await command('relink_photo', { projectId: id, expectedRevision: this.revision(id), photoId }); return project ? this.map(project) : null; }
  async export(id: string, format: 'json' | 'csv' | 'html', includePaths = false) {
    return z.object({ path: z.string(), sha256: z.string(), bytes: z.number() }).nullable().parse(await command('export_project', { projectId: id, format, includePaths }));
  }
}

export const isDesktopRepository = (repository: ProjectRepository): repository is DesktopProjectRepository => repository.mode === 'desktop';
export const desktopRuntime = isTauri;
