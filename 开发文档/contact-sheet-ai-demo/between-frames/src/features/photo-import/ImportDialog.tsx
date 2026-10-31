// 导入照片：支持后台导入（关闭弹窗后显示底部进度条，可展开）。
import { useEffect, useState } from 'react';
import { userFacingError } from '../../services/userFacingError';
import type { DesktopProjectRepository, ImportJob } from '../../domain/models';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';

/** 后台导入指示器：导入进行中固定在底部，可展开详情。 */
export function ImportBanner({ repository, jobId, onDone }: {
  repository: DesktopProjectRepository; jobId: string; onDone: () => void;
}) {
  const [job, setJob] = useState<ImportJob | null>(null);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await repository.importJob(jobId);
        if (stopped) return;
        setJob(next);
        if (next.status === 'running') timer = setTimeout(() => { void poll(); }, 500);
        else onDone();
      } catch { if (!stopped) onDone(); }
    };
    void poll();
    return () => { stopped = true; clearTimeout(timer); };
  }, [repository, jobId, onDone]);

  const running = !job || job.status === 'running';
  const pct = job && job.total > 0 ? Math.round((job.completed / job.total) * 100) : 0;

  return <div className={`import-banner ${expanded ? 'expanded' : ''}`} role="status" aria-label="后台导入">
    <button className="import-banner-header" onClick={() => setExpanded(!expanded)}>
      <span className="import-banner-icon" aria-hidden>📥</span>
      <span className="import-banner-label">{running ? '正在导入' : job?.status === 'completed' ? '导入完成' : '已取消'} · {job?.completed ?? 0} / {job ? job.total : '…'}{job && job.total > 0 ? `（${pct}%）` : ''}</span>
      <progress className="import-banner-bar" max={job?.total || 1} value={job?.completed || 0} />
      <span className="import-banner-toggle" aria-hidden>{expanded ? '▼' : '▲'}</span>
    </button>
    {expanded && <div className="import-banner-detail">
      <p className="muted">新增 {job?.imported ?? 0} 张 · 重复 {job?.skipped ?? 0} 张 · 失败 {job?.failures?.length ?? 0} 张</p>
      {running && <Button size="small" onClick={() => { void repository.cancelImport(jobId); }}>取消导入</Button>}
      {!!job?.failures?.length && <ul className="import-failures">{job.failures.map((f, i) => <li key={`${f.path}-${i}`}><strong>{f.path.split('/').pop()}</strong><p>{userFacingError(new Error(f.message), '无法读取这张照片。')}</p></li>)}</ul>}
      {!running && <Button size="small" onClick={onDone}>知道了</Button>}
    </div>}
  </div>;
}

export function ImportDialog({ repository, projectId, onClose, onChanged, onBackground }: {
  repository: DesktopProjectRepository; projectId: string;
  onClose: () => void; onChanged: () => void;
  onBackground?: (jobId: string) => void;
}) {
  const [jobId, setJobId] = useState<string | null>(null);
  const [job, setJob] = useState<ImportJob | null>(null);
  const [choosing, setChoosing] = useState(false);
  const [error, setError] = useState('');
  useEffect(() => {
    if (!jobId) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await repository.importJob(jobId);
        if (stopped) return;
        setJob(next);
        if (next.status === 'running') timer = setTimeout(() => { void poll(); }, 300);
        else onChanged();
      } catch (failure) { if (!stopped) setError(failure instanceof Error ? failure.message : String(failure)); }
    };
    void poll();
    return () => { stopped = true; clearTimeout(timer); };
  }, [repository, jobId, onChanged]);
  const start = async (folder: boolean) => {
    setChoosing(true); setError('');
    try { const id = await repository.import(projectId, folder); if (id) { setJob(null); setJobId(id); } }
    catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
    finally { setChoosing(false); }
  };
  const running = Boolean(jobId && (!job || job.status === 'running') && !error);
  const canBackground = Boolean(running && jobId && onBackground);
  const background = () => { if (jobId && onBackground) { onBackground(jobId); onClose(); } else if (!running) onClose(); };
  return <Modal title="导入照片" description="原图保留在原目录。"
    onClose={() => { if (choosing) return; if (running && jobId && onBackground) { background(); return; } if (!running) onClose(); }}>
    {!running && <div className="export-options"><Button variant="primary" disabled={choosing} onClick={() => { void start(false); }}>选择照片</Button>
      <Button disabled={choosing} onClick={() => { void start(true); }}>选择文件夹</Button></div>}
    <details><summary>支持的格式</summary><p>JPEG、PNG、WebP、TIFF。HEIC 和 RAW 支持情况取决于系统解码能力。</p></details>
    {choosing && <p role="status" className="muted">等待系统文件选择器…</p>}
    {jobId && <section className="import-progress" aria-label="导入进度"><progress max={job?.total || 1} value={job?.completed || 0} />
      <p role="status">{job?.status === 'cancelled' ? '已取消；已完成照片保留' : job?.status === 'completed' ? '导入完成' : job && job.total === 0 ? '正在扫描图库' : '正在导入'} · {job?.completed || 0} / {job ? job.total : '…'}</p>
      <p className="muted">新增 {job?.imported || 0} 张 · 重复跳过 {job?.skipped || 0} 张 · 失败 {job?.failures.length || 0} 张</p>
      <div className="import-dialog-actions">
        {running && <Button onClick={background}>后台导入</Button>}
        {running && <Button onClick={() => { void repository.cancelImport(jobId).catch(failure => setError(String(failure))); }}>取消导入</Button>}
      </div>
      {!!job?.failures.length && <ul className="import-failures">{job.failures.map((failure, index) => <li key={`${failure.path}-${index}`}><strong>{failure.path.split('/').pop()}</strong><p>{userFacingError(new Error(failure.message), '无法读取这张照片，请确认文件完整且格式受支持。')}</p></li>)}</ul>}
      {!running && !!job?.failures.length && <Button onClick={() => { void repository.retryImport(jobId).then(id => { setJob(null); setJobId(id); setError(''); }).catch(failure => setError(String(failure))); }}>重试失败照片</Button>}
    </section>}
    {error && <p role="alert" className="error-message">{error}</p>}
    <div className="dialog-actions"><Button disabled={choosing || (running && !canBackground)} onClick={canBackground ? background : onClose}>{canBackground ? '后台继续' : '完成'}</Button></div>
  </Modal>;
}
