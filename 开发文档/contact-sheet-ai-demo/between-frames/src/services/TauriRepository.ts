import { managedInspectionSchema } from './colorManagedSchemas';
import { TauriDiagnosticsRepository } from './TauriDiagnosticsRepository';
import { TauriEditorVersionsRepository } from './TauriEditorVersionsRepository';
import { TauriAgentRepository } from './TauriAgentRepository';
import { TauriMemoryRepository } from './TauriMemoryRepository';
import { TauriSharingRepository } from './TauriSharingRepository';
import { TauriCanvasRepository } from './TauriCanvasRepository';
import { TauriBillingRepository } from './TauriBillingRepository';
import { userFacingError } from './userFacingError';
import { colorStatusLabel, zh } from '../app/copy';
import { TauriPublicationRepository } from './TauriPublicationRepository';
import { invokeCommand as command } from './tauriCommand';
import type { SelectionDraft } from '../domain/agent';
import { convertFileSrc, invoke, isTauri } from '@tauri-apps/api/core';
import { z } from 'zod';
import { listen } from '@tauri-apps/api/event';
import { PROJECT_CHANGED_EVENT, PHOTOS_IMPORTED_EVENT, type DomainCommand, type GuiCommandRequest } from '../domain/commands.generated';
import type { DesktopProjectRepository, InspectionImage, PhotoStatus, Project, ProjectRepository } from '../domain/models';

const eventSchema = z.object({ id: z.string(), photoId: z.string().nullable(), actor: z.enum(['user', 'system']), kind: z.string(), previousJson: z.string().nullable(), nextJson: z.string().nullable(), reason: z.string().nullable(), contextJson: z.string(), at: z.string() });
const projectSchema = z.object({
  id: z.string(), archived: z.boolean(), title: z.string(), brief: z.string(), revision: z.number().int(), updatedAt: z.string(),
  photos: z.array(z.object({ id: z.string(), index: z.number().int(), name: z.string(), originalPath: z.string(), thumbnailPath: z.string(), previewPath: z.string(), contentSha256: z.string(), width: z.number(), height: z.number(), fileSize: z.number(), status: z.enum(['undecided', 'keep', 'reject']), note: z.string(), rating: z.number().int().min(0).max(5), isMissing: z.boolean() })),
  sequence: z.array(z.string()), events: z.array(eventSchema),
  history: z.object({ undoGroupId: z.string().nullable(), redoGroupId: z.string().nullable(), undoCount: z.number().int(), redoCount: z.number().int() }),
  versions: z.array(z.object({ id: z.string(), label: z.string(), basedOnRevision: z.number().int(), createdAt: z.string(), photoIds: z.array(z.string()), photos: z.array(z.object({ photoId: z.string(), status: z.enum(['keep','reject','undecided']), rating: z.number().int(), note: z.string() })) })),
});
const changeSchema = z.object({ photoId: z.string().min(1), path: z.string(), filename: z.string(), previousSha256: z.string().min(1), incomingSha256: z.string().min(1), previousWidth: z.number().finite(), previousHeight: z.number().finite(), previousSize: z.number().finite(), incomingWidth: z.number().finite(), incomingHeight: z.number().finite(), incomingSize: z.number().finite() });
const jobSchema = z.object({ jobId: z.string(), projectId: z.string(), status: z.enum(['running', 'completed', 'cancelled']), completed: z.number(), total: z.number(), imported: z.number(), skipped: z.number(), changes: z.array(changeSchema), failures: z.array(z.object({ path: z.string(), code: z.string(), message: z.string() })) });

const proposalSchema = z.object({ photoId: z.string().min(1), filename: z.string(), sidecarPath: z.string().min(1), sidecarRating: z.number().int().min(0).max(5).nullable(), localRating: z.number().finite(), appliesRating: z.boolean(), sidecarPick: z.boolean().nullable(), localStatus: z.enum(['undecided', 'keep', 'reject']), appliesStatus: z.boolean(), colorLabel: z.number().int().min(0).max(8).nullable(), ambiguousRating: z.boolean(), batchSafe: z.boolean() });

