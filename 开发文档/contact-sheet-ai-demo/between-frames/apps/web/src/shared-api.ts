import { userFacingError } from '../../../src/services/userFacingError';
import { z } from "zod";
import { sharedSnapshotSchema } from "@betweenframes/contracts";
export const sessionSchema = z.object({
  id: z.string(),
  email: z.string(),
  name: z.string(),
  avatar: z.string().nullable().optional(),
  emailVerified: z.boolean().optional(),
  csrf: z.string(),
});
export type Session = z.infer<typeof sessionSchema>;
const operation = z.object({
  op: z.enum(["add", "move", "remove"]),
  operation_id: z.string(),
  photo_id: z.string(),
  to_index: z.number().optional(),
  reason: z.string().optional(),
});
export const snapshotSchema = z.object({
  project_id: z.string(),
  workspace_id: z.string(),
  title: z.string(),
  state: z.enum(["STAGING", "SHARED"]),
  revision: z.number().int(),
  cursor: z.string(),
  snapshot: sharedSnapshotSchema,
  principal: z.object({
    role: z.string(),
    can_approve: z.boolean(),
    can_edit_formal: z.boolean(),
  }),
  previews: z.array(
    z.object({
      photo_id: z.string(),
      content_sha256: z.string(),
      width: z.number(),
      height: z.number(),
    }),
  ),
  comments: z.array(
    z.object({
      id: z.string(),
      author_id: z.string(),
      reviewer: z.string(),
      body: z.string(),
      anchor: z.object({
        type: z.string(),
        photo_id: z.string().optional(),
        x: z.number().optional(),
        y: z.number().optional(),
        sequence_item_id: z.string().optional(),
        revision_id: z.string().optional(),
      }),
      parent_id: z.string().nullable(),
      state: z.string(),
    }),
  ),
  proposals: z.array(
    z.object({
      id: z.string(),
      author_id: z.string(),
      author: z.string(),
      title: z.string(),
      base_revision: z.number(),
      revision: z.number(),
      nonce: z.string(),
      state: z.string(),
      operations: z.array(operation),
    }),
  ),
  grants: z.array(
    z.object({
      user_id: z.string(),
      email: z.string(),
      role: z.string(),
      active: z.boolean(),
      can_approve: z.boolean(),
      can_edit_formal: z.boolean(),
    }),
  ),
});
export type CloudSnapshot = z.infer<typeof snapshotSchema>;
export type DraftOperation = z.infer<typeof operation>;
export const proposalDiffSchema = z.object({
  proposalId: z.string(),
  state: z.string(),
  baseRevision: z.number().int(),
  currentRevision: z.number().int(),
  stale: z.boolean(),
  sequence: z
    .object({
      id: z.string(),
      before: z.array(z.string()),
      after: z.array(z.string()),
      added: z.array(z.string()),
      removed: z.array(z.string()),
      moved: z.array(
        z.object({
          photo_id: z.string(),
          fromIndex: z.number().int(),
          toIndex: z.number().int(),
        }),
      ),
    })
    .nullable(),
});
export type ProposalDiff = z.infer<typeof proposalDiffSchema>;
export class CloudError extends Error {
  readonly code: string;
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}
export async function request<T = unknown>(
  path: string,
  session: Session | null,
  method = "GET",
  body?: unknown,
): Promise<T> {
  const response = await fetch(`/api/v2${path}`, {
    method,
    credentials: "same-origin",
    headers: {
      ...(session ? { "X-FocalRoom-CSRF": session.csrf } : {}),
      ...(body instanceof File
        ? { "Content-Type": "image/jpeg" }
        : { "Content-Type": "application/json" }),
    },
    ...(body !== undefined
      ? { body: body instanceof File ? body : JSON.stringify(body) }
      : {}),
  });
  const value = await response.json();
  if (!response.ok)
    throw new CloudError(
      value.error?.code ?? "SERVICE_UNAVAILABLE",
      value.error?.message ?? "请求失败，请稍后重试",
    );
  return value as T;
}
export function commandFor(
  state: CloudSnapshot,
  kind: string,
  payload: unknown,
) {
  const id = crypto.randomUUID();
  return {
    version: "2.0",
    command_id: id,
    workspace_id: state.workspace_id,
    project_id: state.project_id,
    expected_revision: state.revision,
    idempotency_key: id,
    kind,
    payload,
  };
}

export const errorMessage = (error: unknown) =>
  userFacingError(error);
