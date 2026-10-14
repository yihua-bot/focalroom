import type { PhotoStatus, PreviewEvent, Project, ProjectRepository } from '../domain/models';

const names = ['某个上午', '交错的视线', '城市边缘', '远处的光', '玻璃之后', '缓慢时刻', '街角练习', '不在场', '安静的人', '黄昏之前', '偏离中心', '无人应答', '近处与远处', '空间里的时间', '未完成的动作', '转身之后', '影子的长度', '另一次凝视', '记忆碎片', '经过的人', '黑与白', '片刻之间', '远行', '光线的回声', '重复的形状', '消失的轮廓', '陌生的日常', '最后一张'];
const images = import.meta.glob('../../../assets/frame_*.jpg', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;

export function createSampleProject(): Project {
  return {
    id: 'sample-project', title: '未命名的日常',
    brief: '记录那些未被安排的瞬间。不追求单张照片的完美，而关注画面之间的距离、光线与沉默。希望最终形成一组节奏克制、可以反复观看的摄影作品。',
    photos: names.map((name, i) => ({ id: `sample-${i + 1}`, index: i + 1, name,
      src: images[`../../../assets/frame_${String(i + 1).padStart(2, '0')}.jpg`], status: 'undecided', note: '', rating: 0 })),
    sequence: [], events: [],
  };
}

// 仅用于 M0 UI 验收，刷新后重置；不读取用户文件，不使用浏览器持久化。
export class MockRepository implements ProjectRepository {
  async setTags(projectId: string, tags: string[]) {
   const project = this.projects.find(item => item.id === projectId);
   if (project) project.tags = [...new Set(tags.map(tag => tag.trim()).filter(Boolean))];
  }
  private projects: Project[];
  private pending = new Map<string, Pick<Project, 'title' | 'brief' | 'photos' | 'sequence'>>();
  private stacks = new Map<string, { before: Pick<Project, 'title' | 'brief' | 'photos' | 'sequence'>; after: Pick<Project, 'title' | 'brief' | 'photos' | 'sequence'>; id: string }[]>();
  private cursors = new Map<string, number>();
  constructor(projects: Project[] = [createSampleProject()]) { this.projects = structuredClone(projects); }
  async list() { return structuredClone(this.projects); }
  async get(id: string) { return structuredClone(this.find(id)); }
  async create(title: string, brief: string) {
    const project: Project = { id: crypto.randomUUID(), title, brief, photos: [], sequence: [], events: [], revision: 1, versions: [] };
    this.projects.push(project);
    return structuredClone(project);
  }
  async edit(id: string, title: string, brief: string) {
    const project = this.begin(id);
    project.title = title; project.brief = brief;
    this.event(project, 'project', '修改项目意图');
    return structuredClone(project);
  }
  async decide(id: string, photoId: string, status: PhotoStatus) {
    return this.decideMany(id, [photoId], status);
  }
  async decideMany(id: string, photoIds: string[], status: PhotoStatus) {
    const project = this.begin(id);
    if (!photoIds.length || new Set(photoIds).size !== photoIds.length) throw new Error('选区为空或重复');
    const photos = photoIds.map(photoId => this.photo(project, photoId));
    if (photos.every(photo => photo.status === status)) return structuredClone(project);
    for (const photo of photos) {
      photo.status = status;
      if (status === 'keep' && !project.sequence.includes(photo.id)) project.sequence.push(photo.id);
    }
    if (status !== 'keep') project.sequence = project.sequence.filter(item => !photoIds.includes(item));
    this.event(project, 'decision', `${photoIds.length} 张：${status === 'keep' ? '保留' : status === 'reject' ? '淘汰' : '待定'}`, photoIds.length === 1 ? photoIds[0] : undefined);
    return structuredClone(project);
  }
  async rate(id: string, photoIds: string[], rating: number) {
    if (!Number.isInteger(rating) || rating < 0 || rating > 5) throw new Error('评分必须为 0–5');
    const project = this.begin(id);
    if (!photoIds.length || new Set(photoIds).size !== photoIds.length) throw new Error('选区为空或重复');
    const photos = photoIds.map(photoId => this.photo(project, photoId));
    if (photos.every(photo => photo.rating === rating)) return structuredClone(project);
    for (const photo of photos) photo.rating = rating;
    this.event(project, 'rating', `${photoIds.length} 张：人工评分 ${rating}`, photoIds.length === 1 ? photoIds[0] : undefined);
    return structuredClone(project);
  }
  async note(id: string, photoId: string, text: string) {
    const project = this.begin(id);
    const photo = this.photo(project, photoId);
    if (photo.note !== text) {
      photo.note = text;
      this.event(project, 'note', `${photo.name}：${text || '清空备注'}`, photoId);
    }
    return structuredClone(project);
  }
  async reorder(id: string, photoIds: string[]) {
    const project = this.begin(id);
    if (photoIds.length !== project.sequence.length || new Set(photoIds).size !== photoIds.length ||
      photoIds.some(photoId => !project.sequence.includes(photoId))) throw new Error('序列只能包含全部已保留照片，且不可重复');
    if (photoIds.join() === project.sequence.join()) return structuredClone(project);
    project.sequence = [...photoIds];
    this.event(project, 'reorder', '调整作品序列顺序');
    return structuredClone(project);
  }
  async compare(id: string, leftId: string, rightId: string, preferred: string | null, reason: string) {
    const project = this.begin(id);
    this.photo(project, leftId); this.photo(project, rightId);
    if (leftId === rightId || (preferred !== null && preferred !== leftId && preferred !== rightId)) throw new Error('比较照片不合法');
    this.event(project, 'compare', `${preferred ? `偏好${this.photo(project, preferred).name}` : '两图比较：不确定'}${reason ? ` · ${reason}` : ''}`, preferred ?? undefined);
    return structuredClone(project);
  }
  async undo(id: string) { return this.compensate(id, false); }
  async redo(id: string) { return this.compensate(id, true); }
  private compensate(id: string, redo: boolean) {
    const project = this.find(id);
    const stack = this.stacks.get(id) ?? [];
    const cursor = this.cursors.get(id) ?? 0;
    const entry = stack[redo ? cursor : cursor - 1];
    if (!entry) throw new Error(redo ? '没有可重做的操作' : '没有可撤销的操作');
    Object.assign(project, structuredClone(redo ? entry.after : entry.before));
    this.cursors.set(id, cursor + (redo ? 1 : -1));
    this.event(project, redo ? 'redo' : 'undo', redo ? '重做上一步' : '撤销上一步', undefined, false);
    return structuredClone(project);
  }
  async saveVersion(id: string, label: string) {
    const project = this.find(id);
    if (!label.trim() || label.length > 80) throw new Error('版本名称需为 1–80 字');
    (project.versions ??= []).unshift({ id: crypto.randomUUID(), label: label.trim(), basedOnRevision: project.revision ?? 1, createdAt: new Date().toISOString(), photoIds: [...project.sequence],
      photos: project.photos.map(photo => ({ photoId: photo.id, status: photo.status, rating: photo.rating, note: photo.note })) });
    this.event(project, 'version_save', '保存命名版本', undefined, false);
    return structuredClone(project);
  }
  async restoreVersion(id: string, versionId: string) {
    const project = this.begin(id);
    const version = project.versions?.find(item => item.id === versionId);
    if (!version) throw new Error('版本不属于该项目');
    for (const photo of project.photos) {
      const saved = version.photos.find(item => item.photoId === photo.id);
      if (saved) { photo.status = saved.status; photo.rating = saved.rating; photo.note = saved.note; }
      else photo.status = 'undecided';
    }
    project.sequence = [...version.photoIds];
    this.event(project, 'version_restore', `恢复版本：${version.label}`);
    return structuredClone(project);
  }
  private editState(project: Project) {
    return structuredClone({ title: project.title, brief: project.brief, photos: project.photos, sequence: project.sequence });
  }
  private begin(id: string) {
    const project = this.find(id);
    this.pending.set(id, this.editState(project));
    return project;
  }
  private updateHistory(project: Project) {
    const stack = this.stacks.get(project.id) ?? [];
    const cursor = this.cursors.get(project.id) ?? 0;
    project.history = { undoGroupId: stack[cursor - 1]?.id ?? null, redoGroupId: stack[cursor]?.id ?? null, undoCount: cursor, redoCount: stack.length - cursor };
  }
  private find(id: string) {
    const project = this.projects.find(item => item.id === id);
    if (!project) throw new Error('项目不存在');
    return project;
  }
  private photo(project: Project, id: string) {
    const photo = project.photos.find(item => item.id === id);
    if (!photo) throw new Error('照片不存在');
    return photo;
  }
  private event(project: Project, type: PreviewEvent['type'], description: string, photoId?: string, track = true) {
    if (track) {
      const before = this.pending.get(project.id);
      if (!before) throw new Error('缺少编辑前快照');
      const cursor = this.cursors.get(project.id) ?? 0;
      const stack = (this.stacks.get(project.id) ?? []).slice(0, cursor);
      stack.push({ before, after: this.editState(project), id: crypto.randomUUID() });
      this.stacks.set(project.id, stack); this.cursors.set(project.id, cursor + 1);
    }
    project.revision = (project.revision ?? 1) + 1;
    this.updateHistory(project);
    project.events.unshift({ id: crypto.randomUUID(), type, description, photoId, at: new Date().toISOString() });
  }
}