/** 本地预览体检与重算共用一份回执形状：结局是后端原因码，界面只负责说成人话。 */
const previewOutcomeSchema = z.object({ photoId: z.string(), outcome: z.string(), rebuilt: z.array(z.string()), detail: z.string() });

/** 整项目方案口径的欠账：几个数字之间必须自洽，"还差多少"不能和"补了多少"对不上。 */
const projectRecheckSchema = z.object({
  drafts: z.number().int().nonnegative(),
  draftsScanned: z.number().int().nonnegative(),
  recheckOwed: z.number().int().nonnegative(),
  recheckedDrafts: z.number().int().nonnegative(),
  recheckFailed: z.number().int().nonnegative(),
  recheckRemaining: z.number().int().nonnegative(),
}).refine(report => report.draftsScanned <= report.drafts
  && report.recheckOwed <= report.draftsScanned
  && report.recheckedDrafts + report.recheckFailed <= report.recheckOwed
  && report.recheckRemaining === report.recheckOwed - report.recheckedDrafts);

/** 历史观察像素版本的点名单：能证明的与补不了的分开报，几个数字之间还要自洽。 */
const pixelLedgerSchema = z.object({
  rows: z.number().int().nonnegative(),
  scanned: z.number().int().nonnegative(),
  photos: z.number().int().nonnegative(),
  ready: z.number().int().nonnegative(),
  refused: z.number().int().nonnegative(),
  reasons: z.record(z.string(), z.number().int().nonnegative()),
  items: z.array(z.object({
    observationId: z.number().int().positive(),
    photoId: z.string().min(1),
    filename: z.string().min(1),
    outcome: z.string().min(1),
  })),
  backfilled: z.number().int().nonnegative().optional(),
  remaining: z.number().int().nonnegative().optional(),
  recheck: projectRecheckSchema.optional(),
  recheckError: z.object({ code: z.string().min(1), message: z.string() }).optional(),
}).refine(report => report.ready + report.refused === report.scanned
  && report.scanned <= report.rows
  && report.photos <= report.scanned
  && Object.values(report.reasons).reduce((sum, n) => sum + n, 0) === report.refused
  && (report.backfilled === undefined || (report.backfilled <= report.ready
    && report.remaining === report.ready - report.backfilled)));

/** 事件类型 → 人类可读描述；操作历史入口与事件映射共用。 */
export const EVENT_KIND_LABELS: Record<string, string> = {
  decision: '修改选片状态', note: '更新照片备注', compare: '记录两图偏好', reorder: '调整作品序列', import: '导入照片', relink: '重新定位照片', content_apply: '更新原图内容记录',
  project: '更新项目意图', project_create: '创建项目', rating: '保存人工评分', undo: '撤销人工编辑', redo: '重做人工编辑',
  version_save: '保存作品版本', version_restore: '恢复作品版本', project_archive: '归档项目', project_restore: '恢复项目',
  sidecar_apply: '回填外部软件的评分与选片',
  draft_apply: '采纳草案', draft_rebase: '根据当前作品重新比较方案', draft_edit: '调整草案顺序', draft_discard: '丢弃草案',
};

export class TauriRepository implements DesktopProjectRepository {
  readonly editorVersions = new TauriEditorVersionsRepository();
  readonly canvas = new TauriCanvasRepository();
  readonly publication = new TauriPublicationRepository();
  readonly sharing = new TauriSharingRepository();
  readonly cloudAuth = this.sharing.auth;
  readonly billing = new TauriBillingRepository();
  readonly mode = 'desktop' as const;
  readonly agent = new TauriAgentRepository();
  async setTags(projectId: string, tags: string[]) {
   await invoke('set_project_tags', { projectId, tags });
  }
  readonly memory = new TauriMemoryRepository();
  readonly diagnostics = new TauriDiagnosticsRepository();
  private snapshots = new Map<string, Project>();
  private originalLoaders = new Map<string, () => Promise<InspectionImage>>();
  private managedLoaders = new Map<string, () => Promise<InspectionImage>>();

