import { render,screen,waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect,it,vi } from 'vitest';
import type { AgentRepository } from '../../domain/agent';
import type { SelectionRevisionContext as Context } from '../../domain/revisionContext';
import { SelectionRevisionContext } from './SelectionRevisionContext';

const page:Context={draftId:'child',parent:{kind:'selection',id:'parent',revision:2},parentTitle:'前版选片',parentSummary:'保留开篇的人物',parentState:'applied',parentBaseRevision:1,
  items:[{photoId:'a',photoHash:'a'.repeat(64),photoName:'one.jpg',position:0,action:'keep',rationale:'人物开篇',currentHashMatches:true,thumbnailPath:'/preview.jpg'},{photoId:'b',photoHash:'b'.repeat(64),photoName:'two.jpg',position:1,action:'keep',rationale:'空间过渡',currentHashMatches:false,thumbnailPath:null}],
  reviews:[{commandId:'human-action',kind:'apply',createdAt:'2026-10-24T00:00:00Z',mode:'partial',photoIds:['a'],photoNames:['one.jpg'],partial:false,undoneAtDelegation:false}],
  feedback:[{messageId:1,role:'assistant',content:'人物还是空间开篇？',createdAt:'now'},{messageId:2,role:'user',content:'先人物，但不要记为全局偏好',createdAt:'now'}],observations:[],totalItems:2,totalFeedback:2,totalReviews:1,totalObservations:0,nextOffset:null,hasEarlier:false};
const view=(read:AgentRepository['selectionRevisionContext'],projectId='p',draftId='child')=><SelectionRevisionContext repository={{selectionRevisionContext:read} as AgentRepository} projectId={projectId} draftId={draftId} />;
it('前版建议与部分采纳分开呈现，旧照片变更提示，不提供任何写入或模型操作',async()=>{
  const read=vi.fn().mockResolvedValue(page);render(view(read));await userEvent.click(screen.getByText('修订依据',{exact:true}));
  expect(await screen.findByText('采纳所选照片')).toBeVisible();expect(screen.getByText(/委托时的方案状态: 已有采纳记录/)).toBeVisible();
  expect(screen.getByText('先人物，但不要记为全局偏好')).toBeVisible();expect(screen.getByText('当时的提问')).toBeVisible();
  expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
  expect(screen.getByRole('img',{name:'one.jpg'})).toHaveAttribute('src','/preview.jpg');expect(screen.queryByRole('img',{name:'two.jpg'})).not.toBeInTheDocument();
  expect(screen.getByText('照片版本已变化，请重新查看后再作判断。')).toBeVisible();expect(read).toHaveBeenCalledWith('p','child',0);
});
it('父版翻页、早版导航和读取失败后返回，均保持只读',async()=>{
  const read=vi.fn(async (_p:string,draft:string,offset:number)=>{if(draft==='parent')throw new Error('fixture');return {...page,nextOffset:offset===0?30:null,hasEarlier:true};});
  render(view(read));const user=userEvent.setup();await user.click(screen.getByText('修订依据',{exact:true}));
  await user.click(await screen.findByRole('button',{name:'下一页'}));await waitFor(()=>expect(read).toHaveBeenLastCalledWith('p','child',30));
  await user.click(await screen.findByRole('button',{name:'更早版本'}));await screen.findByRole('alert');
  await user.click(screen.getByRole('button',{name:'返回较新版本'}));expect(await screen.findByText('采纳所选照片')).toBeVisible();
});
it('切换项目或方案时立即隐藏旧内容，晚到响应不能串入新方案',async()=>{
  let finish:(value:Context)=>void=()=>{};
  const read=vi.fn().mockImplementationOnce(()=>new Promise<Context>(resolve=>{finish=resolve;})).mockResolvedValueOnce({...page,draftId:'other'});
  const result=render(view(read));result.rerender(view(read,'other-project','other'));
  await userEvent.click(screen.getByText('修订依据',{exact:true}));await screen.findByText('采纳所选照片');
  finish({...page,parentTitle:'不应出现的旧版'});await waitFor(()=>expect(read).toHaveBeenCalledTimes(2));expect(screen.queryByText(/不应出现的旧版/)).not.toBeInTheDocument();
});
it('没有冻结记录的旧方案不补造历史',async()=>{
  render(view(vi.fn().mockResolvedValue(null)));await waitFor(()=>expect(screen.queryByText('修订依据')).not.toBeInTheDocument());
});
