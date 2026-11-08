import { act,fireEvent,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import type { DesktopProjectRepository,ObservationPixelAudit } from '../../domain/models';
import { setCopyLocale } from '../../app/copy';
import { ProjectObservationVersions } from './ProjectObservationVersions';
const item=(observationId:number,photoId:string,filename:string,outcome:string)=>({observationId,photoId,filename,outcome});
const provable=[item(1,'a','旧版一.jpg','ready'),item(2,'b','旧版二.jpg','ready'),item(3,'c','旧版三.jpg','ready')];
const blocked=[item(4,'d','原图丢了.jpg','original_missing'),item(5,'e','换过问法.jpg','pipeline_changed')];
const ledger:ObservationPixelAudit={rows:5,scanned:5,photos:4,ready:3,refused:2,reasons:{original_missing:1,pipeline_changed:1},items:[...provable,...blocked]};
const afterBackfill:ObservationPixelAudit={...ledger,rows:2,scanned:2,photos:2,ready:0,items:[...blocked]};
const clean:ObservationPixelAudit={rows:0,scanned:0,photos:0,ready:0,refused:0,reasons:{},items:[]};
const nothingProvable:ObservationPixelAudit={...ledger,ready:0,refused:5,reasons:{cache_differs:2,original_changed:2,pipeline_changed:1},items:[...blocked,item(6,'f','重算对不上.jpg','cache_differs'),item(7,'g','原图换过.jpg','original_changed')]};
const recheck={drafts:9,draftsScanned:2,recheckOwed:2,recheckedDrafts:2,recheckFailed:0,recheckRemaining:0};
const done:ObservationPixelAudit={...ledger,backfilled:3,remaining:0,recheck};
const countLabel='数一下还有几条历史观察没记像素版本';
const backfillLabel='把能证明的历史观察补上像素版本';
function setup(extra:Record<string,unknown>={}){
  const repository={auditObservationVersions:vi.fn().mockResolvedValue(ledger),backfillObservationVersions:vi.fn().mockResolvedValue(done),...extra};
  render(<ProjectObservationVersions repository={repository as unknown as DesktopProjectRepository} projectId="p"/>);
  return repository as unknown as {auditObservationVersions:ReturnType<typeof vi.fn>,backfillObservationVersions:ReturnType<typeof vi.fn>};
}
afterEach(()=>setCopyLocale('zh'));
it('没有这个能力的仓库整条不出现，网页预览不摆本地补写入口',()=>{
  const bare=render(<ProjectObservationVersions projectId="p"/>);
  expect(bare.container).toBeEmptyDOMElement();
  const web=render(<ProjectObservationVersions repository={{} as DesktopProjectRepository} projectId="p"/>);
  expect(web.container).toBeEmptyDOMElement();
});
it('数字来自后端点名，补不了的按原因分开说，名单落到具体哪几条',async()=>{
  const user=userEvent.setup();const repository=setup();
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(repository.auditObservationVersions).toHaveBeenCalledWith('p');
  expect(await screen.findByText('5 条历史观察没记像素版本：3 条能证明就是现在这一档字节，2 条补不了，涉及 4 张照片。')).toBeVisible();
  expect(screen.getByText(/原图不在这台机器上，要先重新定位这张照片 · 1 条/)).toBeVisible();
  await user.click(screen.getByText('看看是哪几条'));
  expect(screen.getByText(/旧版一\.jpg、旧版二\.jpg、旧版三\.jpg/)).toHaveTextContent('能证明就是现在这一档字节');
  expect(screen.getByText(/换过问法\.jpg/)).toHaveTextContent('这一档问法今天已经不认得');
  expect(screen.queryByText(/已补上/)).not.toBeInTheDocument();
});
it('一条都没记过时说一句实话，不摆补写按钮',async()=>{
  const user=userEvent.setup();setup({auditObservationVersions:vi.fn().mockResolvedValue(clean)});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByText('这个项目的历史观察都记着像素版本，没有要补的。')).toBeVisible();
  expect(screen.queryByRole('button',{name:backfillLabel})).not.toBeInTheDocument();
  expect(screen.queryByText('看看是哪几条')).not.toBeInTheDocument();
});
it('全都补不了就不推销这个动作做不到的"补上"',async()=>{
  const user=userEvent.setup();setup({auditObservationVersions:vi.fn().mockResolvedValue(nothingProvable)});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByText(/5 条补不了/)).toBeVisible();
  expect(screen.queryByRole('button',{name:backfillLabel})).not.toBeInTheDocument();
});
it('仓库没有补写能力时只点名，不摆一个点不动的按钮',()=>{
  const repository={auditObservationVersions:vi.fn().mockResolvedValue(ledger)};
  render(<ProjectObservationVersions repository={repository as unknown as DesktopProjectRepository} projectId="p"/>);
  expect(screen.getByRole('button',{name:countLabel})).toBeVisible();
  expect(screen.queryByRole('button',{name:backfillLabel})).not.toBeInTheDocument();
});
it('补完报清补了几条、留下几条、几份口径重算，状态条改按补完之后那一遍的数字说话',async()=>{
  const user=userEvent.setup();
  const repository=setup({auditObservationVersions:vi.fn().mockResolvedValueOnce(ledger).mockResolvedValue(afterBackfill)});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:backfillLabel}));
  expect(repository.backfillObservationVersions).toHaveBeenCalledWith('p');
  expect(await screen.findByText('已补上 3 条，0 条留到下一轮。')).toBeVisible();
  expect(screen.getByText('收着这些照片的 2 份方案口径已跟着重算。')).toBeVisible();
  expect(repository.auditObservationVersions).toHaveBeenCalledTimes(2);
  expect(screen.getByText('2 条历史观察没记像素版本：0 条能证明就是现在这一档字节，2 条补不了，涉及 2 张照片。')).toBeVisible();
  expect(screen.queryByRole('button',{name:backfillLabel})).not.toBeInTheDocument();
});
it('重新数一遍就把上一轮的补写结果放下，不留着冒充现状',async()=>{
  const user=userEvent.setup();
  setup({auditObservationVersions:vi.fn().mockResolvedValueOnce(ledger).mockResolvedValue(afterBackfill)});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:backfillLabel}));
  expect(await screen.findByText(/已补上 3 条/)).toBeVisible();
  await user.click(screen.getByRole('button',{name:countLabel}));
  await screen.findByText(/2 条历史观察没记像素版本/);
  expect(screen.queryByText(/已补上/)).not.toBeInTheDocument();
});
it('一轮补不完就说还欠几条，不把没轮到的算成补不了',async()=>{
  const user=userEvent.setup();
  setup({auditObservationVersions:vi.fn().mockResolvedValueOnce(ledger).mockResolvedValue({...ledger,rows:5,scanned:5,ready:3,refused:2,backfilled:undefined}),
    backfillObservationVersions:vi.fn().mockResolvedValue({...ledger,backfilled:1,remaining:2,recheck:{...recheck,recheckRemaining:2,recheckedDrafts:1}})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:backfillLabel}));
  expect(await screen.findByText('已补上 1 条，2 条留到下一轮。')).toBeVisible();
  expect(screen.getByText('重算了 1 份，还有 2 份这一轮没点到，再补一次就行。')).toBeVisible();
});
it('这一轮没算成的口径单独说，不混进已经跟上那批',async()=>{
  const user=userEvent.setup();
  setup({backfillObservationVersions:vi.fn().mockResolvedValue({...ledger,backfilled:3,remaining:0,recheck:{...recheck,recheckFailed:1}})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:backfillLabel}));
  expect(await screen.findByText('其中 1 份这次没算成，仍旧按旧口径放着。')).toBeVisible();
});
it('版本号补上了但口径这趟没跑成，两件事分开说',async()=>{
  const user=userEvent.setup();
  setup({backfillObservationVersions:vi.fn().mockResolvedValue({...ledger,backfilled:3,remaining:0,recheckError:{code:'BACKGROUND',message:'本地评估这趟没跑成'}})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:backfillLabel}));
  expect(await screen.findByText('已补上 3 条，0 条留到下一轮。')).toBeVisible();
  expect(screen.getByRole('alert')).toHaveTextContent('口径这一趟没算成：本地评估这趟没跑成。补上的版本已经在库里了，可以再算一次。');
  expect(screen.queryByText(/方案口径已跟着重算/)).not.toBeInTheDocument();
});
it('只点到一部分时不把没点到的说成补不了',async()=>{
  const user=userEvent.setup();
  setup({auditObservationVersions:vi.fn().mockResolvedValue({...ledger,rows:6200,scanned:5,photos:4})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByText('这次只点到 5 条，一共 6200 条；没点到的不算成补不了。')).toBeVisible();
});
it('名单被截断时说清其余都算在总数里',async()=>{
  const user=userEvent.setup();setup({auditObservationVersions:vi.fn().mockResolvedValue({...ledger,items:provable})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByText('看看是哪几条'));
  expect(screen.getByText('名单只列出前 3 条，其余 2 条都算在上面的总数里。')).toBeVisible();
});
it('后端给了这里还没译的结局码就原样露出，不编一句好听的说法',async()=>{
  const user=userEvent.setup();
  setup({auditObservationVersions:vi.fn().mockResolvedValue({...ledger,reasons:{future_check:1},items:[...provable,item(9,'z','将来.jpg','future_check')]})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByText('future_check · 1 条')).toBeVisible();
  await user.click(screen.getByText('看看是哪几条'));
  expect(screen.getByText(/将来\.jpg/)).toHaveTextContent('future_check');
});
it('点名失败只说错误，不编一份数字出来',async()=>{
  const user=userEvent.setup();setup({auditObservationVersions:vi.fn().mockRejectedValue(new Error('项目已不在库里，请刷新后重试'))});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByRole('alert')).toHaveTextContent('项目已不在库里，请刷新后重试');
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:backfillLabel})).not.toBeInTheDocument();
});
it('等回执期间挡住重复点击',async()=>{
  let finish:(value:ObservationPixelAudit)=>void=()=>{};
  const repository=setup({auditObservationVersions:vi.fn(()=>new Promise<ObservationPixelAudit>(resolve=>{finish=resolve;}))});
  const button=screen.getByRole('button',{name:countLabel});
  fireEvent.click(button);fireEvent.click(button);
  expect(repository.auditObservationVersions).toHaveBeenCalledTimes(1);
  await act(async()=>finish(ledger));
  expect(await screen.findByText(/5 条历史观察没记像素版本/)).toBeVisible();
});
