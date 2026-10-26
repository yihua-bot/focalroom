import { zh } from '../../app/copy';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { useState } from 'react';
import type { DesktopProjectRepository, ExportReceipt } from '../../domain/models';
import { SharingControls } from './SharingControls';

export function ExportDialog({ onClose, repository, projectId, revision = 0, onChanged }: { onClose: () => void; repository?: DesktopProjectRepository; projectId?: string; revision?: number; onChanged?: () => void }) {
  const [includePaths, setIncludePaths] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [receipt, setReceipt] = useState<ExportReceipt | null>(null);
  const save = async (format: 'html' | 'print' | 'shared') => {
    if (!repository || !projectId) return;
    setBusy(true); setError(''); setReceipt(null);
    try { setReceipt(await repository.export(projectId, format, includePaths)); }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { setBusy(false); }
  };
  return <Modal title={zh.exportTitle} description={repository ? undefined : '请在桌面版导出项目。'} onClose={() => { if (!busy) onClose(); }}>
    <label className="sync-toggle"><input type="checkbox" checked={includePaths} disabled={!repository || busy} onChange={event => setIncludePaths(event.target.checked)} />包含本机文件路径</label>
    {includePaths && <p className="muted">包含原图和缓存路径，分享前请检查。</p>}
    <div className="export-options"><Button disabled={!repository || busy} variant="primary" onClick={() => { void save('shared'); }}>共享预览包</Button><Button disabled={!repository || busy} onClick={() => { void save('html'); }}>照片总览</Button><Button disabled={!repository || busy} onClick={() => { void save('print'); }}>印刷参考</Button></div>
    <details><summary>格式说明</summary><p>共享预览包：发给信任的人，对方浏览器打开即可评片。不含原图、GPS、本机路径或个人记忆。</p><p>照片总览：一张网页展示全部照片，适合快速浏览挑选。印刷参考：可在浏览器打印为 PDF。</p></details>
    {receipt && <div role="status" className="export-receipt"><p>已导出：{receipt.path}</p><details><summary>文件校验</summary><small>SHA256 {receipt.sha256}</small></details></div>}
    {error && <p role="alert" className="error-message">{error}</p>}
    {repository?.sharing && projectId && <SharingControls repository={repository.sharing} projectId={projectId} revision={revision} onChanged={onChanged} onBusyChange={setBusy} />}
  </Modal>;
}
