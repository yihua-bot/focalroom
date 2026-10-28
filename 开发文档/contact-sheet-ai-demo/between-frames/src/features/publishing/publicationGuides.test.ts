import { expect,it } from 'vitest';
import { publicationDefaultGuideView,publicationPageGuides,publicationGuidesPreview,publicationGuideLines,publicationRulerTicks,publicationSnapTargets } from './publicationGuides';
const page={widthMm:200,heightMm:300,marginMm:20,layoutJson:'{"legacy":{"preserve":[1,2]}}'};
it('网格与参考线先做不可变预览，原页面扩展与正式对象不改写',()=>{
  const before=structuredClone(page),settings={...publicationPageGuides(page),columns:4,verticalGuidesMm:[20.25,100],horizontalGuidesMm:[30]},layoutJson=publicationGuidesPreview(page,settings);
  expect(page).toEqual(before);expect(JSON.parse(layoutJson).legacy).toEqual({preserve:[1,2]});expect(publicationPageGuides({...page,layoutJson})).toEqual(settings);
  const lines=publicationGuideLines({...page,layoutJson},{grid:true,margins:true,guides:true});expect(lines.custom.x).toEqual([20.25,100]);expect(lines.margins.x).toEqual([20,180]);expect(lines.grid.x).toHaveLength(8);expect(lines.grid.x.every(x=>x>=20&&x<=180)).toBe(true);
});
it('非法或过密网格、重复/越界参考线、未知字段和损坏元数据均拒绝',()=>{
  const valid=publicationPageGuides(page);
  for(const change of [{columns:0},{rows:1.5},{columnGapMm:100},{verticalGuidesMm:[10,10]},{horizontalGuidesMm:[301]},{verticalGuidesMm:Array.from({length:33},(_,i)=>i)},{columnGapMm:NaN},{source:'foreign'},{horizontalGuidesMm:[-1]}])expect(()=>publicationGuidesPreview(page,{...valid,...change})).toThrow();
  for(const layoutJson of ['null','[]','{','{"focalroomPageGuidesV1":null}'])expect(()=>publicationPageGuides({...page,layoutJson})).toThrow();
});
it('隐藏网格不成为网格吸附目标，页面边缘与可见对象仍可参考',()=>{
  const targets=publicationSnapTargets(page,{...publicationDefaultGuideView,grid:false,margins:false,guides:false},[{xMm:30,yMm:40,widthMm:50,heightMm:60}]);expect(targets.x).toEqual([0,100,200,30,55,80]);expect(targets.y).toEqual([0,150,300,40,70,100]);
});
it('毫米刻度随屏幕宽度改变密度，不改变页面范围且标签不过密',()=>{
  for(const length of [50,190,257,400,2000])for(const pixels of [80,140,500,4000]){const ticks=publicationRulerTicks(length,pixels),major=ticks.filter(tick=>tick.major);expect(ticks.length).toBeLessThan(1000);expect(ticks.every(tick=>tick.mm>=0&&tick.mm<=length+1e-8)).toBe(true);for(let i=1;i<major.length;i++)expect((major[i].mm-major[i-1].mm)/length*pixels).toBeGreaterThanOrEqual(48-1e-6);}
  expect(publicationRulerTicks(NaN,200)).toEqual([]);
});

it('既有大块未知页面元数据仍可读取，添加参考线不能突破新设置容量或悄悄截断',()=>{
 const legacy={...page,layoutJson:JSON.stringify({legacy:'汉'.repeat(20000)})},before=legacy.layoutJson;expect(publicationPageGuides(legacy)).toEqual(publicationPageGuides(page));expect(()=>publicationGuidesPreview(legacy,publicationPageGuides(legacy))).toThrow();expect(legacy.layoutJson).toBe(before);
 expect(()=>publicationPageGuides({...page,layoutJson:JSON.stringify({legacy:'x'.repeat(32768),focalroomPageGuidesV1:publicationPageGuides(page)})})).toThrow();
 expect(()=>publicationPageGuides({...page,marginMm:100})).toThrow();
});
