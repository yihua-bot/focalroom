import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { PublicationPanel } from './PublicationPanel';
import type { PublicationRepository } from '../../domain/publication';
afterEach(cleanup);

it('共享项目的出版物可读取，但不能创建或生成', async () => {
  const repository: PublicationRepository = {
    list: vi.fn().mockResolvedValue([{ id: 'book', projectId: 'project', pubType: 'photobook', title: '作品集', state: 'draft', createdAt: '2026-10-21' }]),
    create: vi.fn(), generate: vi.fn(), pages: vi.fn(),
  };
  render(<PublicationPanel repository={repository} projectId="project" sequenceCount={5} readOnly />);
  await screen.findByText('作品集', { selector: 'strong' });
  expect(screen.getByRole('textbox', { name: '出版物标题' })).toBeDisabled();
  expect(screen.getByRole('button', { name: '创建' })).toBeDisabled();
  expect(screen.getByRole('button', { name: /从序列生成/ })).toBeDisabled();
  expect(repository.create).not.toHaveBeenCalled();
  expect(repository.generate).not.toHaveBeenCalled();
});

it('创建失败保留输入并显示错误，不冒充成功或产生未处理 Promise', async () => {
  const repository: PublicationRepository = {
    list: vi.fn().mockResolvedValue([]),
    create: vi.fn().mockRejectedValue(new Error('项目已归档')),
    generate: vi.fn(), pages: vi.fn(),
  };
  const user = userEvent.setup();
  render(<PublicationPanel repository={repository} projectId="project" sequenceCount={5} />);
  await user.type(screen.getByRole('textbox', { name: '出版物标题' }), '光与停顿');
  await user.click(screen.getByRole('button', { name: '创建' }));
  expect(await screen.findByRole('status')).toHaveTextContent('项目已归档');
  expect(screen.getByRole('textbox', { name: '出版物标题' })).toHaveValue('光与停顿');
  expect(repository.create).toHaveBeenCalledExactlyOnceWith('project', '光与停顿');
});
