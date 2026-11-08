import { act,fireEvent,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import type { DesktopProjectRepository,ProjectPreviewAudit } from '../../domain/models';
import { setCopyLocale } from '../../app/copy';
import { ProjectPreviewHealth } from './ProjectPreviewHealth';
const item=(photoId:string,filename:string,outcome:string)=>({photoId,filename,outcome,rebuilt:[],detail:''});
const broken=['海边.jpg','山口.jpg','夜车.jpg','窗边.jpg','旧巷.jpg'].map((name,index)=>item(`r${index}`,name,'needs_repair'));
const missing=['丢失一.jpg','丢失二.jpg','丢失三.jpg'].map((name,index)=>item(`m${index}`,name,'original_missing'));
const ledger:ProjectPreviewAudit={total:120,scanned:120,healthy:112,needsRepair:5,blocked:3,reasons:{original_missing:3},items:[...broken,...missing]};
const clean:ProjectPreviewAudit={...ledger,healthy:120,needsRepair:0,blocked:0,reasons:{},items:[]};
const recheck={drafts:3,draftsScanned:3,recheckOwed:3,recheckedDrafts:3,recheckFailed:0,recheckRemaining:0};
const repaired:ProjectPreviewAudit={...ledger,repaired:5,refused:0,remaining:2,recheck};
const countLabel='数一下整个项目还有几张读不出来';
const repairLabel='把整项目的坏预览一次修好';
const continueLabel='接着重算剩下方案的口径';
function setup(extra:Record<string,unknown>={}){
  const repository={auditPreviews:vi.fn().mockResolvedValue(ledger),repairAllPreviews:vi.fn().mockResolvedValue(repaired),...extra};
  render(<ProjectPreviewHealth repository={repository as unknown as DesktopProjectRepository} projectId="p"/>);
  return repository as unknown as {auditPreviews:ReturnType<typeof vi.fn>,repairAllPreviews:ReturnType<typeof vi.fn>,recheckProjectEvidence:ReturnType<typeof vi.fn>};
}
afterEach(()=>setCopyLocale('zh'));
it('没有这个能力的仓库整条不出现，网页预览不摆本地文件入口',()=>{
  const bare=render(<ProjectPreviewHealth projectId="p"/>);
  expect(bare.container).toBeEmptyDOMElement();
  const web=render(<ProjectPreviewHealth repository={{} as DesktopProjectRepository} projectId="p"/>);
  expect(web.container).toBeEmptyDOMElement();
});
it('数字来自后端点名，原因沿用各处同一套说法，名单落到具体文件名',async()=>{
  const user=userEvent.setup();const repository=setup();
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(repository.auditPreviews).toHaveBeenCalledWith('p');
  expect(await screen.findByText('整个项目 120 张：5 张能只读原图重算，3 张得你先把原图找回来，其余现在就能读。')).toBeVisible();
  expect(screen.getByText(/原图不在这台机器上，要先重新定位这张照片 · 3 张/)).toBeVisible();
  await user.click(screen.getByText('看看是哪几张'));
  expect(screen.getByText(/海边\.jpg、山口\.jpg、夜车\.jpg、窗边\.jpg、旧巷\.jpg/)).toHaveTextContent('本地预览读不出来，可以只读原图重算');
  expect(screen.getByText(/丢失一\.jpg、丢失二\.jpg、丢失三\.jpg/)).toHaveTextContent('原图不在这台机器上，要先重新定位这张照片');
  expect(screen.queryByText(/只列出前/)).not.toBeInTheDocument();
});
it('一张都不用修时给一句实话，不摆修不了的按钮',async()=>{
  const user=userEvent.setup();setup({auditPreviews:vi.fn().mockResolvedValue(clean)});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByText('120 张的本地预览现在都能读，没有要修的。')).toBeVisible();
  expect(screen.queryByRole('button',{name:repairLabel})).not.toBeInTheDocument();
  expect(screen.queryByText('看看是哪几张')).not.toBeInTheDocument();
});
it('坏的全是原图丢了或换过的，就不推销这个动作修不了的"一次修好"',async()=>{
  const user=userEvent.setup();setup({auditPreviews:vi.fn().mockResolvedValue({...ledger,needsRepair:0,healthy:117,items:missing})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByText(/3 张得你先把原图找回来/)).toBeVisible();
  expect(screen.queryByRole('button',{name:repairLabel})).not.toBeInTheDocument();
});
it('一次修完报清修了几张、留下几张、几份口径重算，状态条再按修完之后的数字说话',async()=>{
  const user=userEvent.setup();
  const repository=setup({auditPreviews:vi.fn().mockResolvedValueOnce(ledger).mockResolvedValue({...ledger,needsRepair:0,healthy:117,items:missing})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:repairLabel}));
  expect(repository.repairAllPreviews).toHaveBeenCalledWith('p');
  expect(await screen.findByText('已修好 5 张，2 张留到下一轮，3 份方案的口径已跟着重算。')).toBeVisible();
  expect(repository.auditPreviews).toHaveBeenCalledTimes(2);
  expect(screen.getByText('整个项目 120 张：0 张能只读原图重算，3 张得你先把原图找回来，其余现在就能读。')).toBeVisible();
  expect(screen.queryByRole('button',{name:repairLabel})).not.toBeInTheDocument();
});
it('名单被截断时说清其余都算在总数里，不拿列出来的几条冒充全部',async()=>{
  const user=userEvent.setup();setup({auditPreviews:vi.fn().mockResolvedValue({...ledger,items:broken.slice(0,2)})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByText('看看是哪几张'));
  expect(screen.getByText('名单只列出前 2 张，其余 6 张都算在上面的总数里。')).toBeVisible();
});
it('只数到一部分时不把没数到的说成好的',async()=>{
  const user=userEvent.setup();setup({auditPreviews:vi.fn().mockResolvedValue({...clean,scanned:5000,total:6200})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByText('这次只数到 5000 张，项目一共 6200 张；没数到的不算成好的。')).toBeVisible();
});
it('点名失败只说错误，不编一份数字出来',async()=>{
  const user=userEvent.setup();setup({auditPreviews:vi.fn().mockRejectedValue(new Error('项目已不在库里，请刷新后重试'))});
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByRole('alert')).toHaveTextContent('项目已不在库里，请刷新后重试');
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:repairLabel})).not.toBeInTheDocument();
});
it('等回执期间挡住重复点击',async()=>{
  let finish:(value:ProjectPreviewAudit)=>void=()=>{};
  const repository=setup({auditPreviews:vi.fn(()=>new Promise<ProjectPreviewAudit>(resolve=>{finish=resolve;}))});
  const button=screen.getByRole('button',{name:countLabel});
  fireEvent.click(button);fireEvent.click(button);
  expect(repository.auditPreviews).toHaveBeenCalledTimes(1);
  await act(async()=>finish(ledger));
  expect(await screen.findByText(/整个项目 120 张/)).toBeVisible();
});
it('修完再数一遍就按现在的结果说话，上一轮"已修好"不留着冒充现状',async()=>{
  const user=userEvent.setup();
  setup({auditPreviews:vi.fn().mockResolvedValueOnce(ledger).mockResolvedValue(clean)});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:repairLabel}));
  expect(await screen.findByText(/已修好 5 张/)).toBeVisible();
  await user.click(screen.getByRole('button',{name:countLabel}));
  expect(await screen.findByText('120 张的本地预览现在都能读，没有要修的。')).toBeVisible();
  expect(screen.queryByText(/已修好 5 张/)).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:repairLabel})).not.toBeInTheDocument();
});
const owed={drafts:40,draftsScanned:40,recheckOwed:23,recheckedDrafts:3,recheckFailed:0,recheckRemaining:20};
it('口径欠账说清还差几份，接着算只重算口径、不重新修图',async()=>{
  const user=userEvent.setup();
  const repository=setup({
    repairAllPreviews:vi.fn().mockResolvedValue({...ledger,repaired:5,remaining:0,recheck:owed}),
    recheckProjectEvidence:vi.fn().mockResolvedValue({drafts:40,draftsScanned:40,recheckOwed:20,recheckedDrafts:20,recheckFailed:0,recheckRemaining:0}),
  });
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:repairLabel}));
  expect(await screen.findByText('口径还欠 20 份没跟上（项目一共 40 份方案）；接着算不用重新修图。')).toBeVisible();
  await user.click(screen.getByRole('button',{name:continueLabel}));
  expect(repository.recheckProjectEvidence).toHaveBeenCalledWith('p');
  expect(repository.recheckProjectEvidence).toHaveBeenCalledTimes(1);
  expect(repository.repairAllPreviews).toHaveBeenCalledTimes(1);
  expect(await screen.findByText('这一轮重算了 20 份，点到的 40 份方案口径都跟上了。')).toBeVisible();
  expect(screen.queryByText(/口径还欠/)).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:continueLabel})).not.toBeInTheDocument();
});
it('这一轮没算成的仍留在欠账里，不报成已经跟上',async()=>{
  const user=userEvent.setup();
  setup({
    repairAllPreviews:vi.fn().mockResolvedValue({...ledger,repaired:5,recheck:{drafts:9,draftsScanned:9,recheckOwed:5,recheckedDrafts:3,recheckFailed:2,recheckRemaining:2}}),
    recheckProjectEvidence:vi.fn(),
  });
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:repairLabel}));
  expect(await screen.findByText('其中 2 份这次没算成，仍留在欠着的那批里。')).toBeVisible();
  expect(screen.getByText('口径还欠 2 份没跟上（项目一共 9 份方案）；接着算不用重新修图。')).toBeVisible();
  expect(screen.getByRole('button',{name:continueLabel})).toBeVisible();
});
it('方案太多只点到一部分时，跟上的是点到那批，没点到的另说',async()=>{
  const user=userEvent.setup();
  setup({repairAllPreviews:vi.fn().mockResolvedValue({...ledger,repaired:5,recheck:{drafts:2600,draftsScanned:2000,recheckOwed:0,recheckedDrafts:0,recheckFailed:0,recheckRemaining:0}})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:repairLabel}));
  expect(await screen.findByText('这次只点到 2000 份方案，项目一共 2600 份；没点到的不算成跟上了。')).toBeVisible();
  expect(screen.getByText('这一轮重算了 0 份，点到的 2000 份方案口径都跟上了。')).toBeVisible();
});
it('文件修好了但口径这一趟没跑成，两件事分开说，还能接着再算',async()=>{
  const user=userEvent.setup();
  const repository=setup({
    repairAllPreviews:vi.fn().mockResolvedValue({...ledger,repaired:5,remaining:0,recheckError:{code:'BACKGROUND',message:'本地评估这趟没跑成'}}),
    recheckProjectEvidence:vi.fn().mockResolvedValue({drafts:2,draftsScanned:2,recheckOwed:2,recheckedDrafts:2,recheckFailed:0,recheckRemaining:0}),
  });
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:repairLabel}));
  expect(await screen.findByText('已修好 5 张，0 张留到下一轮，0 份方案的口径已跟着重算。')).toBeVisible();
  expect(screen.getByRole('alert')).toHaveTextContent('口径这一趟没算成：本地评估这趟没跑成。修好的文件已经在磁盘上，可以接着再算一次。');
  await user.click(screen.getByRole('button',{name:continueLabel}));
  expect(repository.recheckProjectEvidence).toHaveBeenCalledTimes(1);
  expect(await screen.findByText('这一轮重算了 2 份，点到的 2 份方案口径都跟上了。')).toBeVisible();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});
