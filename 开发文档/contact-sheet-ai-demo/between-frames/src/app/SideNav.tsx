import { Grid2X2, ListOrdered, FolderOpen, Gem, Keyboard, PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import { useUIStore } from '../stores/uiStore';
import type { Project, WorkspaceView } from '../domain/models';
import type { SharingStatus } from '../domain/sharing';
import { zh } from './copy';

export function SideNav({ project, page, view, onView, onProjects, onMemory, onHelp, local = false, authority }: {
  project?: Project; page: 'projects' | 'workspace'; view: WorkspaceView;
  onView: (view: WorkspaceView) => void; onProjects: () => void; onMemory: () => void; onHelp: () => void;
  local?: boolean; authority?: SharingStatus['authority'];
}) {
  const ui = useUIStore();
  const projectKind = !local ? (project?.id === 'sample-project' ? '示例项目' : '预览项目')
    : authority === 'shared' ? '共享项目 · 只读缓存'
    : authority === 'local-promoting' ? '共享迁移中 · 暂停编辑'
    : authority === 'local' ? '本地项目' : '本机项目缓存';
  const storageLabel = authority === 'shared' ? '共享项目 · 本机只读缓存'
    : authority === 'local-promoting' ? '迁移中 · 本地写入冻结' : '本机保存 · 原图只读';
  return <aside className="sidebar"><div className="logo"><img className="logo-symbol" src="/focalroom-mark.png" alt="FocalRoom 标志" /><div>
    <div className="brand">FocalRoom</div><div className="brand-sub">PHOTO STUDIO</div></div></div>
    <button className="nav-fold" onClick={ui.toggleSidebar} aria-label={ui.sidebarOpen ? '收起导航' : '展开导航'}>{ui.sidebarOpen ? <PanelLeftClose size={16} /> : <PanelLeftOpen size={16} />}</button>
    <div className="group-label">WORKSPACE</div><nav aria-label="主导航">
      <button className={`nav ${page === 'workspace' && view === 'grid' ? 'current' : ''}`} aria-label={zh.workspace} onClick={() => onView('grid')} disabled={!project}><Grid2X2 size={17} /><span>{zh.workspace}</span></button>
      <button className={`nav ${page === 'workspace' && view === 'sequence' ? 'current' : ''}`} aria-label={zh.sequence} onClick={() => onView('sequence')} disabled={!project}><ListOrdered size={17} /><span>{zh.sequence}</span></button>
      <button className="nav" aria-label={zh.memory} onClick={onMemory} disabled={!project}><Gem size={17} /><span>{zh.memory}</span></button>
      <button className={`nav ${page === 'projects' ? 'current' : ''}`} aria-label={zh.projects} onClick={onProjects}><FolderOpen size={17} /><span>{zh.projects}</span></button>
    </nav><div className="side-divider" /><div className="group-label">CURRENT PROJECT</div>
    {project && <button className="project-link" onClick={() => onView('grid')}><strong>{project.title}</strong>
      <small>{project.photos.length} 张图像 · {project.sequence.length} 张入选<br />{projectKind}</small></button>}
    <div className="sidebar-bottom"><button className="help-link" onClick={onHelp}><Keyboard size={14} />{zh.help}</button>
      <span className="dot" />{local ? storageLabel : zh.preview}<br />{local ? '图像上传需授权' : '仅本地 UI 预览 · 无云端上传'}</div>
  </aside>;
}
