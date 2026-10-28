import { publicationFrameGeometrySchema } from '@betweenframes/contracts';
import type { PublicationFrameGeometry } from '../../domain/publication';
export const publicationFrameNumbers=['xMm','yMm','widthMm','heightMm','cropX','cropY','cropZoom'] as const;
export type PublicationFrameValues=Record<typeof publicationFrameNumbers[number],string>&{fitMode:PublicationFrameGeometry['fitMode']};
export const publicationFrameValues=(frame:PublicationFrameGeometry):PublicationFrameValues=>({...Object.fromEntries(publicationFrameNumbers.map(key=>[key,String(frame[key]*(key==='cropX'||key==='cropY'?100:1))])) as Record<typeof publicationFrameNumbers[number],string>,fitMode:frame.fitMode});
export function publicationParseFrame(values:PublicationFrameValues):PublicationFrameGeometry|undefined{
  const numbers=Object.fromEntries(publicationFrameNumbers.map(key=>[key,values[key].trim()===''?NaN:Number(values[key])/(key==='cropX'||key==='cropY'?100:1)]));const result=publicationFrameGeometrySchema.safeParse({...numbers,fitMode:values.fitMode});return result.success?result.data:undefined;
}