it('仓库没有接着算的能力时只报欠账，不摆一个点不动的按钮',async()=>{
  const user=userEvent.setup();
  setup({repairAllPreviews:vi.fn().mockResolvedValue({...ledger,repaired:5,recheck:owed})});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:repairLabel}));
  expect(await screen.findByText(/口径还欠 20 份没跟上/)).toBeVisible();
  expect(screen.queryByRole('button',{name:continueLabel})).not.toBeInTheDocument();
});
it('重新数一遍就把上一轮的口径欠账放下，不留着冒充现状',async()=>{
  const user=userEvent.setup();
  setup({repairAllPreviews:vi.fn().mockResolvedValue({...ledger,repaired:5,recheck:owed}),recheckProjectEvidence:vi.fn()});
  await user.click(screen.getByRole('button',{name:countLabel}));
  await user.click(await screen.findByRole('button',{name:repairLabel}));
  expect(await screen.findByText(/口径还欠 20 份没跟上/)).toBeVisible();
  await user.click(screen.getByRole('button',{name:countLabel}));
  await screen.findByText(/整个项目 120 张/);
  expect(screen.queryByText(/口径还欠/)).not.toBeInTheDocument();
  expect(screen.queryByRole('button',{name:continueLabel})).not.toBeInTheDocument();
});