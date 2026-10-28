import { publicationTextDirectionSchema,type PublicationTextDirection } from '@betweenframes/contracts';
export function publicationStoredTextDirection(raw:string|null|undefined):PublicationTextDirection|undefined{
  const style:unknown=JSON.parse(raw??'{}');
  if(!style||typeof style!=='object'||Array.isArray(style))throw Object.assign(new Error(),{code:'PUBLICATION_TEXT_STYLE_INVALID'});
  return Object.prototype.hasOwnProperty.call(style,'textDirection')?publicationTextDirectionSchema.parse((style as Record<string,unknown>).textDirection):undefined;
}
