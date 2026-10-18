import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { z } from "zod";
import type { CloudAuthRepository } from "../domain/cloud-auth";
const status = z.object({ authenticated: z.boolean(), waiting: z.boolean(), origin: z.string().url(), user: z.object({ id: z.string(), email: z.string(), name: z.string(), avatar: z.string().nullable().optional() }).nullable(), error: z.string().nullable() });
async function call<T>(name: string): Promise<T> {
  try { return await invoke<T>(name); }
  catch (cause) { const value = z.object({ message: z.string() }).safeParse(cause); throw Object.assign(new Error(value.success ? value.data.message : "云端连接失败，请重试"), {cause}); }
}
export class TauriCloudAuthRepository implements CloudAuthRepository {
  async status() { return status.parse(await call("cloud_login_status")); }
  open() { return call<void>("cloud_login_open"); }
  cancel() { return call<void>("cloud_login_cancel"); }
  logout() { return call<boolean>("cloud_login_logout"); }
  subscribe(callback: () => void) { return listen("cloud-login-updated", callback); }
}
