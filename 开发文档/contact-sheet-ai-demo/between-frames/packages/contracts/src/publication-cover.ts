import { z } from 'zod';
export const PUBLICATION_COVER_KEY='focalroomCoverV1';
export const publicationCoverSchema=z.strictObject({panelWidthMm:z.number().finite().min(50).max(1000),spineWidthMm:z.number().finite().min(0).max(200)}).refine(value=>value.panelWidthMm*2+value.spineWidthMm<=2000);
export const setPublicationCoverSchema=z.strictObject({pageId:z.uuid(),backPageId:z.uuid().nullable(),settings:publicationCoverSchema,heightMm:z.number().finite().min(50).max(2000),marginMm:z.number().finite().min(0).max(999),bleedMm:z.number().finite().min(0).max(25),contentPlacement:z.enum(['keepPosition','followPanels']),clearPageSettingsConfirmed:z.boolean()});
export type PublicationCover=z.infer<typeof publicationCoverSchema>;
export type SetPublicationCover=z.infer<typeof setPublicationCoverSchema>;
