import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import type { DiagnosticsRepository, FileUsage } from '../../domain/diagnostics';
import { Modal } from '../../components/shared/Modal';
import { Button } from '../../components/shared/Button';

function bytes(size: number) {
  if (size < 1024) return `${size} B`;
  if (size < 1024 ** 2) return `${(size / 1024).toFixed(1)} KiB`;
  if (size < 1024 ** 3) return `${(size / 1024 ** 2).toFixed(1)} MiB`;
  return `${(size / 1024 ** 3).toFixed(2)} GiB`;
}
function amount(part: FileUsage) {
  if (!part.complete && part.files === 0) return '暂不可用';
  return `${part.complete ? '' : '至少 '}${bytes(part.bytes)}`;
}

export function DiagnosticsDialog({ repository, onClose }: { repository: DiagnosticsRepository; onClose: () => void }) {
  const query = useQuery({ queryKey: ['diagnostics'], queryFn: () => repository.query(), retry: false, refetchOnWindowFocus: false, staleTime: 0 });
  const [note, setNote] = useState('');
  const [copying, setCopying] = useState(false);
  const data = query.data;
  const copy = async () => {
    if (!data || copying || query.isFetching || query.isError) return;
    setCopying(true); setNote('');
    try { await navigator.clipboard.writeText(JSON.stringify(data, null, 2)); setNote('支持信息已复制。'); }
    catch { setNote('无法复制，请重试或联系支持。'); }
    finally { setCopying(false); }
  };
  return <Modal title="存储空间" description="查看项目数据、照片预览与备份占用。原图不在统计中。"
    onClose={onClose}>
    <div className="dialog-actions">
      <Button size="small" disabled={query.isFetching || copying} onClick={() => { setNote(''); void query.refetch(); }}>刷新统计</Button>
      <Button size="small" disabled={!data || query.isFetching || query.isError || copying} onClick={() => { void copy(); }}>复制支持信息</Button>
    </div>
    {query.isFetching && <p role="status">正在读取本地统计…</p>}
    {query.isError && <p role="alert">读取失败，请重试。已显示的数据为上次结果。</p>}
    {data && <div className="storage-diagnostics" aria-busy={query.isFetching}>
      <p className="micro">更新时间：{new Date(data.collectedAt).toLocaleString('zh-CN')} · FocalRoom {data.appVersion}</p>
      <h3>存储统计</h3>
      <table><thead><tr><th scope="col">类别</th><th scope="col">已统计大小</th><th scope="col">文件数</th></tr></thead>
        <tbody>{([
          ['项目数据', data.storage.database], ['最近更改', data.storage.wal], ['运行数据', data.storage.sharedMemory],
          ['图片缓存', data.storage.cache], ['项目备份', data.storage.backups],
        ] as const).map(([label, part]) => <tr key={label}><th scope="row">{label}</th><td>{amount(part)}{!part.complete && <small> · 统计不完整</small>}</td><td>{part.files}</td></tr>)}</tbody>
      </table>
      <p>已统计文件合计：{data.storage.complete ? '' : '至少 '}{bytes(data.storage.totalBytes)}</p>
      {data.storage.backupsChecked.length > 0 && <div className="backup-health">
        <h4>备份检查（最近 {data.storage.backupsChecked.length} 份）</h4>
        <ul>{data.storage.backupsChecked.map(backup => <li key={backup.name} data-status={backup.status}>
          {backup.status === 'ok' ? '✓ 完整性通过' : backup.status === 'corrupt' ? '⚠ 完整性异常' : '⚠ 无法打开'} · {bytes(backup.bytes)} · {backup.name}
        </li>)}</ul>
        <p className="micro">建议定期确认备份可以正常恢复。</p>
      </div>}
      <p className="micro">照片预览存储上限：{bytes(data.storage.cacheLimitBytes)}。正在使用的照片预览会保留。</p>
      {data.storage.cache.bytes > data.storage.cacheLimitBytes && <p role="status">预览占用已超过上限；正在使用的照片预览会保留。</p>}
      {!data.storage.complete && <p role="status">部分内容暂时无法统计，显示的占用可能小于实际值。</p>}
      <h3>数据与任务</h3>
      <dl className="diagnostic-counts">
        <div><dt>项目（含归档）</dt><dd>{data.database.projects}</dd></div>
        <div><dt>照片</dt><dd>{data.database.photos}</dd></div>
        <div><dt>AI 选片任务</dt><dd>{data.database.agentTasks}</dd></div>
        <div><dt>失败任务</dt><dd>{data.failedAgentTasks}</dd></div>
        <div><dt>偏好记录</dt><dd>{data.database.evidence}</dd></div>
      </dl>
    </div>}
    {note && <p role="status">{note}</p>}
  </Modal>;
}
