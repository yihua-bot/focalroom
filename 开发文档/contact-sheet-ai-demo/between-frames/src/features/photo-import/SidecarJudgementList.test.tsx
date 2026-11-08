import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SidecarJudgementList } from './SidecarJudgementList';
import type { DesktopProjectRepository, SidecarProposal } from '../../domain/models';

const proposal = (over: Partial<SidecarProposal> = {}): SidecarProposal => ({
  photoId: 'photo-1',
  filename: 'frame-007.jpg',
  sidecarPath: '/Library/frame-007.xmp',
  sidecarRating: 4,
  localRating: 0,
  appliesRating: true,
  sidecarPick: true,
  localStatus: 'undecided',
  appliesStatus: true,
  colorLabel: 6,
  ambiguousRating: false,
  batchSafe: true,
  ...over,
});
const repository = (over: Record<string, unknown> = {}) => ({
  sidecarProposals: vi.fn().mockResolvedValue([proposal()]),
  applySidecarJudgement: vi.fn().mockResolvedValue({ id: 'p1', revision: 5 }),
  backfillSidecarJudgements: vi.fn().mockResolvedValue({ applied: 1, skipped: 0 }),
  ...over,
}) as unknown as DesktopProjectRepository;
const panel = async () => within(await screen.findByRole('region', { name: '外部软件里的判断' }));

