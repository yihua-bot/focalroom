import type { PublicationLayout } from '../../domain/publication';
import { publicationContinuousPairs } from '../../domain/publicationContinuous';
type Page=PublicationLayout['pages'][number];
/** 与原生 ReorderPages 相同的完整身份与封面边界；预览不重建页面内容。 */
export function publicationPageOrderPreview(pages:Page[],ids:string[]):Page[]|undefined{
  const byId=new Map(pages.map(page=>[page.id,page]));
  if(ids.length!==pages.length||byId.size!==pages.length||new Set(ids).size!==ids.length||ids.some(id=>!byId.has(id)))return;
  const cover=pages.find(page=>page.pageNumber===0),back=pages.find(page=>page.pageNumber===-1);
  if(cover&&ids[0]!==cover.id||back&&ids[ids.length-1]!==back.id)return;
  let number=1;const preview=ids.map(id=>{const page=byId.get(id)!;if(page.pageNumber===0||page.pageNumber===-1)return page;const pageNumber=number++;return {...page,pageNumber,side:page.side==='spread'?'spread':pageNumber%2?'right':'left'};});
  try{publicationContinuousPairs(preview);return preview;}catch{return;}
}
export function publicationMovePage(pages:Page[],fromId:string,toId:string):string[]|undefined{
  const from=pages.findIndex(page=>page.id===fromId),to=pages.findIndex(page=>page.id===toId);
  if(from<0||to<0||from===to||pages[from].pageNumber<=0||pages[to].pageNumber<=0)return;
  const paired=publicationContinuousPairs(pages).find(item=>item.left.id===fromId||item.right.id===fromId);
  if(paired){const targetNumber=pages[to].pageNumber%2?pages[to].pageNumber-1:pages[to].pageNumber,target=pages.findIndex(page=>page.pageNumber===targetNumber);if(target<0||targetNumber<2||!pages[target+1]||pages[target+1].pageNumber!==targetNumber+1)return;const first=pages.findIndex(page=>page.id===paired.left.id);if(first===target)return;const ids=pages.filter(page=>page.id!==paired.left.id&&page.id!==paired.right.id).map(page=>page.id);ids.splice(target,0,paired.left.id,paired.right.id);return publicationPageOrderPreview(pages,ids)?ids:undefined;}
  const ids=pages.map(page=>page.id);ids.splice(to,0,ids.splice(from,1)[0]);return publicationPageOrderPreview(pages,ids)?ids:undefined;
}

/** 插页先计算物理配对；只为界面判断可选数量，不写入或生成正式对象。 */
export function publicationInsertionAllowed(pages:Page[],afterId:string|null,count:1|2):boolean{
  if(!pages.length)return true;
  const after=afterId?pages.findIndex(page=>page.id===afterId):-1;
  const back=pages.findIndex(page=>page.pageNumber===-1);
  const at=afterId?after+1:back>=0?back:pages.length;if(afterId&&after<0)return false;
  const draft=pages.slice(),base=pages.find(page=>page.pageNumber>0)??pages[0];
  draft.splice(at,0,...Array.from({length:count},(_,index)=>({...base,id:'__new-page-'+index,pageNumber:1,side:'right',layoutJson:null,frames:[],texts:[]})));
  return !!publicationPageOrderPreview(draft,draft.map(page=>page.id));
}
