import { beforeEach,expect,it,vi } from 'vitest';
import { PublicationCommandClient } from './PublicationCommandClient';
import type { PublicationCommandRequest } from '@betweenframes/contracts';
import { MemoryStorage } from '../test/memoryStorage';
const localStorage=new MemoryStorage();
const project='demo-first-walk',publication=crypto.randomUUID();
const actions=[{type:'CaptureVersion' as const,data:{label:'保留这版'}}];
const receipt=(request:PublicationCommandRequest)=>({commandId:request.commandId,projectId:request.projectId,publicationId:request.publicationId,baseRevision:request.baseRevision,revision:request.baseRevision+1,changed:true,results:[null]});
beforeEach(()=>localStorage.clear());
it('断线后先存完整请求，重开只确认原命令，不创建第二份版本',async()=>{
  const send=vi.fn<(request:PublicationCommandRequest)=>Promise<unknown>>().mockRejectedValueOnce(new Error('closed after commit')).mockImplementation(async request=>receipt(request));
  const client=new PublicationCommandClient(send,localStorage);await expect(client.execute(project,publication,2,actions)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});await expect(client.execute(project,publication,2,actions)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});expect(send).toHaveBeenCalledTimes(1);
  const reopened=new PublicationCommandClient(send,localStorage);expect(reopened.pending(project)).toHaveLength(1);await expect(reopened.retry('wrong-project',publication)).rejects.toThrow();expect(send).toHaveBeenCalledTimes(1);await reopened.retry(project,publication);expect(send.mock.calls[1][0]).toEqual(send.mock.calls[0][0]);expect(reopened.pending(project)).toEqual([]);
});
it('错误回执保留待确认，明确版本冲突则结束旧请求而不自动重发',async()=>{
  const send=vi.fn(async(request:PublicationCommandRequest)=>({...receipt(request),publicationId:crypto.randomUUID()}));const client=new PublicationCommandClient(send,localStorage);await expect(client.execute(project,publication,2,actions)).rejects.toMatchObject({code:'PUBLICATION_PENDING'});expect(client.pending(project)).toHaveLength(1);
  const explicit=vi.fn().mockRejectedValue(Object.assign(new Error('changed'),{code:'REVISION_CONFLICT'}));const reopened=new PublicationCommandClient(explicit,localStorage);await expect(reopened.retry(project,publication)).rejects.toMatchObject({code:'REVISION_CONFLICT'});expect(explicit).toHaveBeenCalledTimes(1);expect(reopened.pending(project)).toEqual([]);
});

it('章节页保存中断后保留原页面/标题/章节编号，重开只确认同一命令',async()=>{
  const {continuousFixture}=await import('../test/publicationContinuousFixture'),{publicationChapterOpening}=await import('../features/publishing/publicationChapterPlans'),{layout}=continuousFixture();
  const data=publicationChapterOpening(layout,{title:'归途',afterPageId:layout.pages[1].id,count:2,startOnRight:true,chapterId:crypto.randomUUID(),pageId:crypto.randomUUID(),textId:crypto.randomUUID(),companionId:crypto.randomUUID()});
  const send=vi.fn<(request:PublicationCommandRequest)=>Promise<unknown>>().mockRejectedValueOnce(new Error('lost')).mockImplementation(async request=>receipt(request));
  const first=new PublicationCommandClient(send,localStorage);await expect(first.execute(project,publication,5,[{type:'CreateChapter',data}])).rejects.toMatchObject({code:'PUBLICATION_PENDING'});
  const reopened=new PublicationCommandClient(send,localStorage);await reopened.retry(project,publication);
  expect(send.mock.calls[1][0]).toEqual(send.mock.calls[0][0]);expect(send.mock.calls[1][0].operations[0].action).toEqual({type:'CreateChapter',data});expect(reopened.pending(project)).toEqual([]);
});
it('改变章节归属的明确确认随冻结页序请求恢复，不追加到旧请求',async()=>{
  const pageIds=[crypto.randomUUID(),crypto.randomUUID()];
  const send=vi.fn<(request:PublicationCommandRequest)=>Promise<unknown>>().mockRejectedValueOnce(new Error('lost')).mockImplementation(async request=>receipt(request));
  const client=new PublicationCommandClient(send,localStorage);await expect(client.execute(project,publication,3,[{type:'ReorderPages',data:{pageIds,chapterChangesConfirmed:true}}])).rejects.toMatchObject({code:'PUBLICATION_PENDING'});
  await new PublicationCommandClient(send,localStorage).retry(project,publication);
  expect(send.mock.calls[1][0]).toEqual(send.mock.calls[0][0]);expect(send.mock.calls[1][0].operations[0].action).toEqual({type:'ReorderPages',data:{pageIds,chapterChangesConfirmed:true}});
});

