import { expect,it } from 'vitest';
import { publicationActionSchema } from '@betweenframes/contracts';
import { continuousFixture } from '../test/publicationContinuousFixture';
import { publicationStoredTextDirection } from './publicationTypography';
import { publicationTextValues,publicationTextStyle } from '../features/publishing/publicationText';
import { checkPublicationScope,publicationLayoutSchema } from '../services/publicationSchemas';
import { userFacingError } from '../services/userFacingError';
it('旧文字没有方向字段时仍保持缺字段，预览和请求不补横排默认值',()=>{
 const {layout}=continuousFixture(),page=layout.pages[1],text={id:crypto.randomUUID(),xMm:20,yMm:30,widthMm:8,heightMm:160,contentJson:'{"kind":"text","text":"Night Walk"}',styleJson:'{"fontFamily":"serif","fontSizePt":10}'};
 const values=publicationTextValues(page,text,text.id,'text');expect(values).not.toHaveProperty('textDirection');expect(publicationTextStyle(values)).not.toHaveProperty('textDirection');
 const request=publicationActionSchema.parse({type:'UpdateText',data:values});expect(request.data).not.toHaveProperty('textDirection');expect(publicationStoredTextDirection(null)).toBeUndefined();
});
it('五种排字方向往返保持物理矩形与原文，明确横排可覆盖已保存的竖排',()=>{
 const {layout}=continuousFixture(),page=layout.pages[1];
 for(const textDirection of ['horizontal','vertical-rl','vertical-lr','rotate-90','rotate-270'] as const){
  const text={id:crypto.randomUUID(),xMm:20,yMm:30,widthMm:8,heightMm:160,contentJson:'{"kind":"text","text":"夜行 <b>Night Walk</b>"}',styleJson:JSON.stringify({textDirection,fontSizePt:10})};
  const values=publicationTextValues(page,text,text.id,'text');expect(values).toMatchObject({xMm:20,yMm:30,widthMm:8,heightMm:160,text:'夜行 <b>Night Walk</b>',textDirection});expect(publicationActionSchema.safeParse({type:'UpdateText',data:values}).success).toBe(true);expect(publicationStoredTextDirection(JSON.stringify(publicationTextStyle(values)))).toBe(textDirection);
 }
});
it('损坏或不支持的方向不伪装横排，也不向用户暴露内部字段',()=>{
 const {layout}=continuousFixture(),id=crypto.randomUUID(),page=layout.pages[1];page.texts.push({id,xMm:20,yMm:30,widthMm:8,heightMm:160,contentJson:'{"kind":"text","text":"夜行"}',styleJson:'{"textDirection":"diagonal"}'});
 expect(()=>checkPublicationScope(publicationLayoutSchema.parse(layout),layout.publication.projectId,layout.publication.id)).toThrow();
 for(const textDirection of [null,'diagonal',90,{}])expect(()=>publicationStoredTextDirection(JSON.stringify({textDirection}))).toThrow();
 let error:unknown;try{publicationStoredTextDirection('{"textDirection":null}');}catch(value){error=value;}expect(userFacingError(error,'无法读取这份版面。')).toBe('无法读取这份版面。');
});
