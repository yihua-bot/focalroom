import { PUBLICATION_GUIDES_KEY,publicationPageGuidesSchema,publicationGuidesFitPage,type PublicationPageGuides } from '@betweenframes/contracts';
import type { PublicationLayout } from './publication';
export type { PublicationPageGuides };
export type GuidePage=Pick<PublicationLayout['pages'][number],'widthMm'|'heightMm'|'marginMm'|'layoutJson'>;
export class PublicationGuidesError extends Error {readonly code='PUBLICATION_GUIDES_INVALID';constructor(readonly reason:'invalid'|'capacity'='invalid'){super('');}}
const invalid=(reason:'invalid'|'capacity'='invalid'):never=>{throw new PublicationGuidesError(reason);};
function metadata(page:GuidePage){const raw=page.layoutJson??'{}';if(raw.length>8*1024*1024)return invalid();const bytes=new TextEncoder().encode(raw).length;if(bytes>8*1024*1024)return invalid();let value:Record<string,unknown>;try{value=JSON.parse(raw);}catch{return invalid();}if(!value||typeof value!=='object'||Array.isArray(value))return invalid();if(PUBLICATION_GUIDES_KEY in value&&bytes>32768)return invalid();return value;}
function validateBounds(page:GuidePage){if(![page.widthMm,page.heightMm,page.marginMm].every(Number.isFinite)||page.widthMm<=0||page.heightMm<=0||page.marginMm<0||page.marginMm*2>=Math.min(page.widthMm,page.heightMm))return invalid();}
export function publicationValidateGuides(page:GuidePage,settings:PublicationPageGuides){
  validateBounds(page);
  if(!publicationGuidesFitPage(page,settings))return invalid();
}
export function publicationStoredPageGuides(page:GuidePage):PublicationPageGuides|undefined{
  validateBounds(page);const raw=metadata(page)[PUBLICATION_GUIDES_KEY];if(raw===undefined)return;
  const parsed=publicationPageGuidesSchema.safeParse(raw);if(!parsed.success)return invalid();publicationValidateGuides(page,parsed.data);return parsed.data;
}
export function publicationPageGuides(page:GuidePage):PublicationPageGuides{return publicationStoredPageGuides(page)??{columns:3,rows:3,columnGapMm:Math.min(5,(page.widthMm-2*page.marginMm)/4),rowGapMm:Math.min(5,(page.heightMm-2*page.marginMm)/4),verticalGuidesMm:[],horizontalGuidesMm:[]};}
export function publicationGuidesPreview(page:GuidePage,settings:PublicationPageGuides){const checked=publicationPageGuidesSchema.parse(settings);publicationValidateGuides(page,checked);const value=metadata(page);value[PUBLICATION_GUIDES_KEY]=checked;const layoutJson=JSON.stringify(value);if(new TextEncoder().encode(layoutJson).length>32768)return invalid('capacity');return layoutJson;}
