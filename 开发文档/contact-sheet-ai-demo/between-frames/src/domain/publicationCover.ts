import { PUBLICATION_COVER_KEY,publicationCoverSchema,type PublicationCover } from '@betweenframes/contracts';
import type { PublicationLayout } from './publication';
type Page=PublicationLayout['pages'][number];
const invalid=()=>Object.assign(new Error(),{code:'PUBLICATION_COVER_INVALID'});
export function publicationPageCover(page:Page):PublicationCover|undefined{
  const metadata=JSON.parse(page.layoutJson??'{}');
  if(!metadata||typeof metadata!=='object'||Array.isArray(metadata))throw invalid();
  if(!Object.prototype.hasOwnProperty.call(metadata,PUBLICATION_COVER_KEY))return undefined;
  const cover=publicationCoverSchema.parse(metadata[PUBLICATION_COVER_KEY]);
  if(page.pageNumber!==0||page.side!=='cover'||Math.abs(page.widthMm-cover.panelWidthMm*2-cover.spineWidthMm)>1e-6)throw invalid();
  return cover;
}
export function checkPublicationCover(layout:PublicationLayout){
  const covers=layout.pages.filter(page=>publicationPageCover(page));
  if(covers.length&&(layout.publication.pubType!=='photobook'||covers.length!==1||layout.pages.some(page=>page.pageNumber===-1)))throw invalid();
}
