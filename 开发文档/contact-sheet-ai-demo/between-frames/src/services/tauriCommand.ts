import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';

const errorSchema = z.object({ code: z.string(), message: z.string(), retriable: z.boolean() });
export async function invokeCommand<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try { return await invoke<T>(name, args); }
  catch (error) {
    const parsed = errorSchema.safeParse(error);
    if (parsed.success) throw Object.assign(new Error(parsed.data.message), { code: parsed.data.code });
    throw error instanceof Error ? error : new Error('操作未完成，请重试');
  }
}
