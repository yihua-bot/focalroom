import { z } from 'zod';
import { publicationMasterSchema,publicationMasterLinkSchema } from '@betweenframes/contracts';
import type { PublicationLayout } from '../../domain/publication';
export function publicationMasters(layout:PublicationLayout){return z.array(publicationMasterSchema).max(16).parse(JSON.parse(layout.publication.specJson).focalroomPageMastersV1??[]);}
export function publicationMasterLink(page:PublicationLayout['pages'][number]){const value=page.layoutJson?JSON.parse(page.layoutJson)?.focalroomPageMasterV1:undefined;return value?publicationMasterLinkSchema.parse(value):undefined;}
