import { expect,it } from 'vitest';
import { publicationActionSchema } from '@betweenframes/contracts';
import { continuousFixture } from '../../test/publicationContinuousFixture';
import { publicationChapters,publicationChapterRanges,publicationChapterChanges,PUBLICATION_CHAPTERS_KEY } from '../../domain/publicationChapters';
import { publicationLinkPreview } from '../../domain/publicationContinuous';
import { publicationChaptersPreview,publicationChapterOpening,publicationChapterOpeningPreview,publicationMoveChapter } from './publicationChapterPlans';
import { publicationPageOrderPreview } from './publicationPageOrder';
import { publicationLayers } from '../../domain/publicationLayers';
function fixture(){
 const {layout}=continuousFixture();const base=layout.pages[0];
 layout.pages.push(...[4,5,6].map(number=>({...base,id:crypto.randomUUID(),pageNumber:number,side:number%2?'right':'left',frames:[],texts:[]})));
 const chapters=[{id:crypto.randomUUID(),title:'夜行',startPageId:layout.pages[1].id},{id:crypto.randomUUID(),title:'归途',startPageId:layout.pages[3].id}];
 return {layout,chapters};
}
it('章节按稳定起始页划范围，整章移动保留全部来源与对象，不改原布局',()=>{
 const {layout,chapters}=fixture(),before=structuredClone(layout),saved=publicationChaptersPreview(layout,{chapters,pageIds:layout.pages.map(p=>p.id)});
 expect(publicationChapterRanges(saved).map(c=>c.pages.length)).toEqual([2,4]);expect(saved.schemaVersion).toBe(3);expect(layout).toEqual(before);
 const ids=publicationMoveChapter(saved,chapters[1].id,-1)!;expect(ids).toBeTruthy();
 const moved=publicationChaptersPreview(saved,{chapters,pageIds:ids});expect(publicationChapters(moved)[0].id).toBe(chapters[1].id);expect(publicationChapterChanges(saved,moved.pages)).toEqual([]);
 for(const p of layout.pages){const actual=moved.pages.find(n=>n.id===p.id)!;expect(actual.frames).toEqual(p.frames);expect(actual.texts).toEqual(p.texts);}
});
it('单页排序给出明确归属变化，重复/外页/封面起始或连续照片右页均拒绝',()=>{
 const {layout,chapters}=fixture(),saved=publicationChaptersPreview(layout,{chapters,pageIds:layout.pages.map(p=>p.id)}),ids=saved.pages.map(p=>p.id);[ids[2],ids[3]]=[ids[3],ids[2]];
 const pages=publicationPageOrderPreview(saved.pages,ids)!;expect(publicationChapterChanges(saved,pages).map(c=>c.pageId)).toEqual([saved.pages[2].id]);
 for(const changed of [[chapters[0],{...chapters[1],startPageId:chapters[0].startPageId}],[{...chapters[0],startPageId:layout.pages[0].id}],[{...chapters[0],startPageId:crypto.randomUUID()}],[{...chapters[0],id:layout.pages[1].frames[0].id}]]){
  expect(()=>publicationChaptersPreview(layout,{chapters:changed,pageIds:layout.pages.map(p=>p.id)})).toThrow();
 }
 const f=continuousFixture(),spread=publicationLinkPreview(f.layout,f.data);
 expect(()=>publicationChaptersPreview(spread,{chapters:[{...chapters[0],startPageId:f.layout.pages[3].id}],pageIds:f.layout.pages.map(p=>p.id)})).toThrow();
});
it('实际章节页预览和请求相同：右页标题、空白陪页、层级和原图保持',()=>{
 const {layout}=fixture(),before=structuredClone(layout);
 const data=publicationChapterOpening(layout,{title:'  第三章  ',afterPageId:layout.pages[1].id,count:2,startOnRight:true,chapterId:crypto.randomUUID(),pageId:crypto.randomUUID(),textId:crypto.randomUUID(),companionId:crypto.randomUUID()});
 expect(publicationActionSchema.parse({type:'CreateChapter',data})).toEqual({type:'CreateChapter',data});
 const preview=publicationChapterOpeningPreview(layout,data),opening=preview.pages.find(p=>p.id===data.chapter.startPageId)!;
 expect(opening.pageNumber).toBe(3);expect(opening.side).toBe('right');expect(JSON.parse(opening.texts[0].contentJson).text).toBe('第三章');
 expect(publicationLayers(preview).nodes.filter(n=>n.id===data.title.textId)).toHaveLength(1);
 expect(preview.pages[2].texts).toEqual([]);expect(preview.pages.length).toBe(layout.pages.length+2);expect(layout).toEqual(before);
 for(const p of layout.pages)expect(preview.pages.find(n=>n.id===p.id)!.frames).toEqual(p.frames);
 const varied=structuredClone(layout);varied.pages[varied.pages.length-1].widthMm=200;varied.pages[varied.pages.length-1].heightMm=280;
 const atEnd=publicationChapterOpening(varied,{title:'尾章',afterPageId:null,count:2,startOnRight:true,chapterId:crypto.randomUUID(),pageId:crypto.randomUUID(),textId:crypto.randomUUID(),companionId:crypto.randomUUID()});
 expect(atEnd.pages.every(p=>p.widthMm===200&&p.heightMm===280)).toBe(true);
 expect(()=>publicationChapterOpening(layout,{title:'章节',afterPageId:layout.pages[1].id,count:1,startOnRight:true,chapterId:crypto.randomUUID(),pageId:crypto.randomUUID(),textId:crypto.randomUUID(),companionId:crypto.randomUUID()})).toThrow();
});
it('旧页序请求不增加确认字段，明确null和损坏章节不可被默默接受',()=>{
 const ids=[crypto.randomUUID(),crypto.randomUUID()];const original={type:'ReorderPages',data:{pageIds:ids}};
 expect(publicationActionSchema.parse(original)).toEqual(original);
 expect(publicationActionSchema.safeParse({type:'ReorderPages',data:{pageIds:ids,chapterChangesConfirmed:null}}).success).toBe(false);
 const {layout,chapters}=fixture(),saved=publicationChaptersPreview(layout,{chapters,pageIds:layout.pages.map(p=>p.id)});
 for(const value of [null,{chapters},[{...chapters[0],title:'\n'}],[{...chapters[0],title:'\u0085'}],[{...chapters[0],title:'\ufeff'}],[{...chapters[0],extra:true}]]){
  const spec={...JSON.parse(saved.publication.specJson),[PUBLICATION_CHAPTERS_KEY]:value};
  expect(()=>publicationChapters({...saved,publication:{...saved.publication,specJson:JSON.stringify(spec)}})).toThrow();
 }
 expect(publicationChapters(layout)).toEqual([]);
});