  private originalLoader(projectId: string, photoId: string) {
    const key = `${projectId}:${photoId}`;
    let loader = this.originalLoaders.get(key);
    if (!loader) {
      loader = async () => {
        const source = z.discriminatedUnion('kind', [
          z.object({ path: z.string(), kind: z.literal('original') }),
          z.object({ path: z.string(), kind: z.literal('derived_preview') }),
          z.object({ path: z.string(), kind: z.literal('display_managed') }),
          z.object({ dataUrl: z.string().max(5_600_000).regex(/^data:image\/jpeg;base64,[A-Za-z0-9+/]+={0,2}$/), kind: z.literal('embedded_preview') }),
        ])
          .parse(await command('get_original_photo', { projectId, photoId }));
        return { src: source.kind === 'embedded_preview' ? source.dataUrl : convertFileSrc(source.path), kind: source.kind };
      };
      this.originalLoaders.set(key, loader);
    }
    return loader;
  }

  /** 色彩管理开启时 100% 检视用的全分辨率显示器渲染。后端说没做成时抛出原因，视口因此
   *  继续显示受限的色彩预览，不会拿一张没变换过的原图冒充显示器色彩。 */
  private managedOriginalLoader(projectId: string, photoId: string) {
    const key = `${projectId}:${photoId}`;
    let loader = this.managedLoaders.get(key);
    if (!loader) {
      loader = async () => {
        const parsed = managedInspectionSchema.safeParse(await command('get_display_managed_inspection', { projectId, photoId }));
        if (!parsed.success) throw new Error(zh.colorManagedInvalidResult);
        const result = parsed.data;
        if (!result.path) {
          const reason = colorStatusLabel(result.status);
          throw new Error(result.detail ? `${reason}：${result.detail}` : reason);
        }
        return { src: convertFileSrc(result.path), kind: 'display_managed' as const };
      };
      this.managedLoaders.set(key, loader);
    }
    return loader;
  }

