import { z } from 'zod';
export const PUBLICATION_GUIDES_KEY='focalroomPageGuidesV1';
const guides=z.array(z.number().finite().min(0).max(2000)).max(32).refine(values=>new Set(values).size===values.length);
export const publicationPageGuidesSchema=z.strictObject({
  columns:z.number().int().min(1).max(24),rows:z.number().int().min(1).max(24),
  columnGapMm:z.number().finite().min(0).max(100),rowGapMm:z.number().finite().min(0).max(100),
  verticalGuidesMm:guides,horizontalGuidesMm:guides,
});
export const setPublicationPageGuidesSchema=z.strictObject({pageId:z.uuid(),settings:publicationPageGuidesSchema});
export type PublicationPageGuides=z.infer<typeof publicationPageGuidesSchema>;
export type SetPublicationPageGuides=z.infer<typeof setPublicationPageGuidesSchema>;
export function publicationGuidesFitPage(page:{widthMm:number;heightMm:number;marginMm:number},settings:PublicationPageGuides){
  return [page.widthMm,page.heightMm,page.marginMm].every(Number.isFinite)&&page.widthMm>0&&page.heightMm>0&&page.marginMm>=0&&page.marginMm*2<Math.min(page.widthMm,page.heightMm)
    &&page.widthMm-2*page.marginMm-(settings.columns-1)*settings.columnGapMm>0&&page.heightMm-2*page.marginMm-(settings.rows-1)*settings.rowGapMm>0
    &&settings.verticalGuidesMm.every(value=>value>=0&&value<=page.widthMm)&&settings.horizontalGuidesMm.every(value=>value>=0&&value<=page.heightMm);
}
