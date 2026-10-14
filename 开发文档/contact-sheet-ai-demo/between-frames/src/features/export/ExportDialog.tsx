import { zh } from '../../app/copy';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';
import { useState } from 'react';
import type { DesktopProjectRepository, ExportReceipt } from '../../domain/models';

export function ExportDialog({ onClose, repository, projectId }: { onClose: () => void; repository?: DesktopProjectRepository; projectId?: string }) {
  const [includePaths, setIncludePaths] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');
  const [receipt, setReceipt] = useState<ExportReceipt | null>(null);
  const save = async (format: 'json' | 'csv' | 'html' | 'print' | 'sidecar') => {
    if (!repository || !projectId) return;
    setBusy(true); setError(''); setReceipt(null);
    try { setReceipt(await repository.export(projectId, format, includePaths)); }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { setBusy(false); }
  };
  return <Modal title={zh.exportTitle} description={repository ? '从本地持久化快照导出。请选择新文件名，已有文件不会被覆盖。' : zh.exportPending} onClose={() => { if (!busy) onClose(); }}>
    <p className="muted">JSON 包含项目意图、人工选择与序列；CSV 用于选片索引。默认省略本机原图和缓存路径。</p>
    <label className="sync-toggle"><input type="checkbox" checked={includePaths} disabled={!repository || busy} onChange={event => setIncludePaths(event.target.checked)} />包含本机文件路径（迁移用途）</label>
    {includePaths && <p className="muted">导出文件将附加原图和缓存的位置引用，分享前请检查这些本机路径。</p>}
    <div className="export-options"><Button disabled={!repository || busy} variant="primary" onClick={() => { void save('json'); }}>项目 JSON</Button><Button disabled={!repository || busy} onClick={() => { void save('csv'); }}>选片 CSV</Button><Button disabled={!repository || busy} onClick={() => { void save('html'); }}>接触印相</Button><Button disabled={!repository || busy} onClick={() => { void save('print'); }}>印刷参考</Button><Button disabled={!repository || busy} onClick={() => { void save('sidecar'); }}>元数据侧车</Button></div>
    <div className="info-card"><span className="eyebrow">{repository ? 'LOCAL SNAPSHOT' : 'BROWSER PREVIEW'}</span><p>{repository ? '接触印相 HTML 内嵌小尺寸缓存预览，可在浏览器打印。导出不复制、不改写原始照片。' : '浏览器只提供内存 UI 示例；请在桌面版导出真实项目。'}</p></div>
    {receipt && <div role="status" className="export-receipt"><p>已导出：{receipt.path}</p><small>SHA256 {receipt.sha256}</small></div>}
    {error && <p role="alert" className="error-message">{error}</p>}
  </Modal>;
}
