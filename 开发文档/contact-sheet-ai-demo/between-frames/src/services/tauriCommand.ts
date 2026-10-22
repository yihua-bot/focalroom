import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import { userFacingError } from './userFacingError';

const errorSchema = z.object({ code: z.string(), message: z.string(), retriable: z.boolean() });
export async function invokeCommand<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  try { return await invoke<T>(name, args); }
  catch (error) {
    const parsed = errorSchema.safeParse(error);
    if (parsed.success) throw Object.assign(new Error(userFacingError(parsed.data)), { code: parsed.data.code, cause: error });
    throw Object.assign(new Error(userFacingError(error)), { cause: error });
  }
}
