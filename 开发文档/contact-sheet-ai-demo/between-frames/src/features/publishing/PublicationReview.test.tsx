import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { PublicationRepository, PublicationReview } from '../../domain/publication';
import { bundles, setCopyLocale } from '../../app/copy';
import { PublicationReviewPanel } from './PublicationReview';

const book = 'b2c3d4e5-f6a7-4b8c-9d0e-1f2a3b4c5d6e';
const thread = { id: 'c3d4e5f6-a7b8-4c9d-8e0f-2a3b4c5d6e7f', revision: 7, state: 'open' as const, comments: [{ id: 1, body: '第 3 页留白太多，压不住后面的跨页', author: 'user:local-owner', createdAt: '2026-11-05T02:00:00Z' }] };
const page = (over: Partial<PublicationReview> = {}): PublicationReview => ({ publicationId: book, revision: 7, open: 1, threads: [thread], ...over });
const repo = (over: Record<string, unknown> = {}) => ({ pendingCommands: () => [], review: vi.fn().mockResolvedValue(page()), addReview: vi.fn().mockResolvedValue(page({ open: 2, revision: 8 })), resolveReview: vi.fn().mockResolvedValue(page({ open: 0, threads: [{ ...thread, state: 'resolved' as const }] })), ...over } as unknown as PublicationRepository);
const renderPanel = (repository: PublicationRepository, readOnly = false) => render(<PublicationReviewPanel repository={repository} projectId="p1" publicationId={book} readOnly={readOnly} />);

beforeEach(() => setCopyLocale('zh'));
afterEach(() => cleanup());

it('意见带着"哪一版提的"和作者上屏，未处理几条一句话说清', async () => {
  const repository = repo();
  renderPanel(repository);
  expect(await screen.findByText('第 3 页留白太多，压不住后面的跨页')).toBeTruthy();
  expect(screen.getByText('第 7 版')).toBeTruthy();
  expect(screen.getByText('user:local-owner')).toBeTruthy();
  expect(screen.getByRole('status').textContent).toBe('还有 1 条意见没处理');
  expect(repository.review).toHaveBeenCalledWith('p1', book);
});

it('没有意见时说的是没有，不摆一个空列表让用户猜', async () => {
  const repository = repo({ review: vi.fn().mockResolvedValue(page({ open: 0, threads: [] })) });
  renderPanel(repository);
  expect(await screen.findByText('还没有审阅意见')).toBeTruthy();
  expect(screen.getByRole('status').textContent).toBe('审阅意见都已处理');
});

it('提交意见只送正文，上屏的是后端刷新回来的那份', async () => {
  const user = userEvent.setup();
  const repository = repo();
  renderPanel(repository);
  await screen.findByText('第 3 页留白太多，压不住后面的跨页');
  const field = screen.getByRole('textbox', { name: '写下审阅意见' });
  expect(screen.getByRole('button', { name: '提交意见' })).toBeDisabled();
  await user.type(field, '  封面那张该往后挪  ');
  await user.click(screen.getByRole('button', { name: '提交意见' }));
  await waitFor(() => expect(repository.addReview).toHaveBeenCalledWith('p1', book, '封面那张该往后挪'));
  expect(await screen.findByText('还有 2 条意见没处理')).toBeTruthy();
  // 输入框清空、旧意见仍在：提交成功不该把已读到的那份丢掉。
  expect(field).toHaveValue('');
  expect(screen.getByText('第 3 页留白太多，压不住后面的跨页')).toBeTruthy();
});

it('处理与重开都点明是这条线程，方向相反', async () => {
  const user = userEvent.setup();
  const repository = repo();
  renderPanel(repository);
  await user.click(await screen.findByRole('button', { name: '标记为已处理' }));
  expect(repository.resolveReview).toHaveBeenCalledWith('p1', book, thread.id, true);
  expect(await screen.findByText('审阅意见都已处理')).toBeTruthy();
  expect(screen.getByText('已处理')).toBeTruthy();
  await user.click(screen.getByRole('button', { name: '重新打开' }));
  expect(repository.resolveReview).toHaveBeenLastCalledWith('p1', book, thread.id, false);
});

it('只读视图读得到意见，但一个写入口都不给', async () => {
  const repository = repo();
  renderPanel(repository, true);
  expect(await screen.findByText('第 3 页留白太多，压不住后面的跨页')).toBeTruthy();
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(screen.getByRole('button', { name: '标记为已处理' })).toBeDisabled();
  expect(repository.addReview).not.toHaveBeenCalled();
  expect(repository.resolveReview).not.toHaveBeenCalled();
});

it('读不到意见就明说，不让用户在半屏空白上按定稿', async () => {
  const repository = repo({ review: vi.fn().mockRejectedValue(new Error('暂时读不到审阅意见')) });
  renderPanel(repository);
  expect(await screen.findByRole('alert')).toBeTruthy();
  expect(screen.queryByText('审阅意见都已处理')).toBeNull();
  expect(screen.queryByRole('status')).toBeNull();
});

const KEYS = ['publicationReview', 'publicationReviewHint', 'publicationReviewOpen', 'publicationReviewSettled', 'publicationReviewEmpty', 'publicationReviewOnVersion', 'publicationReviewResolved', 'publicationReviewResolve', 'publicationReviewReopen', 'publicationReviewBody', 'publicationReviewSend', 'publicationReviewFailed', 'publicationReviewReload'];

it('四套语言都补齐了审阅意见文案，回落中文不算翻译完成', async () => {
  const locales = ['zh', 'en', 'ja', 'ko'] as const;
  const text = (locale: (typeof locales)[number], key: string) => (bundles[locale].copy as Record<string, string>)[key];
  for (const locale of locales) for (const key of KEYS) expect(text(locale, key), `${locale}/${key}`).toBeTruthy();
  setCopyLocale('en');
  renderPanel(repo());
  expect(await screen.findByText('1 comment(s) still unresolved')).toBeTruthy();
  expect(screen.getByText('Version 7')).toBeTruthy();
  expect(text('ja', 'publicationReviewResolve')).not.toBe(text('zh', 'publicationReviewResolve'));
  expect(text('ko', 'publicationReviewResolve')).not.toBe(text('zh', 'publicationReviewResolve'));
  expect(text('ja', 'publicationReviewResolve')).not.toMatch(/\p{Script=Hangul}/u);
  setCopyLocale('zh');
});
