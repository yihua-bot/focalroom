import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import type { CanvasRepository, EditorialMessage } from '../../domain/canvas';
import { EditorialDiscussion } from './EditorialDiscussion';

const root: EditorialMessage = { id:'root',threadId:'root',changeId:'a',body:'中段需要停顿',rootBody:'中段需要停顿',createdAt:'2026-10-24T00:00:00+08:00',resolved:false,isReply:false };
function setup(initial: EditorialMessage[] = [], readOnly = false) {
  let messages = initial, cursor = 1;
  const onRevise = vi.fn();
  const repository = {
    editorialDiscussion: vi.fn(async () => ({ draftId:'draft',draftRevision:4,cursor,eventCount:messages.length,messages,totalMessages:messages.length,nextOffset:null })),
    addEditorialComment: vi.fn(async (_p: string,_d: string,_r: number, data: { changeId: string|null; threadId: string|null; body: string }) => {
      cursor++; const id = `message-${cursor}`;
      messages = [...messages,{ id,threadId:data.threadId ?? id,changeId:data.changeId,body:data.body,rootBody:data.threadId ? root.body : data.body,createdAt:root.createdAt,resolved:false,isReply:!!data.threadId }];
    }),
    resolveEditorialThread: vi.fn(async (_p: string,_d: string,_r: number,thread: string,resolved: boolean) => { cursor++; messages=messages.map(m => m.threadId===thread ? {...m,resolved} : m); }),
  } as unknown as CanvasRepository;
  const view = (changeId = 'a') => <EditorialDiscussion repository={repository} projectId="p" draftId="draft" revision={4} changeId={changeId} changeTitles={{a:'建议一',b:'建议二'}} readOnly={readOnly} onRevise={onRevise} />;
  const result=render(view());
  return { repository,onRevise,result,view };
}
it('批注绑定当前建议，保存和查看不会自动委托 AI', async () => {
  const { repository,onRevise } = setup(); const user=userEvent.setup();
  await screen.findByText('还没有相关批注。');
  await user.type(screen.getByRole('textbox',{name:'写下修改意见'}),'保留开篇，调整中段');
  await user.click(screen.getByRole('button',{name:'保存批注'}));
  await waitFor(() => expect(repository.addEditorialComment).toHaveBeenCalledWith('p','draft',4,{changeId:'a',threadId:null,body:'保留开篇，调整中段'}));
  expect(await screen.findByText('保留开篇，调整中段')).toBeInTheDocument();
  expect(onRevise).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button',{name:'根据批注继续修改'}));
  expect(onRevise).toHaveBeenCalledOnce();
});
it('回复属于原线程，解决后不能回复，可重新打开', async () => {
  const { repository } = setup([root]); const user=userEvent.setup();
  await user.click(await screen.findByRole('button',{name:'回复'}));
  await user.type(screen.getByRole('textbox',{name:'回复内容'}),'空镜放在人物之前');
  const form=screen.getByRole('textbox',{name:'回复内容'}).closest('form')!;
  await user.click(within(form).getByRole('button',{name:'保存批注'}));
  expect(await screen.findByText('空镜放在人物之前')).toBeInTheDocument();
  expect(repository.addEditorialComment).toHaveBeenCalledWith('p','draft',4,{changeId:'a',threadId:'root',body:'空镜放在人物之前'});
  await user.click(screen.getByRole('button',{name:'标记已解决'}));
  await screen.findByText('已解决');
  expect(screen.queryByRole('button',{name:'回复'})).not.toBeInTheDocument();
  await user.click(screen.getByRole('button',{name:'重新讨论'}));
  expect(await screen.findByRole('button',{name:'回复'})).toBeEnabled();
  expect(repository.resolveEditorialThread).toHaveBeenLastCalledWith('p','draft',4,'root',false);
});
it('切换建议保留各自未发送的内容，不把上一条意见写到另一项', async () => {
  const { result,view } = setup(); const user=userEvent.setup();
  await user.type(screen.getByRole('textbox',{name:'写下修改意见'}),'只属于建议一');
  result.rerender(view('b'));
  expect(screen.getByRole('textbox',{name:'写下修改意见'})).toHaveValue('');
  await user.type(screen.getByRole('textbox',{name:'写下修改意见'}),'只属于建议二');
  result.rerender(view('a'));
  expect(screen.getByRole('textbox',{name:'写下修改意见'})).toHaveValue('只属于建议一');
});
it('只读保留讨论，禁止保存、回复、解决和修订', async () => {
  setup([root],true);
  await screen.findByText('中段需要停顿');
  expect(screen.getByRole('textbox',{name:'写下修改意见'})).toBeDisabled();
  expect(screen.getByRole('button',{name:'保存批注'})).toBeDisabled();
  expect(screen.getByRole('button',{name:'标记已解决'})).toBeDisabled();
  expect(screen.getByRole('button',{name:'根据批注继续修改'})).toBeDisabled();
});