it('整章移除结果未知时保留完整页面范围与空白选择，重开只确认原命令',async()=>{
 const pageIds=[crypto.randomUUID(),crypto.randomUUID(),crypto.randomUUID()],data={pageIds,chapterId:crypto.randomUUID(),retainBlankPageId:pageIds[0]};
 const send=vi.fn<(request:PublicationCommandRequest)=>Promise<unknown>>().mockRejectedValueOnce(new Error('lost')).mockImplementation(async request=>receipt(request));
 const client=new PublicationCommandClient(send,localStorage);await expect(client.execute(project,publication,5,[{type:'RemovePages',data}])).rejects.toMatchObject({code:'PUBLICATION_PENDING'});
 await new PublicationCommandClient(send,localStorage).retry(project,publication);
 expect(send.mock.calls[1][0]).toEqual(send.mock.calls[0][0]);expect(send.mock.calls[1][0].operations[0].action).toEqual({type:'RemovePages',data});expect(client.pending(project)).toEqual([]);
});

it('封面保存中断后只确认原展开页和封底范围，不重新换算或追加授权',async()=>{
 const data={pageId:crypto.randomUUID(),backPageId:null,settings:{panelWidthMm:190,spineWidthMm:10},heightMm:257,marginMm:15,bleedMm:3,contentPlacement:'followPanels' as const,clearPageSettingsConfirmed:false};
 const send=vi.fn<(request:PublicationCommandRequest)=>Promise<unknown>>().mockRejectedValueOnce(new Error('lost')).mockImplementation(async request=>receipt(request));
 const client=new PublicationCommandClient(send,localStorage);await expect(client.execute(project,publication,5,[{type:'SetCover',data}])).rejects.toMatchObject({code:'PUBLICATION_PENDING'});
 await new PublicationCommandClient(send,localStorage).retry(project,publication);
 expect(send.mock.calls[1][0]).toEqual(send.mock.calls[0][0]);expect(send.mock.calls[1][0].operations[0].action).toEqual({type:'SetCover',data});expect(client.pending(project)).toEqual([]);
});

it('书脊文字保存中断后保留明确方向、原文与毫米矩形，只确认原请求',async()=>{
 const {continuousFixture}=await import('../test/publicationContinuousFixture'),{publicationTextValues}=await import('../features/publishing/publicationText'),{layout}=continuousFixture(),data={...publicationTextValues(layout.pages[0],undefined,crypto.randomUUID(),'text'),xMm:20,yMm:30,widthMm:8,heightMm:160,text:'Night Walk · 夜行',textDirection:'rotate-90' as const};
 const send=vi.fn<(request:PublicationCommandRequest)=>Promise<unknown>>().mockRejectedValueOnce(new Error('lost')).mockImplementation(async request=>receipt(request));
 await expect(new PublicationCommandClient(send,localStorage).execute(project,publication,5,[{type:'AddText',data}])).rejects.toMatchObject({code:'PUBLICATION_PENDING'});
 await new PublicationCommandClient(send,localStorage).retry(project,publication);expect(send.mock.calls[1][0]).toEqual(send.mock.calls[0][0]);expect(send.mock.calls[1][0].operations[0].action).toEqual({type:'AddText',data});
});
