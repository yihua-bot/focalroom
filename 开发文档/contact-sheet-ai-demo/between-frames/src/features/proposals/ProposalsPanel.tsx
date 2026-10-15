import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import type { Project } from '../../domain/models';
import { TauriProposalsRepository } from '../../services/TauriProposalsRepository';
import { Button } from '../../components/shared/Button';
import { EmptyState } from '../../components/shared/EmptyState';

const stateLabels: Record<string, string> = { draft: '草稿', submitted: '待审批', applied: '已执行', discarded: '已驳回' };
const opLabels: Record<string, string> = { add: '加入序列', move: '移位', remove: '移出序列' };

export function ProposalsPanel({ project, enabled }: { project: Project; enabled: boolean }) {
  const client = useQueryClient();
  const repository = new TauriProposalsRepository();
  const query = useQuery({ queryKey: ['proposals', project.id], queryFn: () => repository.query(project.id), enabled, staleTime: 5_000 });
  const [openId, setOpenId] = useState<string | null>(null);
  const [accepted, setAccepted] = useState<Record<string, boolean>>({});
  const [nonce, setNonce] = useState('');
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState('');
  const projectRevision = () => {
    if (project.revision === undefined) throw new Error('项目版本缺失，请刷新后再审批');
    return project.revision;
  };
  const act = async (action: () => Promise<string>) => {
    if (busy) return; setBusy(true); setNote('');
    try { setNote(await action()); void query.refetch(); void client.invalidateQueries({ queryKey: ['projects'] }); }
    catch (error) { setNote(error instanceof Error ? error.message : String(error)); }
    finally { setBusy(false); }
  };
  const proposals = query.data ?? [];
  const open = proposals.find(p => p.id === openId) ?? null;
  return <section className="proposals-panel">
    <h3>编辑提案 · v2 审批</h3>
    <p className="micro">Agent 或协作者提出的操作化提案；执行必须由你确认 nonce 并勾选接受的操作。部分采纳、驳回均可；执行后可在工作台整体撤销。</p>
    {note && <p role="status">{note}</p>}
    {proposals.length === 0 && <EmptyState title="还没有提案" hint="委托 Agent 或经 v2 通道创建提案后，这里等待你的审批。" />}
    <ol className="proposal-list">{proposals.map(proposal => <li key={proposal.id}>
      <button aria-expanded={openId === proposal.id} onClick={() => { setOpenId(openId === proposal.id ? null : proposal.id); setAccepted(Object.fromEntries(proposal.operations.map(op => [op.operationId, true]))); setNonce(proposal.approvalNonce); setNote(''); }}>
        <strong>{proposal.title}</strong>
        <span className="micro">{stateLabels[proposal.state]} · {proposal.operations.length} 个操作 · {proposal.author} · v{proposal.revision}</span>
      </button>
    </li>)}</ol>
    {open && open.state === 'draft' && <div className="proposal-actions">
      <p className="micro">草稿待提交审批。</p>
      <Button size="small" disabled={busy} onClick={() => { void act(() => repository.submit(project.id, projectRevision(), open.id, open.revision, `submit-${Date.now()}`).then(() => '已提交审批')); }}>提交审批</Button>
    </div>}
    {open && open.state === 'submitted' && <div className="proposal-review">
      <h4>审批 · {open.title}</h4>
      <ul className="proposal-ops">{open.operations.map(op => <li key={op.operationId}>
        <label><input type="checkbox" checked={accepted[op.operationId] ?? false} onChange={event => setAccepted(previous => ({ ...previous, [op.operationId]: event.target.checked }))} />
          {opLabels[op.op]} · {op.photoId.slice(0, 8)}{op.toIndex !== null ? ` → 第 ${op.toIndex + 1} 位` : ''}{op.reason ? ` · ${op.reason}` : ''}</label>
      </li>)}</ul>
      <label>审批 nonce<input className="text-input" value={nonce} onChange={event => setNonce(event.target.value)} /></label>
      <p className="micro">nonce 与提案不符会被拒绝；不勾任何操作即整体驳回。</p>
      <div className="proposal-actions">
        <Button size="small" variant="primary" disabled={busy} onClick={() => { const ids = Object.entries(accepted).filter(([, on]) => on).map(([id]) => id); void act(() => repository.apply(project.id, projectRevision(), open.id, open.revision, ids, nonce.trim(), `apply-${Date.now()}`).then(result => result.discarded ? '已驳回该提案' : `已执行 ${result.applied} 个操作`)); }}>
          {Object.values(accepted).some(Boolean) ? '执行所选操作' : '驳回提案'}
        </Button>
      </div>
    </div>}
    {open && ['applied', 'discarded'].includes(open.state) && <p className="micro">该提案已{open.state === 'applied' ? '执行（可在工作台撤销）' : '驳回'}。</p>}
  </section>;
}
