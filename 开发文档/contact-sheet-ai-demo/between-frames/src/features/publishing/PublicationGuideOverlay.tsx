import { zh } from '../../app/copy';
import { publicationGuideLines,publicationRulerTicks,type GuidePage,type PublicationGuideView } from './publicationGuides';
import type { PublicationSnapLines } from './publicationSnap';

export function PublicationGuideOverlay({page,view,snap}:{page:GuidePage;view:PublicationGuideView;snap:PublicationSnapLines}){
  const lines=publicationGuideLines(page,view);
  return <svg className="publication-guide-overlay" aria-hidden="true" viewBox={`0 0 ${page.widthMm} ${page.heightMm}`} preserveAspectRatio="none">
    {(['grid','margins','custom'] as const).map(kind=><g key={kind} className={'publication-guide-'+kind}>{lines[kind].x.map(x=><line key={'x'+x} data-guide-axis="x" data-guide-mm={x} x1={x} x2={x} y1={0} y2={page.heightMm}/>)}{lines[kind].y.map(y=><line key={'y'+y} data-guide-axis="y" data-guide-mm={y} x1={0} x2={page.widthMm} y1={y} y2={y}/>)}</g>)}
    <g className="publication-guide-snap">{snap.x.map(x=><line key={'x'+x} x1={x} x2={x} y1={0} y2={page.heightMm}/>)}{snap.y.map(y=><line key={'y'+y} x1={0} x2={page.widthMm} y1={y} y2={y}/>)}</g>
  </svg>;
}
export function PublicationRulers({page,width}:{page:GuidePage;width:number}){
  const height=width*page.heightMm/page.widthMm;
  return <div className="publication-rulers" role="img" aria-label={zh.publicationRulerDescription.replace('{width}',String(page.widthMm)).replace('{height}',String(page.heightMm))}>
    <span className="publication-ruler-unit" aria-hidden="true">mm</span>
    <svg className="publication-ruler-horizontal" aria-hidden="true" viewBox={`0 0 ${width} 20`} preserveAspectRatio="none">{publicationRulerTicks(page.widthMm,width).map(tick=>{const x=tick.mm/page.widthMm*width;return <g key={tick.mm}><line x1={x} x2={x} y1={tick.major?13:17} y2={20}/>{tick.major&&<text x={x+(tick.mm===0?3:-2)} y={10} textAnchor={tick.mm===0?'start':'end'}>{tick.mm}</text>}</g>;})}</svg>
    <svg className="publication-ruler-vertical" aria-hidden="true" viewBox={`0 0 20 ${height}`} preserveAspectRatio="none">{publicationRulerTicks(page.heightMm,height).map(tick=>{const y=tick.mm/page.heightMm*height;return <g key={tick.mm}><line x1={tick.major?13:17} x2={20} y1={y} y2={y}/>{tick.major&&<text transform={`translate(10 ${y+(tick.mm===0?3:-2)}) rotate(-90)`} textAnchor={tick.mm===0?'end':'start'}>{tick.mm}</text>}</g>;})}</svg>
  </div>;
}
