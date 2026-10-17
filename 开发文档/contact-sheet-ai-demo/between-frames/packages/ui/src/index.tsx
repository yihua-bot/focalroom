import App from '../../../src/App';
import { MockRepository } from '../../../src/services/MockRepository';

export { App as DesktopWorkspace };
export { Button } from '../../../src/components/shared/Button';
export { AppShell } from '../../../src/app/AppShell';

// Web M0 独立使用内存示例仓库，不根据宿主环境切换真实 IPC。
const previewRepository = new MockRepository();
export function PreviewWorkspace() {
  return <><div className="v2-preview-notice" role="status">Web 工作台预览 · 示例数据，刷新后重置 · <a href="/shared">进入共享评片</a></div><App repository={previewRepository} /></>;
}
