import { expect,it } from 'vitest';
import { continuousFixture } from '../test/publicationContinuousFixture';
import { publicationLinkPreview } from '../domain/publicationContinuous';
import { checkPublicationScope,publicationLayoutSchema } from './publicationSchemas';
it('读取格式2在展示前验证正式对象和左右关联，不把格式1或错误配对静默降级',()=>{
  const {layout,data}=continuousFixture(),good=publicationLinkPreview(layout,data),parse=(value:unknown)=>{const result=publicationLayoutSchema.parse(value);checkPublicationScope(result,'project','book');return result;};expect(parse(good)).toEqual(good);
  const malformed=structuredClone(good);malformed.pages[3].id=crypto.randomUUID();expect(()=>parse(malformed)).toThrow();expect(()=>parse({...good,schemaVersion:1})).toThrow();expect(()=>parse({...good,schemaVersion:3})).toThrow();expect(()=>parse({...good,publication:{...good.publication,projectId:'foreign'}})).toThrow();const duplicate=structuredClone(good);duplicate.pages[3].frames=[{...duplicate.pages[2].frames.find(frame=>frame.id===data.frameId)!,id:crypto.randomUUID()}];expect(()=>parse(duplicate)).toThrow();
});
