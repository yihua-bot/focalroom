import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { SharedReviewLink } from './SharedReviewLink';

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => true, invoke: vi.fn() }));

it('原生入口只提交项目身份，系统打开失败时保留可复制地址', async () => {
  vi.mocked(invoke).mockRejectedValueOnce(new Error('OS opener unavailable'));
  render(<SharedReviewLink projectId="owned-project" url="https://review.example/shared#project=cloud-project" />);
  await userEvent.click(screen.getByRole('link', { name: '打开共享评片 ↗' }));
  expect(invoke).toHaveBeenCalledWith('open_shared_review', { projectId: 'owned-project' });
  expect(await screen.findByRole('alert')).toHaveTextContent('https://review.example/shared#project=cloud-project');
  expect(screen.getByRole('alert')).not.toHaveTextContent('OS opener unavailable');
});