describe('外部软件里的判断回流', () => {
  it('不点就不读盘，读了也只说事实不说已经改过', async () => {
    const repo = repository();
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    expect(screen.getByRole('button', { name: '读取外部判断' })).toBeInTheDocument();
    expect(screen.queryByText(/外部说/)).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    expect(repo.sidecarProposals).toHaveBeenCalledWith('p1');
    expect(await screen.findByText('外部有 1 条本地还没有的判断，确认后才写进记录。')).toBeInTheDocument();
    expect(screen.getByText('外部说：4 星 · 已选中 · 色标签 6 → 本地记的：0 星 · 待定')).toBeInTheDocument();
    expect(screen.queryByText('已回填这条记录')).not.toBeInTheDocument();
  });

  it('读过的这一屏还能再读一遍，不用重开导入', async () => {
    const repo = repository();
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    const rescan = await screen.findByRole('button', { name: '重新读取' });
    await userEvent.click(rescan);
    expect(repo.sidecarProposals).toHaveBeenCalledTimes(2);
    await waitFor(() => expect(rescan).toBeEnabled());
  });

  it('确认只发这条照片编号，成功后按钮换成已回填并通知刷新', async () => {
    const repo = repository();
    const onApplied = vi.fn();
    render(<SidecarJudgementList repository={repo} projectId="p1" onApplied={onApplied} />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    await userEvent.click(await screen.findByRole('button', { name: '回填这一条' }));
    expect(repo.applySidecarJudgement).toHaveBeenCalledWith('p1', 'photo-1');
    expect(await screen.findByText('已回填这条记录')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '回填这一条' })).not.toBeInTheDocument();
    expect(onApplied).toHaveBeenCalledTimes(1);
  });

  it('后端拒掉时说清原因，不冒充已回填，也还能再点一次', async () => {
    const repo = repository({ applySidecarJudgement: vi.fn().mockRejectedValue(new Error('本地判断已经和这张照片的边车一致，无需回填')) });
    const onApplied = vi.fn();
    render(<SidecarJudgementList repository={repo} projectId="p1" onApplied={onApplied} />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    await userEvent.click(await screen.findByRole('button', { name: '回填这一条' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('本地判断已经和这张照片的边车一致，无需回填');
    expect(screen.queryByText('已回填这条记录')).not.toBeInTheDocument();
    expect(onApplied).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: '回填这一条' })).toBeEnabled();
  });

  it('一键只报本地还空着的那几条，只发项目编号，落库后按现在的台账重读一遍', async () => {
    const repo = repository();
    const onApplied = vi.fn();
    render(<SidecarJudgementList repository={repo} projectId="p1" onApplied={onApplied} />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    await userEvent.click(await screen.findByRole('button', { name: '一次填上 1 条空白' }));
    expect(repo.backfillSidecarJudgements).toHaveBeenCalledWith('p1');
    expect(await screen.findByText('填了 1 条；0 条你自己改过，没动。')).toBeInTheDocument();
    expect(repo.sidecarProposals).toHaveBeenCalledTimes(2);
    expect(onApplied).toHaveBeenCalledTimes(1);
    const region = await panel();
    expect(region.getByText('外部说：4 星 · 已选中 · 色标签 6 → 本地记的：0 星 · 待定')).toBeInTheDocument();
  });

  it('一键被拒时不谎报填了几条，清单也不跟着重读', async () => {
    const repo = repository({ backfillSidecarJudgements: vi.fn().mockRejectedValue(new Error('这些判断在你点一键回填之前已经被你自己改过了，一条都没有动')) });
    const onApplied = vi.fn();
    render(<SidecarJudgementList repository={repo} projectId="p1" onApplied={onApplied} />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    await userEvent.click(await screen.findByRole('button', { name: '一次填上 1 条空白' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('这些判断在你点一键回填之前已经被你自己改过了，一条都没有动');
    expect(screen.queryByText(/填了 1 条/)).not.toBeInTheDocument();
    expect(repo.sidecarProposals).toHaveBeenCalledTimes(1);
    expect(onApplied).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: '一次填上 1 条空白' })).toBeEnabled();
  });

  it('一条都填不成时不摆一键的按钮，只说这些得逐条点头', async () => {
    const repo = repository({ sidecarProposals: vi.fn().mockResolvedValue([proposal({ batchSafe: false })]) });
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    expect(await screen.findByText('只有本地空着的能一次填，判过的要逐条确认。')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /一次填上/ })).not.toBeInTheDocument();
    expect(repo.backfillSidecarJudgements).not.toHaveBeenCalled();
  });

  it('没有一键通道的仓储不摆兑现不了的按钮，逐条还是走', async () => {
    const repo = repository({ backfillSidecarJudgements: undefined });
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    expect(await screen.findByRole('button', { name: '回填这一条' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /一次填上/ })).not.toBeInTheDocument();
    expect(screen.queryByText(/判过的要逐条确认/)).not.toBeInTheDocument();
  });

  it('两处评分互相矛盾的那条不给按钮，只说要先统一', async () => {
    const repo = repository({ sidecarProposals: vi.fn().mockResolvedValue([proposal({ sidecarRating: null, localRating: 2, appliesRating: false, sidecarPick: null, localStatus: 'keep', appliesStatus: false, colorLabel: null, ambiguousRating: true, batchSafe: false })]) });
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    expect(await screen.findByText('两处评分互相矛盾，先在外部软件里统一')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '回填这一条' })).not.toBeInTheDocument();
    expect(repo.applySidecarJudgement).not.toHaveBeenCalled();
  });

  it('外部写着未选中不等于淘汰：报得出来，本地不动', async () => {
    const repo = repository({ sidecarProposals: vi.fn().mockResolvedValue([proposal({ sidecarRating: 3, localRating: 3, appliesRating: false, sidecarPick: false, localStatus: 'keep', appliesStatus: false, batchSafe: false })]) });
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    expect(await screen.findByText('外部说：3 星 · 未选中 · 色标签 6 → 本地记的：3 星 · 已保留')).toBeInTheDocument();
    expect(screen.getByText('外部写未选中，不等于淘汰，本地不动')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: '回填这一条' })).not.toBeInTheDocument();
  });

  it('旁边什么都没有时说的是没有，不是空列表', async () => {
    const repo = repository({ sidecarProposals: vi.fn().mockResolvedValue([]) });
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    expect(await screen.findByText('旁边没有评分或选片旗标。')).toBeInTheDocument();
    expect(screen.queryByRole('listitem')).not.toBeInTheDocument();
  });

  it('读不到时如实报错，回头还能再读一次', async () => {
    const repo = repository({ sidecarProposals: vi.fn().mockRejectedValue(new Error('旁边的 XMP 文件过大，不当成这张照片的判断')) });
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    await userEvent.click(screen.getByRole('button', { name: '读取外部判断' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('旁边的 XMP 文件过大，不当成这张照片的判断');
    expect(screen.queryByText(/外部说/)).not.toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('button', { name: '读取外部判断' })).toBeEnabled());
  });

  it('没有这条通道的仓储不给无法兑现的按钮', () => {
    const repo = repository({ sidecarProposals: undefined, applySidecarJudgement: undefined, backfillSidecarJudgements: undefined });
    render(<SidecarJudgementList repository={repo} projectId="p1" />);
    const scan = screen.getByRole('button', { name: '读取外部判断' });
    expect(scan).toBeDisabled();
    expect(screen.queryByRole('button', { name: '回填这一条' })).not.toBeInTheDocument();
  });
});
