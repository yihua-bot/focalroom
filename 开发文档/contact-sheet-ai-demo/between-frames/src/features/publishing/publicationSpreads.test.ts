import { expect,it } from 'vitest';
import type { PublicationLayout } from '../../domain/publication';
import { publicationSpread,publicationPageName,publicationAdjacentPage } from './publicationSpreads';
import { setCopyLocale } from '../../app/copy';
type Page=PublicationLayout['pages'][number];
const p=(number:number,side:string):Page=>({id:String(number),pageNumber:number,side,widthMm:190,heightMm:257,marginMm:15,bleedMm:3,layoutJson:null,frames:[],texts:[]});
it('左右内页按当前版本页序配对，封面封底和整幅跨页独立',()=>{
  const pages=[p(0,'cover'),p(1,'right'),p(2,'left'),p(3,'right'),p(4,'spread'),p(-1,'back')];
  expect(publicationSpread(pages,pages[0])).toEqual([pages[0]]);expect(publicationSpread(pages,pages[1])).toEqual([null,pages[1]]);
  expect(publicationSpread(pages,pages[2])).toEqual([pages[2],pages[3]]);expect(publicationSpread(pages,pages[3])).toEqual([pages[2],pages[3]]);
  expect(publicationSpread(pages,pages[4])).toEqual([pages[4]]);expect(publicationSpread(pages,pages[5])).toEqual([pages[5]]);
});
it('缺页或不连续页码不会配上其他照片，也不创建虚构页面',()=>{
  const pages=[p(2,'left'),p(5,'right')];expect(publicationSpread(pages,pages[0])).toEqual([pages[0],null]);expect(publicationSpread(pages,pages[1])).toEqual([null,pages[1]]);
  setCopyLocale('zh');expect(publicationPageName(-1)).toBe('封底');
});
it('跨页逐组前进后退，封面、缺页和整幅跨页均不会重复停留',()=>{
  const pages=[p(0,'cover'),p(1,'right'),p(2,'left'),p(3,'right'),p(4,'left'),p(5,'right'),p(6,'spread'),p(9,'right'),p(-1,'back')];
  expect(publicationAdjacentPage(pages,pages[0],true,-1)).toBeUndefined();
  expect(publicationAdjacentPage(pages,pages[0],true,1)).toBe(pages[1]);
  for(const active of [pages[2],pages[3]]){
    expect(publicationAdjacentPage(pages,active,true,1)).toBe(pages[4]);
    expect(publicationAdjacentPage(pages,active,true,-1)).toBe(pages[1]);
  }
  expect(publicationAdjacentPage(pages,pages[4],true,-1)).toBe(pages[3]);
  expect(publicationAdjacentPage(pages,pages[5],true,1)).toBe(pages[6]);
  expect(publicationAdjacentPage(pages,pages[6],true,1)).toBe(pages[7]);
  expect(publicationAdjacentPage(pages,pages[7],true,1)).toBe(pages[8]);
  expect(publicationAdjacentPage(pages,pages[8],true,1)).toBeUndefined();
});
it('单页逐页移动，历史页不在当前版时不借相同页码跳页',()=>{
  const pages=[p(2,'left'),p(3,'right')];
  expect(publicationAdjacentPage(pages,pages[0],false,1)).toBe(pages[1]);
  expect(publicationAdjacentPage(pages,{...pages[0],id:'historical'},true,1)).toBeUndefined();
  expect(publicationAdjacentPage([],pages[0],false,1)).toBeUndefined();
});
