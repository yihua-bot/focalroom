import { z } from 'zod';
export const publicationChapterSchema=z.strictObject({
  id:z.uuid(),title:z.string().min(1).max(120).refine(value=>value===value.trim()&&Array.from(value).every(char=>char.charCodeAt(0)>=32&&(char.charCodeAt(0)<127||char.charCodeAt(0)>159)&&![0xfeff,0x2028,0x2029].includes(char.charCodeAt(0)))),
  startPageId:z.uuid(),
});
export const publicationChaptersSchema=z.array(publicationChapterSchema).max(64).superRefine((chapters,ctx)=>{
  if(new Set(chapters.map(item=>item.id)).size!==chapters.length||new Set(chapters.map(item=>item.startPageId)).size!==chapters.length)
    ctx.addIssue({code:'custom',message:'Duplicate chapter or starting page'});
});
export type PublicationChapter=z.infer<typeof publicationChapterSchema>;
