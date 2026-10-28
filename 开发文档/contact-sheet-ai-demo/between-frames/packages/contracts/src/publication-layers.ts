import { z } from 'zod';
export const PUBLICATION_LAYERS_KEY='focalroomLayersV1';
export const publicationLayerNodeSchema=z.strictObject({
  id:z.uuid(),kind:z.enum(['frame','text','group']),parentId:z.uuid().nullable(),
  name:z.string().trim().min(1).max(80).refine(value=>Array.from(value).every(character=>{const code=character.codePointAt(0)!;return code>=32&&(code<127||code>159);})).nullable(),
  visible:z.boolean(),locked:z.boolean(),
});
export const publicationLayersSchema=z.strictObject({nodes:z.array(publicationLayerNodeSchema).max(4608)});
export type PublicationLayerNode=z.infer<typeof publicationLayerNodeSchema>;
export type PublicationLayers=z.infer<typeof publicationLayersSchema>;
