import type { PublicationLayout } from '../../domain/publication';
import { publicationPageCover } from '../../domain/publicationCover';
import { zh } from '../../app/copy';
export function PublicationCoverOverlay({page,thumbnail=false}:{page:PublicationLayout['pages'][number];thumbnail?:boolean}){
  const cover=publicationPageCover(page);if(!cover)return null;
  const split=cover.panelWidthMm/page.widthMm*100,end=(cover.panelWidthMm+cover.spineWidthMm)/page.widthMm*100;
  return <div className="publication-cover-overlay" aria-hidden="true">
    <i style={{left:split+'%'}}/>{cover.spineWidthMm>0&&<i style={{left:end+'%'}}/>}
    {!thumbnail&&<><span style={{left:split/2+'%'}}>{zh.publicationBackCover}</span>{cover.spineWidthMm>0&&<span className="publication-cover-spine-caption" style={{left:(split+end)/2+'%'}}>{zh.publicationCoverSpine}</span>}<span style={{left:(end+100)/2+'%'}}>{zh.publicationCover}</span></>}
  </div>;
}
