import { cleanup, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { PublicationPanel } from './PublicationPanel';
import type { PublicationInfo, PublicationRepository } from '../../domain/publication';
afterEach(cleanup);

// 类型和日期同在一行里，按整段文本比对才不会被拆成多个节点所误伤。
const metaOf = (card: Element) => card.querySelector('.publication-card-meta')?.textContent ?? '';

const row = (over: Partial<PublicationInfo> = {}): PublicationInfo => ({
  id: 'book', projectId: 'project', pubType: 'photobook', title: '作品集', state: 'draft', createdAt: '2026-10-21', ...over,
});

it('共享项目的出版物可读取，但不能创建或生成', async () => {
  const repository: PublicationRepository = {
    list: vi.fn().mockResolvedValue([row()]),
    create: vi.fn(), generate: vi.fn(), pages: vi.fn(),
  };
  render(<PublicationPanel repository={repository} projectId="project" sequenceCount={5} readOnly />);
  await screen.findByText('作品集', { selector: 'strong' });
  expect(screen.getByRole('textbox', { name: '出版物标题' })).toBeDisabled();
  // 类型和开本也是写操作的一部分：只读项目里不能先把一本展览册"改成"摄影书再发现建不了。
  expect(screen.getByRole('combobox', { name: '类型' })).toBeDisabled();
  expect(screen.getByRole('combobox', { name: '开本' })).toBeDisabled();
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
  expect(repository.create).toHaveBeenCalledExactlyOnceWith('project', '光与停顿',
    { pubType: 'photobook', widthMm: 210, heightMm: 297 });
});

it('选了展览和 A3 就真的按这两项创建，不再被静默写成摄影书 A4', async () => {
  const repository: PublicationRepository = {
    list: vi.fn().mockResolvedValueOnce([]).mockResolvedValue([row({ id: 'wall', pubType: 'exhibition', title: '展墙册' })]),
    create: vi.fn().mockResolvedValue(row({ id: 'wall', pubType: 'exhibition', title: '展墙册' })),
    generate: vi.fn(), pages: vi.fn(),
  };
  const user = userEvent.setup();
  render(<PublicationPanel repository={repository} projectId="project" sequenceCount={5} />);
  await user.type(screen.getByRole('textbox', { name: '出版物标题' }), '展墙册');
  await user.selectOptions(screen.getByRole('combobox', { name: '类型' }), 'exhibition');
  await user.selectOptions(screen.getByRole('combobox', { name: '开本' }), 'a3');
  await user.click(screen.getByRole('button', { name: '创建' }));
  expect(repository.create).toHaveBeenCalledExactlyOnceWith('project', '展墙册',
    { pubType: 'exhibition', widthMm: 297, heightMm: 420 });
  // 卡片上的类型要跟着变，否则用户看不出自己建的到底是哪一种。
  const card = (await screen.findByText('展墙册', { selector: 'strong' })).closest('.publication-card')!;
  expect(metaOf(card)).toContain('展览');
  expect(metaOf(card)).not.toContain('photobook');
});

it('类型标签只认契约里的三种，库里出现陌生值就原样露出而不是套个中文标签', async () => {
  const repository: PublicationRepository = {
    list: vi.fn().mockResolvedValue([row({ title: '年历', pubType: 'calendar' })]),
    create: vi.fn(), generate: vi.fn(), pages: vi.fn(),
  };
  render(<PublicationPanel repository={repository} projectId="project" sequenceCount={5} />);
  const card = (await screen.findByText('年历', { selector: 'strong' })).closest('.publication-card')!;
  expect(metaOf(card)).toContain('calendar');
  expect(metaOf(card)).not.toContain('作品集');
});
