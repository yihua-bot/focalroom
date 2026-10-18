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
    try { await navigator.clipboard.writeText(JSON.stringify(data, null, 2)); setNote('诊断信息已复制。'); }
    catch { setNote('剪贴板写入失败；可以展开诊断 JSON 后手动复制。'); }
    finally { setCopying(false); }
  };
  return <Modal title="本地存储与诊断" description="查看数据库、图片缓存与备份占用，不含原图。"
    onClose={onClose}>
    <div className="dialog-actions">
      <Button size="small" disabled={query.isFetching || copying} onClick={() => { setNote(''); void query.refetch(); }}>刷新统计</Button>
      <Button size="small" disabled={!data || query.isFetching || query.isError || copying} onClick={() => { void copy(); }}>复制诊断信息</Button>
    </div>
    {query.isFetching && <p role="status">正在读取本地统计…</p>}
    {query.isError && <p role="alert">读取失败，请重试。已显示的数据为上次结果。</p>}
    {data && <div className="storage-diagnostics" aria-busy={query.isFetching}>
      <p className="micro">采集时间：{new Date(data.collectedAt).toLocaleString('zh-CN')} · 应用 {data.appVersion} · 提供方 {data.provider.providerId === 'mock' ? '演示模式' : 'OpenAI'} · {data.provider.ready ? '密钥已配置' : '密钥未配置'}</p>
      <h3>存储统计</h3>
      <table><thead><tr><th scope="col">类别</th><th scope="col">已统计大小</th><th scope="col">文件数</th></tr></thead>
        <tbody>{([
          ['数据库', data.storage.database], ['数据库 WAL', data.storage.wal], ['数据库共享内存', data.storage.sharedMemory],
          ['图片缓存', data.storage.cache], ['迁移前备份', data.storage.backups],
        ] as const).map(([label, part]) => <tr key={label}><th scope="row">{label}</th><td>{amount(part)}{!part.complete && <small> · 统计不完整</small>}</td><td>{part.files}</td></tr>)}</tbody>
      </table>
      <p>已统计文件合计：{data.storage.complete ? '' : '至少 '}{bytes(data.storage.totalBytes)}</p>
      {data.storage.backupsChecked.length > 0 && <div className="backup-health">
        <h4>备份完整性巡检（最近 {data.storage.backupsChecked.length} 份）</h4>
        <ul>{data.storage.backupsChecked.map(backup => <li key={backup.name} data-status={backup.status}>
          {backup.status === 'ok' ? '✓ 完整性通过' : backup.status === 'corrupt' ? '⚠ 完整性异常' : '⚠ 无法打开'} · {bytes(backup.bytes)} · {backup.name}
        </li>)}</ul>
        <p className="micro">完整性检查通过后，仍需验证备份能否恢复。</p>
      </div>}
      <p className="micro">缓存配置额度：{bytes(data.storage.cacheLimitBytes)}。仅自动清理未使用的缓存。</p>
      {data.storage.cache.bytes > data.storage.cacheLimitBytes && <p role="status">已统计缓存超过配置额度；引用缓存不会自动淘汰。</p>}
      {!data.storage.complete && <p role="status">部分条目不可读取、不是普通文件或超出扫描上限；仅显示已统计下界。不会跟随符号链接。</p>}
      <h3>数据与任务</h3>
      <dl className="diagnostic-counts">
        <div><dt>项目（含归档）</dt><dd>{data.database.projects}</dd></div>
        <div><dt>照片引用</dt><dd>{data.database.photos}</dd></div>
        <div><dt>Agent 任务</dt><dd>{data.database.agentTasks}</dd></div>
        <div><dt>失败任务</dt><dd>{data.failedAgentTasks}</dd></div>
        <div><dt>人工证据</dt><dd>{data.database.evidence}</dd></div>
        <div><dt>已应用迁移</dt><dd>{data.migrationsApplied}</dd></div>
      </dl>
      <details><summary>检查诊断 JSON</summary><pre tabIndex={0} aria-label="脱敏诊断 JSON">{JSON.stringify(data, null, 2)}</pre></details>
    </div>}
    {note && <p role="status">{note}</p>}
  </Modal>;
}