  private map(raw: unknown): Project {
    const parsed = projectSchema.parse(raw);
    const current = this.snapshots.get(parsed.id);
    if (current && (current.revision ?? 0) > parsed.revision) return current;
    const project: Project = { ...parsed, photos: parsed.photos.map(photo => ({ ...photo, src: convertFileSrc(photo.thumbnailPath), previewSrc: convertFileSrc(photo.previewPath),
      loadOriginal: this.originalLoader(parsed.id, photo.id), loadManagedOriginal: this.managedOriginalLoader(parsed.id, photo.id) })),
      events: parsed.events.map(event => ({ id: event.id, type: event.kind, photoId: event.photoId ?? undefined, at: event.at, actor: event.actor,
        description: event.reason || (EVENT_KIND_LABELS[event.kind] ?? event.kind),
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
    try { raw = await command('execute_gui_command', { request, compact: true }); }
    catch (error) {
      if (error instanceof Error && 'code' in error) throw error;
      raw = await command('execute_gui_command', { request, compact: true });
    }
    const reply = replySchema.parse(raw);
    if (!reply.ok || !reply.data?.project) throw Object.assign(new Error(userFacingError(reply.error)), { code: reply.error?.code, currentRevision: reply.error?.currentRevision });
    const current = this.snapshots.get(id)?.revision ?? 0;
    if (reply.data.revision < current) return this.get(id);
    return this.map(reply.data.project);
  }
  async list(archived = false) { return z.array(projectSchema).parse(await command('list_projects', { archived, compact: true })).map(project => this.map(project)); }
  async get(id: string) { return this.map(await command('get_project', { projectId: id, compact: true })); }
  async create(title: string, brief: string) { return this.execute(crypto.randomUUID(), { type: 'CreateProject', title, brief }, 0); }
  async edit(id: string, title: string, brief: string) { return this.execute(id, { type: 'UpdateProject', title, brief }); }
  async decide(id: string, photoId: string, status: PhotoStatus) { return this.decideMany(id, [photoId], status); }
  async decideMany(id: string, photoIds: string[], status: PhotoStatus) { return this.execute(id, { type: 'SetPhotoDecision', photoIds, status }); }
  async rate(id: string, photoIds: string[], rating: number) { return this.execute(id, { type: 'SetPhotoRating', photoIds, rating }); }
  async setBookMark(id: string, photoId: string, mark: 'spread' | 'full' | null) { return this.execute(id, { type: 'SetPhotoBookMark', photoId, mark }); }
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
  async delete(id: string) { await command('delete_project', { projectId: id }); this.snapshots.delete(id); }
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
  // 只点名照片、不点名文件：要读哪个文件由后端按这条记录在册的原图决定，界面给不了路径。
  async applyPhotoChange(id: string, photoId: string) { return this.map(await command('apply_photo_change', { projectId: id, expectedRevision: this.revision(id), photoId })); }
  // 修坏掉的中间文件：后端只写图库自己生成的缓存，原图一个字节都不动，所以不占修订号。
  async repairPreviews(id: string, photoIds: string[]) {
    return z.array(previewOutcomeSchema).parse(await command('repair_photo_previews', { projectId: id, photoIds }));
  }
  // 只读体检这一张：结局词表与重算共用，界面不用再翻译一遍；这条路不写文件，所以 rebuilt 一律为空。
  async inspectPreview(id: string, photoId: string) {
    return previewOutcomeSchema.parse(await command('inspect_photo_preview', { projectId: id, photoId }));
  }
  // 整项目点名与一次修完：都只按库里在册的记录动手，界面给不了路径，返回值也只有计数和原因码。
  private ledger(value: unknown) {
    const count = z.number().int().nonnegative();
    return z.object({
      total: count,
      scanned: count,
      healthy: count,
      needsRepair: count,
      blocked: count,
      reasons: z.record(z.string(), count),
      items: z.array(previewOutcomeSchema.extend({ filename: z.string() })),
      repaired: count.optional(),
      refused: count.optional(),
      remaining: count.optional(),
      recheck: projectRecheckSchema.optional(),
      recheckError: z.object({ code: z.string().min(1), message: z.string() }).optional(),
    }).parse(value);
  }
  async auditPreviews(id: string) { return this.ledger(await command('audit_photo_previews', { projectId: id })); }
  async repairAllPreviews(id: string) { return this.ledger(await command('repair_all_broken_previews', { projectId: id })); }
  // 接着补还欠着的口径：不重新修图，也不问模型，只把库里的事实再比一遍。
  async recheckProjectEvidence(id: string) { return projectRecheckSchema.parse(await command('recheck_project_draft_evidence', { projectId: id })); }
  // 历史观察的像素版本：点名只读本地文件，回填写的也是本地算出来的指纹，两条都不碰模型。
  async auditObservationVersions(id: string) { return pixelLedgerSchema.parse(await command('audit_observation_pixel_versions', { projectId: id })); }
  async backfillObservationVersions(id: string) { return pixelLedgerSchema.parse(await command('backfill_observation_pixel_versions', { projectId: id })); }
  // 读边车是只读的：不占修订号，也不进幂等命令通道。
  async sidecarProposals(id: string) { return z.array(proposalSchema).parse(await command('sidecar_proposals', { projectId: id })); }
  async applySidecarJudgement(id: string, photoId: string) { return this.map(await command('apply_sidecar_judgement', { projectId: id, expectedRevision: this.revision(id), photoId })); }
  // 一键回填只收项目与期望修订号：要读哪些边车由 Rust 从台账推，界面递不进路径。
  async backfillSidecarJudgements(id: string) {
    const reply = z.object({ applied: z.number().int().min(0), skipped: z.number().int().min(0), project: z.unknown() }).parse(await command('backfill_sidecar_judgements', { projectId: id, expectedRevision: this.revision(id) }));
    this.map(reply.project);
    return { applied: reply.applied, skipped: reply.skipped };
  }
  async export(id: string, format: 'json' | 'csv' | 'html' | 'print' | 'sidecar' | 'shared', includePaths = false) {
    return z.object({ path: z.string(), sha256: z.string(), bytes: z.number() }).nullable().parse(await command('export_project', { projectId: id, format, includePaths }));
  }
}

export const isDesktopRepository = (repository: ProjectRepository): repository is DesktopProjectRepository => repository.mode === 'desktop';
export const desktopRuntime = isTauri;
