import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { PreferenceProposalCard } from './PreferenceProposalCard';
import type { PreferenceProposal } from '../../domain/agent';
const proposal: PreferenceProposal = { id: 'p', statement: '保留留白', applicability: '仅本项目', photoIds: [], sources: [{ messageId: 1, text: '我想留下更多留白', createdAt: 'now' }], createdAt: 'now', state: 'pending' };
it('保存前允许编辑并查看原话，不会因打开卡片自动写入记忆', async () => {
 const onSave = vi.fn().mockResolvedValue(undefined), onDismiss = vi.fn(); const user = userEvent.setup();
 render(<PreferenceProposalCard proposal={proposal} disabled={false} onSave={onSave} onDismiss={onDismiss} />);
 expect(onSave).not.toHaveBeenCalled();
 await user.click(screen.getByText('你的原话')); expect(screen.getByText('我想留下更多留白')).toBeVisible();
 await user.clear(screen.getByRole('textbox')); await user.type(screen.getByRole('textbox'), '只在开篇保留留白');
 await user.click(screen.getByRole('button', { name: '保存到本项目' }));
 await waitFor(() => expect(onSave).toHaveBeenCalledExactlyOnceWith('只在开篇保留留白'));
 expect(onDismiss).not.toHaveBeenCalled();
});
it('忽略与保存分开，已保存的建议不允许再次确认', async () => {
 const onSave = vi.fn(), onDismiss = vi.fn().mockResolvedValue(undefined);
 const { rerender } = render(<PreferenceProposalCard proposal={proposal} disabled={false} onSave={onSave} onDismiss={onDismiss} />);
 await userEvent.click(screen.getByRole('button', { name: '忽略' })); expect(onDismiss).toHaveBeenCalledOnce(); expect(onSave).not.toHaveBeenCalled();
 rerender(<PreferenceProposalCard proposal={{ ...proposal, state: 'saved' }} disabled={false} onSave={onSave} onDismiss={onDismiss} />);
 expect(screen.queryByRole('button', { name: '保存到本项目' })).not.toBeInTheDocument(); expect(screen.getByRole('status')).toHaveTextContent('已保存');
});
