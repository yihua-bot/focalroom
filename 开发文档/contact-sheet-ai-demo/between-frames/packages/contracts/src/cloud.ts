import { z } from "zod";
const id = z.string().min(1).max(160);
const item = z.strictObject({ id, photo_id: id });
export const sharedSnapshotSchema = z
  .strictObject({
    photos: z
      .array(
        z.strictObject({
          id,
          filename: z.string().min(1).max(500),
          source_sha256: z.string().regex(/^[a-f0-9]{64}$/),
          selection: z.enum(["UNDECIDED", "KEEP", "REJECT"]),
        }),
      )
      .min(1)
      .max(10000),
    sequences: z
      .array(
        z.strictObject({
          id,
          title: z.string().min(1).max(200),
          items: z.array(item).max(10000),
        }),
      )
      .min(1)
      .max(20),
  })
  .superRefine((value, ctx) => {
    const photos = new Set(value.photos.map((p) => p.id));
    if (
      photos.size !== value.photos.length ||
      new Set(value.sequences.map((s) => s.id)).size !== value.sequences.length
    )
      ctx.addIssue({ code: "custom", message: "照片与序列 ID 必须唯一" });
    const allItems = value.sequences.flatMap((s) => s.items);
    if (new Set(allItems.map((i) => i.id)).size !== allItems.length)
      ctx.addIssue({ code: "custom", message: "序列项 ID 必须唯一" });
    for (const sequence of value.sequences) {
      if (
        new Set(sequence.items.map((i) => i.photo_id)).size !==
          sequence.items.length ||
        sequence.items.some((i) => !photos.has(i.photo_id))
      )
        ctx.addIssue({ code: "custom", message: "序列照片必须存在且不能重复" });
    }
  });
export type SharedSnapshot = z.infer<typeof sharedSnapshotSchema>;
export const promotionSchema = z.strictObject({
  workspace_id: id,
  local_project_id: id,
  promotion_id: id,
  title: z.string().trim().min(1).max(200),
  from_revision: z.number().int().min(0),
  snapshot: sharedSnapshotSchema,
  media_policy: z.strictObject({
    previews_only: z.literal(true),
    strip_gps: z.literal(true),
    max_long_edge: z.literal(2048),
    ai_upload_authorized: z.literal(false),
  }),
});
export type PromotionManifest = z.infer<typeof promotionSchema>;
