import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import App from '../App';
import { MockRepository } from '../services/MockRepository';
import { useUIStore } from '../stores/uiStore';

beforeEach(() => { window.history.replaceState(null, '', '/'); useUIStore.getState().reset(); });
async function open() {
  const repository = new MockRepository();
  render(<App repository={repository} />);
  await screen.findByRole('button', { name: '查看第 1 张 某个上午' });
  return { repository, user: userEvent.setup() };
}
describe('工作台交互', () => {
  it('K/X 和方向键有效，输入备注时不触发全局快捷键', async () => {
    const { repository, user } = await open();
    await user.keyboard('k');
    await waitFor(async () => expect((await repository.get('sample-project')).sequence).toEqual(['sample-1']));
    await user.keyboard('{ArrowRight}x');
    await waitFor(async () => expect((await repository.get('sample-project')).photos[1].status).toBe('reject'));
    await user.click(screen.getByRole('textbox', { name: '为什么保留 / 淘汰这张照片？' }));
    await user.keyboard('kx');
    expect((await repository.get('sample-project')).photos[1].status).toBe('reject');
  });
  it('比较支持不确定，关闭后状态保持原样', async () => {
    const { repository, user } = await open();
    await user.click(screen.getByRole('button', { name: '两图对比' }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '不确定' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect((await repository.get('sample-project')).sequence).toEqual([]);
    expect((await repository.get('sample-project')).events[0].type).toBe('compare');
  });
  it('浏览器 Mock 入口不启动桌面任务，保留集合不变', async () => {
    const { repository, user } = await open();
    await user.click(screen.getByRole('button', { name: 'Agent · 本地 Mock' }));
    expect(screen.getByText('验证受控工具闭环。AI 不直接修改正式选片；云端图像仅在明确授权后上传。')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '开始本地 Mock 任务' })).toBeDisabled();
    expect((await repository.get('sample-project')).sequence).toEqual([]);
  });
  it('已保留筛选无照片时，显示恢复路径', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: '已保留0' }));
    expect(screen.getByRole('heading', { name: '当前筛选没有照片' })).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '查看全部' }));
    expect(screen.getAllByTestId(/^photo-/)).toHaveLength(28);
  });
  it('仓库异常明确显示加载失败，并提供重试', async () => {
    class FailingRepository extends MockRepository { override async list(): Promise<never> { throw new Error('读取失败'); } }
    render(<App repository={new FailingRepository()} />);
    expect(await screen.findByRole('heading', { name: '项目加载失败' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重试' })).toBeInTheDocument();
  });
});
