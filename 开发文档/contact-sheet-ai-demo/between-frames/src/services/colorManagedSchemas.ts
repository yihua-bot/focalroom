import { z } from 'zod';
const succeeded=(status:string)=>status==='transformed'||status==='assumed_srgb';
export const colorManagedPreviewSchema=z.object({
 status:z.string(),detail:z.string().nullable(),sourceProfile:z.string(),displayProfile:z.string(),
 pixelSource:z.literal('original').optional(),
 dataUrl:z.string().max(32*1024*1024).regex(/^data:image\/jpeg;base64,\/9j\/[A-Za-z0-9+/]*={0,2}$/).nullable(),
}).superRefine((value,ctx)=>{
 if(succeeded(value.status)!==(value.dataUrl!==null)) ctx.addIssue({code:'custom',message:'Color status and rendered pixels disagree'});
});
export const managedInspectionSchema=z.object({status:z.string(),detail:z.string().nullable(),path:z.string().min(1).nullable()}).superRefine((value,ctx)=>{
 if(succeeded(value.status)!==(value.path!==null)) ctx.addIssue({code:'custom',message:'Color status and rendered file disagree'});
});
