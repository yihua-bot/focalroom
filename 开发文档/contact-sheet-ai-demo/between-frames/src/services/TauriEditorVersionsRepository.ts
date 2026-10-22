import { convertFileSrc } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { EditorVersionsRepository } from '../domain/editorVersions';
import { invokeCommand } from './tauriCommand';
const version = z.object({ id: z.string(), photoName: z.string(), provider: z.enum(['lightroom_classic', 'capture_one']), previewPath: z.string(), createdAt: z.string() });
export class TauriEditorVersionsRepository implements EditorVersionsRepository {
  async list(projectId: string) {
    const result = z.array(version).parse(await invokeCommand('list_editor_versions', { projectId }));
    return result.map(item => ({ ...item, previewSrc: convertFileSrc(item.previewPath) }));
  }
  async import(projectId: string) {
    return z.number().int().nonnegative().nullable().parse(await invokeCommand('import_editor_results', { projectId }));
  }
}
