import { describe, expect, it } from 'vitest';
import { promotionSchema, sharedSnapshotSchema } from './cloud';
const snapshot = { photos: [{ id: 'photo', filename: 'frame.jpg', source_sha256: 'a'.repeat(64), selection: 'KEEP' }], sequences: [{ id: 'sequence', title: '作品序列', items: [{ id: 'item', photo_id: 'photo' }] }] };
const manifest = { workspace_id: 'workspace', local_project_id: 'local', promotion_id: 'promotion', title: '项目', from_revision: 1, snapshot, media_policy: { previews_only: true, strip_gps: true, max_long_edge: 2048, ai_upload_authorized: false } };
describe('共享迁移契约', () => {
  it('不接受原图路径、额外授权或隐式 AI 上传', () => {
    expect(promotionSchema.safeParse(manifest).success).toBe(true);
    expect(promotionSchema.safeParse({ ...manifest, original_path: '/private/image.jpg' }).success).toBe(false);
    expect(promotionSchema.safeParse({ ...manifest, media_policy: { ...manifest.media_policy, ai_upload_authorized: true } }).success).toBe(false);
    expect(promotionSchema.safeParse({ ...manifest, snapshot: { ...snapshot, photos: [{ ...snapshot.photos[0], originalPath: '/private/image.jpg' }] } }).success).toBe(false);
  });
  it('拒绝重复照片、重复序列项与其他项目的照片引用', () => {
    expect(sharedSnapshotSchema.safeParse({ ...snapshot, photos: [...snapshot.photos, ...snapshot.photos] }).success).toBe(false);
    expect(sharedSnapshotSchema.safeParse({ ...snapshot, sequences: [{ ...snapshot.sequences[0], items: [{ id: 'foreign', photo_id: 'other' }] }] }).success).toBe(false);
    expect(sharedSnapshotSchema.safeParse({ ...snapshot, sequences: [{ ...snapshot.sequences[0], items: [...snapshot.sequences[0].items, ...snapshot.sequences[0].items] }] }).success).toBe(false);
  });
});
