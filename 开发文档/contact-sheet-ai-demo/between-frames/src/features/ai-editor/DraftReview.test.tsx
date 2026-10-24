import { act,fireEvent,render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import type { DesktopProjectRepository,Project } from '../../domain/models';
import type { SelectionDraft } from '../../domain/agent';
import { setCopyLocale } from '../../app/copy';
import { DraftReview } from './DraftReview';
const project:Project={id:'p',title:'作品',brief:'',revision:1,photos:[{id:'a',name:'a.jpg',src:'/a.jpg',index:1,status:'undecided',note:'',rating:0},{id:'b',name:'b.jpg',src:'/b.jpg',index:2,status:'undecided',note:'',rating:0}],sequence:[],events:[]};
const draft:SelectionDraft={id:'draft',projectId:'p',taskId:'task',title:'选片方案',baseRevision:1,revision:1,state:'pending',stale:false,summary:'保留人物',items:[{photo_id:'a',proposed_action:'keep',position:0,rationale:'开篇'},{photo_id:'b',proposed_action:'keep',position:1,rationale:'过渡'}],createdAt:'now'};
function setup(value=draft){const repository={agent:{},applyDraft:vi.fn().mockResolvedValue(project),discardDraft:vi.fn().mockResolvedValue(project),rebaseDraft:vi.fn().mockResolvedValue(project),updateDraft:vi.fn().mockResolvedValue(project)};const onUpdated=vi.fn(),onClose=vi.fn();render(<DraftReview draft={value} project={project} repository={repository as unknown as DesktopProjectRepository} onClose={onClose} onUpdated={onUpdated}/>);return {repository,onUpdated,onClose};}
afterEach(()=>setCopyLocale('zh'));
it('只发送明确勾选的照片，等待回执期间阻止重复采纳',async()=>{
  const {repository,onUpdated}=setup();const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'清除选择'}));await user.click(screen.getByRole('checkbox',{name:'采纳 a.jpg'}));
  let finish:(value:Project)=>void=()=>{};repository.applyDraft.mockImplementationOnce(()=>new Promise<Project>(resolve=>{finish=resolve;}));
  const button=screen.getByRole('button',{name:'采纳所选 1 张'});fireEvent.click(button);fireEvent.click(button);
  expect(repository.applyDraft).toHaveBeenCalledTimes(1);expect(repository.applyDraft).toHaveBeenCalledWith('p',draft,['a'],'partial');expect(onUpdated).not.toHaveBeenCalled();await act(async()=>finish(project));expect(onUpdated).toHaveBeenCalledOnce();
});
it('替换整个作品序列必须再次明确确认',async()=>{
  const {repository}=setup();const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'替换作品序列'}));expect(repository.applyDraft).not.toHaveBeenCalled();
  await user.click(screen.getByRole('button',{name:'确认替换'}));expect(repository.applyDraft).toHaveBeenCalledWith('p',draft,['a','b'],'full');
});
it('已处理的方案不把全部建议显示为已勾选采纳，也不显示写入入口',()=>{
  setup({...draft,state:'applied'});expect(screen.queryByRole('checkbox')).not.toBeInTheDocument();expect(screen.queryByRole('button',{name:'全选方案照片'})).not.toBeInTheDocument();expect(screen.getByText('这份方案已结束审核。采纳记录可在工作台撤销。')).toBeVisible();
});
it.each([['en','Replace sequence'],['ja','作品シーケンスを置き換え'],['ko','작품 시퀀스 교체']])('选片审核支持%s，不残留中文操作词', (locale,label)=>{
  setCopyLocale(locale);setup();expect(screen.getByRole('button',{name:label})).toBeVisible();expect(screen.queryByText('清除选择')).not.toBeInTheDocument();
});
