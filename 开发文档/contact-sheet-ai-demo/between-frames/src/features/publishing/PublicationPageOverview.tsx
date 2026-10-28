import type { PublicationLayout } from '../../domain/publication';
import { Button } from '../../components/shared/Button';
import { zh } from '../../app/copy';
import { publicationPageName } from './publicationSpreads';

type Page=PublicationLayout['pages'][number];
/** 未选中对象时提供页面概览和创作入口，不展示空白的对象属性表。 */
export function PublicationPageOverview({page,photoCount,emptyFrames,textCount,disabled,onPhotos,onText,onSettings}:{
  page:Page;photoCount:number;emptyFrames:number;textCount:number;disabled:boolean;
  onPhotos?:()=>void;onText?:()=>void;onSettings?:()=>void;
}){
  return <section className="publication-page-overview" aria-label={zh.publicationPageOverview}>
    <strong>{publicationPageName(page.pageNumber)}</strong>
    <p>{page.widthMm} × {page.heightMm} mm</p>
    <dl>
      <div><dt>{zh.publicationOverviewPhotos}</dt><dd>{photoCount}</dd></div>
      <div><dt>{zh.publicationOverviewTexts}</dt><dd>{textCount}</dd></div>
      {emptyFrames>0&&<div><dt>{zh.publicationEmptyFrames}</dt><dd>{emptyFrames}</dd></div>}
    </dl>
    <p>{photoCount+emptyFrames+textCount===0?zh.publicationEmptyPageHint:zh.publicationSelectObjectHint}</p>
    <div className="publication-page-overview-actions">
      {onPhotos&&<Button size="small" variant={photoCount+emptyFrames+textCount===0?'primary':'default'} disabled={disabled} onClick={onPhotos}>{zh.publicationChoosePagePhotos}</Button>}
      {onText&&<Button size="small" disabled={disabled} onClick={onText}>{zh.publicationWritePageText}</Button>}
      {onSettings&&<Button size="small" disabled={disabled} onClick={onSettings}>{zh.publicationAdjustPage}</Button>}
    </div>
  </section>;
}
