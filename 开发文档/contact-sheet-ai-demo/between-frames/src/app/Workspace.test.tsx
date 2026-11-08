import { render,screen } from '@testing-library/react';
import { afterEach,expect,it,vi } from 'vitest';
import type { DesktopProjectRepository,Project } from '../domain/models';
import type { SharingStatus } from '../domain/sharing';
import { useUIStore } from '../stores/uiStore';
import { Workspace } from './Workspace';
const project:Project={id:'p',title:'作品',brief:'',revision:1,photos:[{id:'a',name:'a.jpg',src:'/a.jpg',index:1,status:'undecided',note:'',rating:0}],sequence:[],events:[]};
const pixelRepo={auditObservationVersions:vi.fn(),backfillObservationVersions:vi.fn()};
const sidecarRepo={...pixelRepo,sidecarProposals:vi.fn().mockResolvedValue([])};
function setup(overrides:Partial<Parameters<typeof Workspace>[0]>={},repoExtra:Record<string,unknown>={}){
  const repository={auditPreviews:vi.fn(),...repoExtra};
  render(<Workspace project={project} visible={project.photos} kept={[]} selectedIds={[]} busy={false}
    onSelect={vi.fn()} onBatchDecide={vi.fn()} onBatchRate={vi.fn()} onEdit={vi.fn()} onImport={vi.fn()} onCompare={vi.fn()} onExport={vi.fn()}
    onDecide={vi.fn()} onReorder={vi.fn()} onEnlarge={vi.fn()} onUndo={vi.fn()} onRedo={vi.fn()} onVersions={vi.fn()} onActivity={vi.fn()} onCopyPlan={vi.fn()}
    local projectRepository={repository as unknown as DesktopProjectRepository} {...overrides}/>);
  return repository;
}
afterEach(()=>useUIStore.getState().reset());
it('本地预览状态常驻在图库一侧：网格和作品序列都看得到，不用先打开某份方案',()=>{
  setup();
  expect(screen.getByRole('region',{name:'本地预览状态'})).toBeVisible();
  useUIStore.getState().setView('sequence');
  setup();
  expect(screen.getAllByRole('region',{name:'本地预览状态'}).length).toBeGreaterThan(0);
});
it('网页预览和只读共享不摆本地文件入口',()=>{
  setup({local:false});
  expect(screen.queryByRole('region',{name:'本地预览状态'})).not.toBeInTheDocument();
  setup({sharing:{authority:'shared'} as unknown as SharingStatus});
  expect(screen.queryByRole('region',{name:'本地预览状态'})).not.toBeInTheDocument();
});
it('画布视图讲的是画布的事，这条项目文件状态不挤进去',()=>{
  useUIStore.getState().setView('canvas');
  setup();
  expect(screen.queryByRole('region',{name:'本地预览状态'})).not.toBeInTheDocument();
});
it('历史观察的像素版本与预览状态同一侧：网格和序列都常驻，不用先打开某份方案',()=>{
  setup({},pixelRepo);
  expect(screen.getByRole('region',{name:'历史观察的像素版本'})).toBeVisible();
  useUIStore.getState().setView('sequence');
  setup({},pixelRepo);
  expect(screen.getAllByRole('region',{name:'历史观察的像素版本'}).length).toBeGreaterThan(0);
});
it('这条不在网页预览、只读共享和画布里出现；仓库没这个能力时也不多摆一条',()=>{
  setup({local:false},pixelRepo);
  expect(screen.queryByRole('region',{name:'历史观察的像素版本'})).not.toBeInTheDocument();
  setup({sharing:{authority:'shared'} as unknown as SharingStatus},pixelRepo);
  expect(screen.queryByRole('region',{name:'历史观察的像素版本'})).not.toBeInTheDocument();
  useUIStore.getState().reset();
  setup();
  expect(screen.getByRole('region',{name:'本地预览状态'})).toBeVisible();
  expect(screen.queryByRole('region',{name:'历史观察的像素版本'})).not.toBeInTheDocument();
});
it('外部软件里的判断在图库这一侧就能看，不用先开导入窗口',()=>{
  setup({},sidecarRepo);
  expect(screen.getByRole('region',{name:'外部软件里的判断'})).toBeVisible();
});
it('作品序列那一侧也常驻同一条判断回流',()=>{
  useUIStore.getState().setView('sequence');
  setup({},sidecarRepo);
  expect(screen.getAllByRole('region',{name:'外部软件里的判断'}).length).toBeGreaterThan(0);
});
it('仓库没这条通道、网页预览和只读共享都不摆回填判断的入口',()=>{
  setup({},pixelRepo);
  expect(screen.queryByRole('region',{name:'外部软件里的判断'})).not.toBeInTheDocument();
  setup({local:false},sidecarRepo);
  expect(screen.queryByRole('region',{name:'外部软件里的判断'})).not.toBeInTheDocument();
  setup({sharing:{authority:'shared'} as unknown as SharingStatus},sidecarRepo);
  expect(screen.queryByRole('region',{name:'外部软件里的判断'})).not.toBeInTheDocument();
});