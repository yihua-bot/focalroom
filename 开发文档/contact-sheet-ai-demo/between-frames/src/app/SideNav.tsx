import { CloudAccount } from "../features/cloud-login/CloudLoginView";
import type { CloudAuthRepository } from "../domain/cloud-auth";
import { Grid2X2, ListOrdered, FolderOpen, Gem, PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import { useUIStore } from '../stores/uiStore';
import type { Project, WorkspaceView } from '../domain/models';
import type { SharingStatus } from '../domain/sharing';
import { zh } from './copy';
import { useState } from 'react';
import type { DiagnosticsRepository } from '../domain/diagnostics';
import { SettingsDialog } from './SettingsDialog';

export function SideNav({ project, page, view, onView, onProjects, onMemory, onHelp, local = false, authority, cloudAuth, diagnostics }: {
  project?: Project; page: 'projects' | 'workspace'; view: WorkspaceView;
  onView: (view: WorkspaceView) => void; onProjects: () => void; onMemory: () => void; onHelp: () => void;
  cloudAuth?: CloudAuthRepository; diagnostics?: DiagnosticsRepository;
  local?: boolean; authority?: SharingStatus['authority'];
}) {
  const ui = useUIStore();
  const [settingsPage, setSettingsPage] = useState<'general' | 'contact' | null>(null);
  const projectKind = !local ? (project?.id === 'sample-project' ? '示例项目' : '预览项目')
    : authority === 'shared' ? '共享项目 · 只读'
    : authority === 'local-promoting' ? '共享迁移中 · 暂停编辑'
    : authority === 'local' ? '本地项目' : '状态待确认';
  return <aside className="sidebar"><div className="logo"><img className="logo-symbol" src="/focalroom-mark.png" alt="FocalRoom 标志" /><div>
    <div className="brand">FocalRoom</div><div className="brand-sub">PHOTO STUDIO</div></div></div>
    <button className="nav-fold" onClick={ui.toggleSidebar} aria-label={ui.sidebarOpen ? '收起导航' : '展开导航'}>{ui.sidebarOpen ? <PanelLeftClose size={16} /> : <PanelLeftOpen size={16} />}</button>
    <nav aria-label={zh.projects}>
      <button className={`nav ${page === 'projects' ? 'current' : ''}`} aria-current={page === 'projects' ? 'page' : undefined} aria-label={zh.projects} onClick={onProjects}><FolderOpen size={17} /><span>{zh.projects}</span></button>
    </nav>
    {page === 'workspace' && project && <><div className="side-divider" /><div className="group-label">{zh.currentProject}</div>
      <div className="project-link"><strong>{project.title}</strong><small>{project.photos.length} 张图像 · {project.sequence.length} 张入选<br />{projectKind}</small></div>
      <nav aria-label={project.title}>
        <button className={`nav ${ui.panel === 'photo' && view === 'grid' ? 'current' : ''}`} aria-current={ui.panel === 'photo' && view === 'grid' ? 'page' : undefined} aria-label={zh.workspace} onClick={() => onView('grid')}><Grid2X2 size={17} /><span>{zh.workspace}</span></button>
        <button className={`nav ${ui.panel === 'photo' && view === 'sequence' ? 'current' : ''}`} aria-current={ui.panel === 'photo' && view === 'sequence' ? 'page' : undefined} aria-label={zh.sequence} onClick={() => onView('sequence')}><ListOrdered size={17} /><span>{zh.sequence}</span></button>
        <button className={`nav ${ui.panel === 'memory' ? 'current' : ''}`} aria-current={ui.panel === 'memory' ? 'page' : undefined} aria-label={zh.memory} onClick={onMemory}><Gem size={17} /><span>{zh.memory}</span></button>
      </nav>
    </>}
    <div className="sidebar-bottom"><CloudAccount repository={cloudAuth} onSettings={() => setSettingsPage('general')} onHelp={onHelp} onContact={() => setSettingsPage('contact')} /></div>
    {settingsPage && <SettingsDialog page={settingsPage} diagnostics={diagnostics} onClose={() => setSettingsPage(null)} />}
  </aside>;
}
