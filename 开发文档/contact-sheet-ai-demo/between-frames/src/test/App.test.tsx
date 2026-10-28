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
  it('画布、共享评片和出版物使用独立工作区，隐藏照片筛选与检查器且不响应选片快捷键', async () => {
    const { repository, user } = await open();
    const before = await repository.get('sample-project');
    expect(screen.getByRole('textbox', { name: '照片备注' })).toBeVisible();
    for (const name of ['叙事画布', '共享评片', '出版物']) {
      await user.click(screen.getByRole('button', { name }));
      expect(screen.queryByLabelText('照片筛选')).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: '两图对比' })).not.toBeInTheDocument();
      expect(screen.queryByRole('textbox', { name: '照片备注' })).not.toBeInTheDocument();
      expect(document.querySelector('.selection-dock')).not.toBeInTheDocument();
      await user.keyboard('kx5');
    }
    expect(await repository.get('sample-project')).toEqual(before);
    await user.click(screen.getByRole('button', { name: '选片工作台' }));
    expect(screen.getByLabelText('照片筛选')).toBeVisible();
    expect(screen.getByRole('textbox', { name: '照片备注' })).toBeVisible();
  });
  it('切换语言后，项目导航和 AI 标题立即同步更新', async () => {
    const originalStorage = globalThis.localStorage;
    const storage = new Map([['focalroom-locale', 'zh']]);
    vi.stubGlobal('localStorage', { getItem: (key: string) => storage.get(key) ?? null, setItem: (key: string, value: string) => storage.set(key, value), removeItem: (key: string) => storage.delete(key) });
    render(<I18nProvider><App repository={new MockRepository()} auth={signedIn} /></I18nProvider>);
    await screen.findByRole('button', { name: '查看第 1 张 某个上午' });
    const user = userEvent.setup();
    await user.click(screen.getByRole('button', { name: '账户菜单' }));
    await user.click(screen.getByRole('menuitem', { name: '设置' }));
    for (const [locale, name] of [['en', 'Focal AI'], ['ja', 'Focal AI'], ['ko', 'Focal AI'], ['zh', 'Focal AI']]) {
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
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'Focal AI' }));
    expect(screen.getByText('请在桌面版使用 Focal AI。')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '发送' })).toBeDisabled();
    expect((await repository.get('sample-project')).sequence).toEqual([]);
  });
  it('项目列表不显示上一个项目的功能，打开项目后 AI 仅从工作台呼出', async () => {
    const { user } = await open();
    await user.click(screen.getByRole('button', { name: '项目列表' }));
    await screen.findByRole('heading', { name: '项目' });
    expect(screen.queryByRole('button', { name: '选片工作台' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Focal AI' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: '未命名的日常' }));
    const nav = await screen.findByRole('navigation', { name: '未命名的日常' });
    expect(within(nav).queryByRole('button', { name: 'Focal AI' })).not.toBeInTheDocument();
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'Focal AI' }));
    expect(screen.getByRole('dialog', { name: 'Focal AI' })).toBeInTheDocument();
    expect(screen.queryByLabelText('检查器面板')).not.toBeInTheDocument();
    await user.clear(screen.getByLabelText('消息'));
    await user.type(screen.getByLabelText('消息'), '为夏日旅行选择照片');
    await user.click(screen.getByRole('button', { name: '查看第 2 张 交错的视线' }));
    expect(screen.getByLabelText('消息')).toHaveValue('为夏日旅行选择照片');
    await user.click(screen.getByRole('button', { name: '关闭 Focal AI' }));
    expect(screen.getByRole('complementary', { name: '照片详情' })).toBeInTheDocument();
    expect(within(nav).queryByRole('button', { name: 'Focal AI' })).not.toBeInTheDocument();
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'Focal AI' }));
    expect(screen.getByLabelText('消息')).toHaveValue('为夏日旅行选择照片');
  });
  it('AI 目标按项目隔离，切换项目不会带入上一个项目的输入', async () => {
    const repository = new MockRepository();
    await repository.create('另一组作品', '');
    render(<App repository={repository} auth={signedIn} />);
    const user = userEvent.setup();
    await screen.findByRole('button', { name: '查看第 1 张 某个上午' });
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'Focal AI' }));
    await user.clear(screen.getByLabelText('消息'));
    await user.type(screen.getByLabelText('消息'), '仅属于第一个项目');
    await user.click(screen.getByRole('button', { name: '项目列表' }));
    await user.click(await screen.findByRole('button', { name: '另一组作品' }));
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'Focal AI' }));
    expect(screen.getByLabelText('消息')).toHaveValue('');
  });
  it('审美记忆仅从 Focal AI 抽屉进入，返回时保留对话、选区和工作台位置', async () => {
    const { user } = await open();
    const nav = screen.getByRole('navigation', { name: '未命名的日常' });
    expect(within(nav).queryByRole('button', { name: '审美记忆' })).not.toBeInTheDocument();
    await user.click(within(screen.getByRole('main')).getByRole('button', { name: 'Focal AI' }));
    const drawer = screen.getByRole('dialog', { name: 'Focal AI' });
    await user.clear(within(drawer).getByLabelText('消息'));
    await user.type(within(drawer).getByLabelText('消息'), '继续讨论照片的叙事');
    const selection = useUIStore.getState().selectedIds;
    await user.click(within(drawer).getByRole('button', { name: '审美记忆' }));
    expect(within(drawer).getByRole('region', { name: '审美记忆' })).toBeInTheDocument();
    expect(within(drawer).queryByRole('textbox', { name: '消息' })).not.toBeInTheDocument();
    expect(within(nav).getByRole('button', { name: '选片工作台' })).toHaveAttribute('aria-current', 'page');
    expect(useUIStore.getState().selectedIds).toEqual(selection);
    await user.click(within(drawer).getByRole('button', { name: '返回对话' }));
    expect(within(drawer).getByLabelText('消息')).toHaveValue('继续讨论照片的叙事');
    await user.click(within(drawer).getByRole('button', { name: '审美记忆' }));
    await user.keyboard('{Escape}');
    expect(screen.getByRole('dialog', { name: 'Focal AI' })).toBeInTheDocument();
    expect(within(drawer).getByLabelText('消息')).toHaveValue('继续讨论照片的叙事');
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog', { name: 'Focal AI' })).not.toBeInTheDocument();
  });
  it('旧记忆入口转到 Focal AI，不再切换照片检查器', () => {
    useUIStore.getState().setPanel('memory');
    expect(useUIStore.getState()).toMatchObject({ panel: 'photo', aiOpen: true, aiMemoryOpen: true });
    useUIStore.getState().reset();
    expect(useUIStore.getState()).toMatchObject({ aiOpen: false, aiMemoryOpen: false });
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
