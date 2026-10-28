import type { PublicationLayout } from '../../domain/publication';
import { zh } from '../../app/copy';
type Page=PublicationLayout['pages'][number];
export function publicationPageName(number:number){return number===0?zh.publicationCover:number===-1?zh.publicationBackCover:zh.publicationPageNumber.replace('{number}',String(number));}
/** 依照当前版本的物理左右页配对；封面、封底和整幅跨页保持独立。 */
export function publicationSpread(pages:Page[],active:Page):Array<Page|null>{
  const index=pages.findIndex(page=>page.id===active.id);
  if(active.side==='cover'||active.side==='back'||active.side==='spread'||index<0)return [active];
  if(active.side==='left'){
    const next=pages[index+1];return [active,next?.side==='right'&&next.pageNumber===active.pageNumber+1?next:null];
  }
  const previous=pages[index-1];return [previous?.side==='left'&&previous.pageNumber+1===active.pageNumber?previous:null,active];
}
/** 翻动当前可见的一组页面；空白占位不计入书页，也不会重复停在同一跨页。 */
export function publicationAdjacentPage(pages:Page[],active:Page,spread:boolean,direction:-1|1):Page|undefined{
  const index=pages.findIndex(page=>page.id===active.id);
  if(index<0)return undefined;
  const visible=spread?publicationSpread(pages,active).filter((page):page is Page=>page!==null):[active];
  const indices=visible.map(page=>pages.findIndex(value=>value.id===page.id));
  return pages[(direction<0?Math.min(...indices):Math.max(...indices))+direction];
}
