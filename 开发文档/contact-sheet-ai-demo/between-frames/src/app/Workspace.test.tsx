import { render,screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach,expect,it,vi } from 'vitest';
import type { DesktopProjectRepository,Project } from '../domain/models';
import type { SharingStatus } from '../domain/sharing';
import { useUIStore } from '../stores/uiStore';
import { Workspace } from './Workspace';
const project:Project={id:'p',title:'作品',brief:'',revision:1,photos:[{id:'a',name:'a.jpg',src:'/a.jpg',index:1,status:'undecided',note:'',rating:0}],sequence:[],events:[]};
const audit={total:120,scanned:120,healthy:112,needsRepair:5,blocked:3,reasons:{original_missing:3},items:[]};
const pixelRepo={auditObservationVersions:vi.fn(),backfillObservationVersions:vi.fn()};
const sidecarRepo={...pixelRepo,sidecarProposals:vi.fn().mockResolvedValue([])};
function setup(overrides:Partial<Parameters<typeof Workspace>[0]>={},repoExtra:Record<string,unknown>={}){
  const repository={auditPreviews:vi.fn().mockResolvedValue(audit),...repoExtra};
  render(<Workspace project={project} visible={project.photos} kept={[]} selectedIds={[]} busy={false}
    onSelect={vi.fn()} onBatchDecide={vi.fn()} onBatchRate={vi.fn()} onEdit={vi.fn()} onImport={vi.fn()} onCompare={vi.fn()} onExport={vi.fn()}
    onDecide={vi.fn()} onReorder={vi.fn()} onEnlarge={vi.fn()} onUndo={vi.fn()} onRedo={vi.fn()} onVersions={vi.fn()} onActivity={vi.fn()} onCopyPlan={vi.fn()}
    local projectRepository={repository as unknown as DesktopProjectRepository} {...overrides}/>);
  return repository as unknown as {auditPreviews:ReturnType<typeof vi.fn>};
}
const row=()=>document.querySelector('.local-checks');
const body=()=>document.querySelector('.local-checks-body');
const toggle=()=>screen.getByRole('button',{name:/本机自查/});
const expand=async()=>{await userEvent.click(toggle());};
afterEach(()=>useUIStore.getState().reset());
it('三条本机自查折成图库一侧的一行，平时不占选片网格上面的位置',async()=>{
  setup();
  expect(row()).not.toBeNull();
  expect(toggle()).toHaveAttribute('aria-expanded','false');
  expect(body()).toHaveAttribute('hidden');
});
it('点行才摊开三条，摊开本身不读盘',async()=>{
  const repository=setup();
  await expand();
  expect(toggle()).toHaveAttribute('aria-expanded','true');
  expect(body()).not.toHaveAttribute('hidden');
  expect(screen.getByRole('region',{name:'本地预览'})).toBeVisible();
  expect(repository.auditPreviews).not.toHaveBeenCalled();
});
it('数过的数字收起再摊开还在，不用重新点一遍',async()=>{
  setup();
  await expand();
  await userEvent.click(screen.getByRole('button',{name:'检查预览'}));
  expect(await screen.findByText('120 张里 5 张可重生成，3 张要先找回原图。')).toBeVisible();
  await expand();
  expect(body()).toHaveAttribute('hidden');
  await expand();
  expect(screen.getByText('120 张里 5 张可重生成，3 张要先找回原图。')).toBeVisible();
  expect(screen.getByRole('button',{name:'检查预览'})).toBeVisible();
});
it('历史观察在同一行里，不用先打开某份方案',async()=>{
  setup({},pixelRepo);
  await expand();
  expect(screen.getByRole('region',{name:'历史观察'})).toBeVisible();
});
it('作品序列那侧也是这一行，外部软件的判断同样点得开',async()=>{
  useUIStore.getState().setView('sequence');
  setup({},sidecarRepo);
  await expand();
  expect(screen.getByRole('region',{name:'外部软件里的判断'})).toBeVisible();
});
it('网页预览和只读共享不摆本机文件这一行',()=>{
  setup({local:false});
  expect(row()).toBeNull();
  setup({sharing:{authority:'shared'} as unknown as SharingStatus});
  expect(row()).toBeNull();
});
it('画布视图讲的是画布的事，这条本机自查不挤进去',()=>{
  useUIStore.getState().setView('canvas');
  setup();
  expect(row()).toBeNull();
});
it('三条能力都没有时不摆一个点开是空的行',()=>{
  setup({}, {auditPreviews:undefined});
  expect(row()).toBeNull();
});
