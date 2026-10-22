import { useEffect, useState } from 'react';
import type { EditorVersion, EditorVersionsRepository } from '../../domain/editorVersions';
import { Button } from '../../components/shared/Button';
import { Modal } from '../../components/shared/Modal';
import { useI18n } from '../../i18n';

export function EditorVersionsDialog({ repository, projectId, readOnly = false, onClose }: {
  repository: EditorVersionsRepository; projectId: string; readOnly?: boolean; onClose: () => void;
}) {
  const { t } = useI18n();
  const [versions, setVersions] = useState<EditorVersion[]>([]);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [active, setActive] = useState<EditorVersion | null>(null);
  useEffect(() => {
    let stopped = false;
    repository.list(projectId).then(data => { if (!stopped) setVersions(data); })
      .catch(() => { if (!stopped) setError(t('editor.loadError', '调色版本暂时无法加载，请关闭后重试。')); })
      .finally(() => { if (!stopped) setLoading(false); });
    return () => { stopped = true; };
  }, [repository, projectId, t]);
  const importResults = async () => {
    setBusy(true); setError(''); setNotice('');
    try {
      const count = await repository.import(projectId);
      if (count !== null) {
        setVersions(await repository.list(projectId));
        setNotice(count ? t('editor.imported', '调色版本已保存。原照片与选片保持不变。') : t('editor.duplicate', '这些调色版本已经导入，无需重复添加。'));
      }
    } catch (failure) { setError(failure instanceof Error ? failure.message : t('editor.importError', '导入未完成，请确认照片文件夹完整。')); }
    finally { setBusy(false); }
  };
  return <Modal title={t('editor.title', '调色版本')} wide onClose={() => { if (!busy) onClose(); }}
    description={t('editor.description', '查看 Lightroom Classic 或 Capture One 导出的调色结果。原照片与正式作品序列保持不变。')}>
    <div className="dialog-actions"><a href="https://focroom.cc/plugins/" target="_blank" rel="noreferrer">{t('editor.guide', '插件下载与安装说明')} ↗</a>
      <Button variant="primary" disabled={busy || loading || readOnly} onClick={() => { void importResults(); }}>{busy ? t('editor.working', '正在准备照片…') : t('editor.import', '导入调色结果')}</Button></div>
    {loading && <p role="status">{t('editor.loading', '正在读取调色版本…')}</p>}
    {error && <p role="alert" className="error-message">{error}</p>}
    {notice && <p role="status">{notice}</p>}
    {!loading && !versions.length && <p className="muted">{t('editor.empty', '先从插件导出照片，再选择生成的 FocalRoom 文件夹。请先在当前项目导入对应的原照片。')}</p>}
    {active && <button className="editor-version-large" onClick={() => setActive(null)} aria-label={t('editor.collapse', '收起大图')}><img src={active.previewSrc} alt={active.photoName} /></button>}
    <div className="editor-version-grid">{versions.map(version => <button key={version.id} className="editor-version-card" onClick={() => setActive(version)}>
      <img src={version.previewSrc} alt={version.photoName} loading="lazy" /><strong>{version.photoName}</strong>
      <span>{version.provider === 'lightroom_classic' ? 'Lightroom Classic' : 'Capture One'}</span>
    </button>)}</div>
    <div className="dialog-actions"><Button disabled={busy} onClick={onClose}>{t('editor.done', '完成')}</Button></div>
  </Modal>;
}
