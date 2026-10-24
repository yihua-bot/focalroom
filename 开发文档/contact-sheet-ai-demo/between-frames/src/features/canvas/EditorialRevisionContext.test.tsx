import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import type { CanvasRepository, EditorialRevisionContext as RevisionContext } from '../../domain/canvas';
import { EditorialRevisionContext } from './EditorialRevisionContext';

const context: RevisionContext = {
  draftId:'child',parent:{kind:'board',id:'parent',revision:3},parentTitle:'第二轮方案',
  discussion:{draftId:'parent',draftRevision:3,cursor:12,totalMessages:1,eventCount:1,nextOffset:null,messages:[{id:'message',threadId:'thread',changeId:'old-change',body:'保留人物与空间的关系',rootBody:'保留人物与空间的关系',isReply:false,resolved:false,createdAt:'2026-10-24T00:00:00Z'}]},
  decisions:[{changeId:'old-change',nodeType:'photo',kind:'change',decision:'rejected',fieldDecisions:{position:'rejected'}}],
  observations:[{photoId:'photo',photoHash:'a'.repeat(64),photoName:'frame.jpg',thumbnailPath:null,modelId:'fixture',promptVersion:'fixture',taskId:'old-task',callId:'inspect',visibleFacts:['生成色块'],interpretation:'旧版解读',uncertainty:'不代表专业质量结论',currentHashMatches:false}],
  totalDecisions:35,totalObservations:1,nextOffset:30,hasEarlier:true,
};
const renderContext = (read: CanvasRepository['editorialRevisionContext']) => render(<EditorialRevisionContext projectId="project" draftId="child" repository={{editorialRevisionContext:read} as CanvasRepository} />);
it('前版意见、决定和观察只读呈现，照片变化时明确提示，不暴露内部标识', async () => {
  const read=vi.fn().mockResolvedValue(context);renderContext(read);const user=userEvent.setup();
  await waitFor(()=>expect(read).toHaveBeenCalledWith('project','child',0));
  await user.click(screen.getByText('修订依据',{exact:true}));
  expect(await screen.findByText('保留人物与空间的关系')).toBeVisible();
  expect(screen.getByText('照片版本已变化，请重新查看后再作判断。')).toBeVisible();
  expect(screen.getByText('旧版解读')).toBeVisible();expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:'采纳选中改动'})).not.toBeInTheDocument();
  expect(screen.queryByText('a'.repeat(64))).not.toBeInTheDocument();
});
it('按固定前版翻页并沿父版查看更早修订，可返回，不把批注写入当前版本', async () => {
  const read=vi.fn(async (_p:string,draft:string,offset:number) => draft==='parent'?{...context,draftId:'parent',parent:{...context.parent,id:'root'},parentTitle:'第一轮方案',hasEarlier:false,nextOffset:null}:{...context,nextOffset:offset===0?30:null});
  renderContext(read);const user=userEvent.setup();await user.click(screen.getByText('修订依据',{exact:true}));
  await user.click(await screen.findByRole('button',{name:'下一页'}));await waitFor(()=>expect(read).toHaveBeenLastCalledWith('project','child',30));
  await user.click(await screen.findByRole('button',{name:'上一页'}));await waitFor(()=>expect(read).toHaveBeenLastCalledWith('project','child',0));
  await user.click(await screen.findByRole('button',{name:'更早版本'}));expect(await screen.findByText(/第一轮方案/)).toBeVisible();
  await user.click(await screen.findByRole('button',{name:'返回较新版本'}));expect(await screen.findByText(/第二轮方案/)).toBeVisible();
});
it('读取失败可重试同一页，不能将失败伪装成没有修订依据', async () => {
  const read=vi.fn().mockRejectedValueOnce(new Error('fixture')).mockResolvedValueOnce(context);renderContext(read);const user=userEvent.setup();
  await user.click(screen.getByText('修订依据',{exact:true}));await screen.findByRole('alert');
  await user.click(screen.getByRole('button',{name:'重试'}));expect(await screen.findByText('保留人物与空间的关系')).toBeVisible();expect(read).toHaveBeenCalledTimes(2);
});
it('没有冻结依据的旧方案不补造父版内容', async () => {
  const read=vi.fn().mockResolvedValue(null);renderContext(read);await waitFor(()=>expect(screen.queryByText('修订依据')).not.toBeInTheDocument());
});
it('照片版本一致时显示关联预览，版本不一致时不展示当前像素冒充旧观察', async () => {
  const read=vi.fn().mockResolvedValue({...context,observations:[{...context.observations[0],currentHashMatches:true,thumbnailPath:'/fixture-preview.jpg'}]});
  renderContext(read);await userEvent.click(screen.getByText('修订依据',{exact:true}));
  expect(await screen.findByRole('img',{name:'frame.jpg'})).toHaveAttribute('src','/fixture-preview.jpg');
});
it('尚未决定的前版建议显示用户文案，不能显示内部翻译键',async()=>{
  renderContext(vi.fn().mockResolvedValue({...context,decisions:[{...context.decisions[0],decision:'pending',fieldDecisions:{position:'pending'}}]}));
  await userEvent.click(screen.getByText('修订依据',{exact:true}));expect(await screen.findByText('调整 · 尚未决定')).toBeVisible();
  expect(screen.queryByText(/editorialDecision_pending/)).not.toBeInTheDocument();
});
