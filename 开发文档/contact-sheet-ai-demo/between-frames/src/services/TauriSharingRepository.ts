import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";
import type { ShareRequest, SharingRepository } from "../domain/sharing";
const status = z.object({
  project_id: z.string(),
  authority: z.enum(["local", "local-promoting", "shared"]),
  state: z.string().optional(),
  remote: z
    .object({
      webUrl: z.string().optional(),
      projectId: z.string().optional(),
      binding: z
        .object({
          baseUrl: z.string(),
          webUrl: z.string(),
          userId: z.string(),
          workspaceId: z.string(),
        })
        .optional(),
    })
    .nullable()
    .optional(),
});
export class TauriSharingRepository implements SharingRepository {
  async status(projectId: string) {
    return status.parse(await invoke("get_sharing_status", { projectId }));
  }
  async share(request: ShareRequest) {
    try {
      return status.parse(await invoke("share_project", { request }));
    } catch (error) {
      const parsed = z
        .object({ code: z.string(), message: z.string() })
        .safeParse(error);
      const wrapped = new Error(
        parsed.success
          ? parsed.data.message
          : "共享操作失败，请检查服务连接后重试",
      );
      Object.defineProperty(wrapped, "cause", { value: error });
      throw wrapped;
    }
  }
}
