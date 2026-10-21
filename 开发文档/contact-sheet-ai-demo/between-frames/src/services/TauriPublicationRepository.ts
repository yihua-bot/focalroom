import { invokeCommand as invoke } from './tauriCommand';
import { convertFileSrc } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { PublicationRepository } from '../domain/publication';

const publicationSchema = z.object({
  id: z.string(), project_id: z.string(), pub_type: z.enum(['photobook', 'exhibition', 'portfolio']),
  title: z.string(), state: z.enum(['draft', 'in_review', 'finalized', 'delivered']), created_at: z.string(),
}).transform(value => ({
  id: value.id, projectId: value.project_id, pubType: value.pub_type,
  title: value.title, state: value.state, createdAt: value.created_at,
}));
const pageSchema = z.object({
  id: z.string(), pageNumber: z.number().int(), widthMm: z.number().finite().positive(), heightMm: z.number().finite().positive(),
  frames: z.array(z.object({
    id: z.string(), xMm: z.number().finite(), yMm: z.number().finite(),
    widthMm: z.number().finite().positive(), heightMm: z.number().finite().positive(),
    fitMode: z.enum(['fit', 'fill', 'stretch']), thumbnailPath: z.string().nullable(),
  }).transform(({ thumbnailPath, ...frame }) => ({
    ...frame, thumbnailSrc: thumbnailPath ? convertFileSrc(thumbnailPath) : undefined,
  }))),
});

export class TauriPublicationRepository implements PublicationRepository {
  async pages(projectId: string, publicationId: string) {
    return z.array(pageSchema).parse(await invoke('pub_pages', { projectId, publicationId }));
  }
  async list(projectId: string) {
    return z.array(publicationSchema).parse(await invoke('pub_list', { projectId }));
  }
  async create(projectId: string, title: string, pubType = 'photobook') {
    return publicationSchema.parse(await invoke('pub_create', { projectId, cmd: { title, pubType } }));
  }
  async generate(projectId: string, publicationId: string, photosPerPage = 1) {
    return z.number().int().positive().parse(await invoke('pub_generate', {
      projectId, cmd: { publicationId, photosPerPage },
    }));
  }
}
