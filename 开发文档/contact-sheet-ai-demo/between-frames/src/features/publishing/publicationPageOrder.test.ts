import { expect,it } from 'vitest';
import type { PublicationLayout } from '../../domain/publication';
import { publicationMovePage,publicationPageOrderPreview,publicationInsertionAllowed } from './publicationPageOrder';
const pages:PublicationLayout['pages']=Array.from({length:6},(_,index)=>({id:'page-'+index,pageNumber:index===5?-1:index,side:index===0?'cover':index===5?'back':index===2?'spread':index%2?'right':'left',widthMm:210,heightMm:297,marginMm:15,bleedMm:3,layoutJson:'{"chapter":"保留"}',frames:[],texts:[{id:'text-'+index,xMm:15,yMm:20,widthMm:30,heightMm:10,contentJson:'{"kind":"page_number"}',styleJson:null}]}));
it('完整身份排序只改变顺序与页码/左右关系，保留内容和spread身份，不修改原版',()=>{
  const before=structuredClone(pages),ids=publicationMovePage(pages,'page-4','page-1')!,preview=publicationPageOrderPreview(pages,ids)!;expect(ids).toEqual(['page-0','page-4','page-1','page-2','page-3','page-5']);expect(preview.map(page=>page.pageNumber)).toEqual([0,1,2,3,4,-1]);expect(preview.map(page=>page.side)).toEqual(['cover','right','left','spread','left','back']);for(const page of preview){const old=pages.find(value=>value.id===page.id)!;expect({...page,pageNumber:old.pageNumber,side:old.side}).toEqual(old);}expect(pages).toEqual(before);
});
it('封面封底不可拖入内页，缺失/重复/外部页面不能以半本书保存，跨缩略图批次仍按全部身份排序',()=>{
  const ids=pages.map(page=>page.id);for(const invalid of [ids.slice(1),[ids[1],ids[0],...ids.slice(2)],[...ids.slice(0,-2),ids[5],ids[4]],[...ids.slice(0,-1),'foreign'],ids.map(()=>ids[0])])expect(publicationPageOrderPreview(pages,invalid)).toBeUndefined();expect(publicationMovePage(pages,'page-0','page-1')).toBeUndefined();expect(publicationMovePage(pages,'page-1','page-5')).toBeUndefined();const many=Array.from({length:85},(_,index)=>({...pages[1],id:'p'+index,pageNumber:index+1}));const moved=publicationMovePage(many,'p2','p80')!;expect(moved).toHaveLength(85);expect(moved[80]).toBe('p2');expect(new Set(moved).size).toBe(85);
});

it('连续照片所在两页整组排序，错误的单页交换与插页在预览阶段被拒绝',()=>{
  const list:PublicationLayout['pages']=Array.from({length:6},(_,i)=>({...pages[1],id:crypto.randomUUID(),pageNumber:i,side:i===0?'cover':i%2?'right':'left',layoutJson:null,frames:[],texts:[]}));
  const asset='photo',frame={id:crypto.randomUUID(),xMm:15,yMm:15,widthMm:390,heightMm:260,fitMode:'fill' as const,assetId:asset,renditionId:null,cropX:.5,cropY:.5,cropZoom:1,source:{assetId:asset,renditionId:null,sourceSha256:'a'.repeat(64),width:6000,height:4000,sourceProfileDigest:null,variantId:null,renditionSha256:null,renditionWidth:null,renditionHeight:null,bitDepth:null,iccDigest:null,previewSha256:null}};
  list[2].frames=[frame];list[2].layoutJson=JSON.stringify({focalroomContinuousFramesV1:[{frameId:frame.id,rightPageId:list[3].id}]});
  const moved=publicationMovePage(list,list[3].id,list[4].id)!;expect(moved).toEqual([list[0].id,list[1].id,list[4].id,list[5].id,list[2].id,list[3].id]);expect(publicationMovePage(list,list[2].id,list[5].id)).toEqual(moved);expect(publicationMovePage(publicationPageOrderPreview(list,moved)!,list[3].id,list[4].id)).toEqual(list.map(page=>page.id));
  const invalid=list.map(page=>page.id);[invalid[1],invalid[2]]=[invalid[2],invalid[1]];expect(publicationPageOrderPreview(list,invalid)).toBeUndefined();expect(publicationMovePage(list,list[2].id,list[1].id)).toBeUndefined();expect(publicationInsertionAllowed(list,list[1].id,1)).toBe(false);expect(publicationInsertionAllowed(list,list[1].id,2)).toBe(true);expect(publicationInsertionAllowed(list,list[3].id,1)).toBe(true);
});
