import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from '../App';
import { MockRepository } from '../services/MockRepository';
import { useUIStore } from '../stores/uiStore';
import { I18nProvider } from '../i18n';
import { setCopyLocale } from '../app/copy';
import type { CloudAuthRepository } from '../domain/cloud-auth';
const signedIn: CloudAuthRepository = {
  status: async () => ({ authenticated: true, waiting: false, origin: 'https://team.focroom.cc', user: { id: 'fixture-user', name: '摄影师', email: 'author@fixture.test' }, error: null }),
  open: vi.fn(), cancel: vi.fn(), logout: vi.fn(), updateProfile: vi.fn(), subscribe: async () => () => {},
};

beforeEach(() => { window.history.replaceState(null, '', '/'); useUIStore.getState().reset(); useUIStore.setState({ aiDrafts: {} }); setCopyLocale('zh'); });
async function open() {
  const repository = new MockRepository();
  render(<App repository={repository} auth={signedIn} />);
  await screen.findByRole('button', { name: '查看第 1 张 某个上午' });
  return { repository, user: userEvent.setup() };
}
describe('工作台交互', () => {
  it('切换语言后，项目导航和 AI 标题立即同步更新', async () => {
    const originalStorage = globalThis.localStorage;
    const storage = new Map([['focalroom-locale', 'zh']]);
    vi.stubGlobal('localStorage', { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) });
    render(<I18nProvider><App repository={new MockRepository()} auth={signedIn} /></I18nProvider>);
    await screen.findByRole('button', { name: '查看第 1 张 某个上午' });
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: '账户菜单' }));
    await user.click(screen.getByRole('menuitem', { name: '设置' }));
    for (const [locale, name] of [['en', 'AI Selection'], ['ja', 'AI 写真セレクト'], ['ko', 'AI 사진 선택'], ['zh', 'AI 智能选片']]) {
      await user.selectOptions(screen.getByRole('combobox', { name: 'Language / 语言 / 言語 / 언어' }), locale);
      expect(within(screen.getByRole('main', { hidden: true })).getByRole('button', { name, hidden: true })).toBeInTheDocument();
    }
    vi.stubGlobal('localStorage', originalStorage);
  });
  it('K/X 和方向键有效，输入备注时不触发全局快捷键', async () => {
    const { repository, user } = await open();
    await user.keyboard('k');
    await waitFor(async () => expect((await repository.get('sample-project')).sequence).toEqual(['sample-1']));
    await user.keyboard('{ArrowRight}x');
    await waitFor(async () => expect((await repository.get('sample-project')).photos[1].status).toBe('reject'));
    await user.click(screen.getByRole('textbox', { name: '照片备注' }));
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
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'AI 智能选片' }));
    expect(screen.getByText('请在桌面版使用 AI 智能选片。')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '发送' })).toBeDisabled();
    expect((await repository.get('sample-project')).sequence).toEqual([]);
  });
  it('项目列表不显示上一个项目的功能，打开项目后 AI 仅从工作台呼出', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: '项目列表' }));
    await screen.findByRole('heading', { name: '项目' });
    expect(screen.queryByRole('button', { name: '选片工作台' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'AI 智能选片' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: /未命名的日常.*28 张照片/ }));
    const nav = await screen.findByRole('navigation', { name: '未命名的日常' });
    expect(within(nav).queryByRole('button', { name: 'AI 智能选片' })).not.toBeInTheDocument();
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'AI 智能选片' }));
    expect(screen.getByRole('dialog', { name: 'AI 智能选片' })).toBeInTheDocument();
    expect(screen.queryByLabelText('检查器面板')).not.toBeInTheDocument();
    await user.clear(screen.getByLabelText('选片消息'));
    await user.type(screen.getByLabelText('选片消息'), '为夏日旅行选择照片');
    await user.click(screen.getByRole('button', { name: '查看第 2 张 交错的视线' }));
    expect(screen.getByLabelText('选片消息')).toHaveValue('为夏日旅行选择照片');
    await user.click(screen.getByRole('button', { name: '关闭 AI 智能选片' }));
    expect(screen.getByRole('complementary', { name: '照片详情' })).toBeInTheDocument();
    expect(within(nav).queryByRole('button', { name: 'AI 智能选片' })).not.toBeInTheDocument();
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'AI 智能选片' }));
    expect(screen.getByLabelText('选片消息')).toHaveValue('为夏日旅行选择照片');
  });
  it('AI 目标按项目隔离，切换项目不会带入上一个项目的输入', async () => {
    const repository = new MockRepository();
    await repository.create('另一组作品', '');
    render(<App repository={repository} auth={signedIn} />);
    const user = userEvent.setup();
    await screen.findByRole('button', { name: '查看第 1 张 某个上午' });
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'AI 智能选片' }));
    await user.clear(screen.getByLabelText('选片消息'));
    await user.type(screen.getByLabelText('选片消息'), '仅属于第一个项目');
    await user.click(screen.getByRole('button', { name: '项目列表' }));
    await user.click(await screen.findByRole('button', { name: /另一组作品.*0 张照片/ }));
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'AI 智能选片' }));
    expect(screen.getByLabelText('选片消息')).toHaveValue('');
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
    render(<App repository={new FailingRepository()} auth={signedIn} />);
    expect(await screen.findByRole('heading', { name: '项目加载失败' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '重试' })).toBeInTheDocument();
  });
});
