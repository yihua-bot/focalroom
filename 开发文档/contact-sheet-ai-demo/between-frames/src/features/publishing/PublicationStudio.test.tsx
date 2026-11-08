import { cleanup, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { PublicationStudio } from './PublicationStudio';
import type { PublicationDocument, PublicationRepository, PublicationSpecUpdate, PublicationVersion } from '../../domain/publication';

const source = { assetId: 'a1', sourceSha256: 'a'.repeat(64), width: 3000, height: 4000, sourceProfileDigest: null, renditionId: null, variantId: null, renditionSha256: null, renditionWidth: null, renditionHeight: null, bitDepth: null, iccDigest: null, previewSha256: null };
const draft = (over: Partial<PublicationDocument> = {}): PublicationDocument => ({
  revision: 4,
  layout: {
    schemaVersion: 1,
    publication: { id: 'book', projectId: 'project', title: '初稿', pubType: 'photobook', specJson: '{"width_mm":210,"height_mm":297}', state: 'draft' },
    pages: [{ id: 'p1', pageNumber: 0, side: 'cover', widthMm: 210, heightMm: 297, marginMm: 15, bleedMm: 3, layoutJson: null, texts: [], frames: [{ id: 'f1', xMm: 15, yMm: 15, widthMm: 180, heightMm: 267, fitMode: 'fit', assetId: 'a1', renditionId: null, cropX: 0.5, cropY: 0.5, cropZoom: 1, source }] }],
  },
  previews: [{ frameId: 'f1' }],
  ...over,
});
const saved: PublicationVersion = { version: { id: 'v9', publicationId: 'book', revision: 3, label: '送印稿', kind: 'saved', snapshotSha256: 'b'.repeat(64), createdAt: '2026-10-21T00:00:00Z' }, layout: draft().layout, previews: [] };
const captionRow = { id: 't1', xMm: 15, yMm: 200, widthMm: 180, heightMm: 18, contentJson: JSON.stringify({ text: '十胜，雪原尽头' }), styleJson: '{"sizePt":12,"align":"center"}' };
const withCaption = (): PublicationDocument => {
  const base = draft();
  return { ...base, layout: { ...base.layout, pages: base.layout.pages.map(page => ({ ...page, texts: [captionRow] })) } };
};
// 一本开本不一致的书：封面 A4、第 1 页被手工改成 A5、第 2 页还是 A4。
// 每页的内容都按自己的尺寸摆，所以"放不下"只会是目标开本太小造成的，不是夹具本身越界。
type BookPage = PublicationDocument['layout']['pages'][number];
const sizeOf = (id: string, pageNumber: number, widthMm: number, heightMm: number): BookPage => ({
  id, pageNumber, side: pageNumber === 0 ? 'cover' : pageNumber % 2 ? 'right' : 'left',
  widthMm, heightMm, marginMm: 15, bleedMm: 3, layoutJson: null, texts: [],
  frames: [{ id: `frame-${id}`, xMm: 15, yMm: 15, widthMm: widthMm - 30, heightMm: heightMm - 30,
    fitMode: 'fit', assetId: `asset-${id}`, renditionId: null, cropX: 0.5, cropY: 0.5, cropZoom: 1, source }],
});
const withBook = (pages: BookPage[]): PublicationDocument => {
  const base = draft();
  return { ...base, layout: { ...base.layout, pages }, previews: pages.flatMap(page => page.frames.map(frame => ({ frameId: frame.id }))) };
};
const mixedBook = () => withBook([sizeOf('p1', 0, 210, 297), sizeOf('p2', 1, 148, 210), sizeOf('p3', 2, 210, 297)]);
const sameBook = () => withBook([sizeOf('p1', 0, 210, 297), sizeOf('p2', 1, 210, 297), sizeOf('p3', 2, 210, 297)]);
// 一面展墙：观看距离与悬挂中心线记在 spec_json 里，页幅那两个数照旧留着。
const withSpec = (specJson: string, pubType: 'exhibition' | 'photobook' = 'exhibition'): PublicationDocument => {
  const base = draft();
  return { ...base, layout: { ...base.layout, publication: { ...base.layout.publication, pubType, specJson } } };
};
const wall = () => withSpec('{"width_mm":210,"height_mm":297,"viewing_distance_mm":2500,"hang_center_mm":1450}');
const distance = '观看距离（毫米）';
const hang = '悬挂中心线（毫米）';
const saveSpec = '保存展览规格';
const apply = '应用到全部页面';
const report = (over: Record<string, unknown> = {}) => ({
  minPpi: 300, pages: 1, frames: 1, texts: 0, blocking: 2, warnings: 0, infos: 0, truncated: false,
  issues: [
    { code: 'low_resolution', severity: 'blocking', pageNumber: 0, frameId: 'f1', textId: null, measured: 169, required: 300 },
    { code: 'original_offline', severity: 'blocking', pageNumber: null, frameId: null, textId: null, measured: null, required: null },
  ],
  ...over,
});
const repository = (over: Partial<PublicationRepository> = {}): PublicationRepository => ({
  list: vi.fn().mockResolvedValue([]), create: vi.fn(), generate: vi.fn(), pages: vi.fn().mockResolvedValue([]),
  document: vi.fn().mockResolvedValue(draft()), versions: vi.fn().mockResolvedValue([{ ...saved.version }]), version: vi.fn().mockResolvedValue(saved),
  editFrame: vi.fn().mockResolvedValue(undefined), saveVersion: vi.fn().mockResolvedValue(undefined), restoreVersion: vi.fn().mockResolvedValue(undefined),
  addText: vi.fn().mockResolvedValue(undefined), editText: vi.fn().mockResolvedValue(undefined), removeText: vi.fn().mockResolvedValue(undefined),
  editPage: vi.fn().mockResolvedValue(undefined), editPages: vi.fn().mockResolvedValue(undefined),
  changeState: vi.fn().mockResolvedValue(undefined),
  exportWorklist: vi.fn().mockResolvedValue({ path: '/Users/me/清单/worklist.csv', sha256: 'c'.repeat(64), bytes: 2048 }),
  preflight: vi.fn().mockResolvedValue(report()), ...over,
} as PublicationRepository);
const props = (repo: PublicationRepository) => ({ repository: repo, projectId: 'project', publication: { id: 'book', projectId: 'project', pubType: 'photobook', title: '摄影书', state: 'draft', createdAt: '2026-10-21', revision: 4 }, readOnly: false, onClose: vi.fn(), onChanged: vi.fn().mockResolvedValue(undefined) });

afterEach(cleanup);

it('检查整本把页码、实测与要求一起摆上屏幕', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '检查整本' }));
  await screen.findByText('2 项必须先处理，现在还不能送印。');
  const rows = screen.getAllByRole('listitem');
  expect(rows).toHaveLength(2);
  expect(within(rows[0]).getByText('封面')).toBeInTheDocument();
  expect(within(rows[0]).getByText('按现在的版面尺寸印出来分辨率不足')).toBeInTheDocument();
  expect(within(rows[0]).getByText('实测 169 PPI，要求 300 PPI')).toBeInTheDocument();
  expect(within(rows[1]).getByText('整本')).toBeInTheDocument();
  expect(within(rows[1]).getByText('原图文件已失联，无法印出')).toBeInTheDocument();
  expect(repo.preflight).toHaveBeenCalledWith('project', 'book', 300);
  // 只读通道：不产生任何写命令。
  expect(repo.editFrame).not.toHaveBeenCalled();
  expect(repo.saveVersion).not.toHaveBeenCalled();
});

it('换门槛会让人重新查一遍，旧结果不留在屏幕上冒充新结论', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '检查整本' }));
  await screen.findByRole('button', { name: '重新检查' });
  await user.selectOptions(screen.getByRole('combobox', { name: '最低分辨率' }), '150');
  expect(screen.queryByText('2 项必须先处理，现在还不能送印。')).toBeNull();
  await user.click(screen.getByRole('button', { name: '检查整本' }));
  await screen.findByText('实测 169 PPI，要求 300 PPI');
  expect(repo.preflight).toHaveBeenLastCalledWith('project', 'book', 150);
});

it('检查失败说清没查成，不给出"可以送印"的假结论', async () => {
  const user = userEvent.setup();
  const repo = repository({ preflight: vi.fn().mockRejectedValue(new Error('印前检查没能完成，请稍后重试。')) });
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '检查整本' }));
  expect(await screen.findByRole('status')).toHaveTextContent('印前检查没能完成，请稍后重试。');
  expect(screen.queryByText('没有发现问题，这份书稿可以送印。')).toBeNull();
  expect(screen.queryByRole('listitem')).toBeNull();
});

it('翻回历史版本时不显示当前版面的检查结果', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '检查整本' }));
  await screen.findAllByRole('listitem');
  await user.click(screen.getByRole('button', { name: /送印稿/ }));
  expect(screen.queryByText('印前检查')).toBeNull();
  expect(screen.queryByRole('listitem')).toBeNull();
});

it('新增图注给一组落在安全区里的默认位置，写进去就能在页面上看到', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  // 版面加载完成后按钮才可点。
  await screen.findByText('这一页还没有图注');
  await user.click(screen.getByRole('button', { name: '新增图注' }));
  expect(screen.queryByText('这一页还没有图注')).toBeNull();
  expect(screen.getByRole('spinbutton', { name: '左侧位置（mm）' })).toHaveValue(15);
  expect(screen.getByRole('spinbutton', { name: '顶部位置（mm）' })).toHaveValue(264);
  expect(screen.getByRole('spinbutton', { name: '宽度（mm）' })).toHaveValue(180);
  expect(screen.getByRole('spinbutton', { name: '高度（mm）' })).toHaveValue(18);
  expect(screen.getByRole('spinbutton', { name: '字号（磅）' })).toHaveValue(10);
  expect(screen.getByText('文字、字号或位置还不对，先修正再保存')).toBeInTheDocument();
  const save = screen.getByRole('button', { name: '保存调整' });
  expect(save).toBeDisabled();
  await user.type(screen.getByRole('textbox', { name: '文字内容' }), '冬，北海道');
  expect(within(screen.getByRole('figure')).getByText('冬，北海道')).toBeInTheDocument();
  expect(save).toBeEnabled();
  await user.click(save);
  expect(await screen.findByText('图注已保存')).toBeInTheDocument();
  expect(repo.addText).toHaveBeenCalledWith('project', 'book', 4, {
    textId: expect.any(String), pageId: 'p1', text: '冬，北海道',
    xMm: 15, yMm: 264, widthMm: 180, heightMm: 18, sizePt: 10, align: 'left',
  });
  expect(repo.editText).not.toHaveBeenCalled();
});

it('点页面里的文字框改内容、字号和对齐，改动落在同一条图注上而不是新增一条', async () => {
  const user = userEvent.setup();
  const repo = repository({ document: vi.fn().mockResolvedValue(withCaption()) });
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '第 1 段文字' }));
  const box = screen.getByRole('textbox', { name: '文字内容' });
  expect(box).toHaveValue('十胜，雪原尽头');
  expect(screen.getByRole('combobox', { name: '对齐' })).toHaveValue('center');
  await user.clear(box);
  await user.type(box, '十胜，雪原尽头 · 2026');
  await user.selectOptions(screen.getByRole('combobox', { name: '对齐' }), 'left');
  await user.click(screen.getByRole('button', { name: '保存调整' }));
  expect(await screen.findByText('图注已保存')).toBeInTheDocument();
  expect(repo.editText).toHaveBeenCalledWith('project', 'book', 4, {
    textId: 't1', text: '十胜，雪原尽头 · 2026', xMm: 15, yMm: 200, widthMm: 180, heightMm: 18, sizePt: 12, align: 'left',
  });
  expect(repo.addText).not.toHaveBeenCalled();
});

it('删图注先问一遍，确认之后才真的删，并说明能在版本历史找回', async () => {
  const user = userEvent.setup();
  const repo = repository({ document: vi.fn().mockResolvedValue(withCaption()) });
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '第 1 段文字' }));
  await user.click(screen.getByRole('button', { name: '删除图注' }));
  expect(repo.removeText).not.toHaveBeenCalled();
  const group = await screen.findByRole('group', { name: '确认删除这段文字？删除后可以在版本历史里找回。' });
  expect(within(group).getByText('确认删除这段文字？删除后可以在版本历史里找回。')).toBeInTheDocument();
  await user.click(within(group).getByRole('button', { name: '删除图注' }));
  expect(await screen.findByText('图注已删除')).toBeInTheDocument();
  expect(repo.removeText).toHaveBeenCalledWith('project', 'book', 4, 't1');
});

it('后端拒绝时把原因原样说清，草稿留在屏幕上，不冒充已保存', async () => {
  const user = userEvent.setup();
  const repo = repository({
    document: vi.fn().mockResolvedValue(withCaption()),
    editText: vi.fn().mockRejectedValue(new Error('图注超出了页面范围，请调整位置或大小。')),
  });
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '第 1 段文字' }));
  const box = screen.getByRole('textbox', { name: '文字内容' });
  await user.clear(box);
  await user.type(box, '美瑛，积雪的农道');
  await user.click(screen.getByRole('button', { name: '保存调整' }));
  expect(await screen.findByRole('status')).toHaveTextContent('图注超出了页面范围，请调整位置或大小。');
  expect(screen.queryByText('图注已保存')).toBeNull();
  expect(box).toHaveValue('美瑛，积雪的农道');
});

it('翻回历史版本时图注只读：不给新增入口，也点不动页面里的文字', async () => {
  const user = userEvent.setup();
  const repo = repository({
    document: vi.fn().mockResolvedValue(withCaption()),
    version: vi.fn().mockResolvedValue({ ...saved, layout: withCaption().layout }),
  });
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '第 1 段文字' }));
  expect(screen.getByRole('button', { name: '新增图注' })).toBeEnabled();
  await user.click(screen.getByRole('button', { name: /送印稿/ }));
  expect(screen.queryByRole('button', { name: '新增图注' })).toBeNull();
  expect(screen.getByRole('button', { name: '第 1 段文字' })).toBeDisabled();
  expect(repo.editText).not.toHaveBeenCalled();
});

// 页面按百分比定位，所以"这张图在新尺寸下还放不放得下"就是内联宽度有没有超过 100%。
const share = (element: HTMLElement, property: 'width' | 'height') => Number.parseFloat(element.style.getPropertyValue(property));

it('换开本立刻按新尺寸重画页面，图框放不下是当场看得见的', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  const preset = await screen.findByRole('combobox', { name: '开本' });
  const width = screen.getByRole('spinbutton', { name: '页面宽度（mm）' });
  expect(preset).toHaveValue('a4');
  expect(width).toHaveValue(210);
  expect(screen.getByRole('spinbutton', { name: '页面高度（mm）' })).toHaveValue(297);
  expect(screen.getByRole('spinbutton', { name: '安全边距（mm）' })).toHaveValue(15);
  expect(screen.getByRole('spinbutton', { name: '出血（mm）' })).toHaveValue(3);
  expect(screen.getByRole('button', { name: '保存纸张' })).toBeDisabled();
  const frame = screen.getByRole('button', { name: '选择照片框 1' });
  expect(share(frame, 'width')).toBeCloseTo((180 / 210) * 100);
  await user.selectOptions(preset, 'a5');
  expect(width).toHaveValue(148);
  // 开本预设只换尺寸：边距和出血是作者自己定的数，不许被顺手改掉。
  expect(screen.getByRole('spinbutton', { name: '安全边距（mm）' })).toHaveValue(15);
  expect(screen.getByRole('spinbutton', { name: '出血（mm）' })).toHaveValue(3);
  // 180mm 宽的图框放进 148mm 的页面：预览直接超出边界，不用等保存被拒才知道。
  expect(share(frame, 'width')).toBeGreaterThan(100);
  expect(screen.getByRole('button', { name: '保存纸张' })).toBeEnabled();
  await user.click(screen.getByRole('button', { name: '保存纸张' }));
  expect(await screen.findByText('纸张尺寸已保存')).toBeInTheDocument();
  expect(repo.editPage).toHaveBeenCalledWith('project', 'book', 4, { pageId: 'p1', widthMm: 148, heightMm: 210, marginMm: 15, bleedMm: 3 });
});

it('无效的纸张值一条命令都不发：越界量程和吃掉整页的边距只让保存按钮灰着', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  const height = await screen.findByRole('spinbutton', { name: '页面高度（mm）' });
  const bleed = screen.getByRole('spinbutton', { name: '出血（mm）' });
  const margin = screen.getByRole('spinbutton', { name: '安全边距（mm）' });
  const save = screen.getByRole('button', { name: '保存纸张' });
  const discard = screen.getByRole('button', { name: '放弃纸张改动' });
  const invalid = '纸张尺寸、边距或出血还不对，先修正再保存';
  await user.clear(height);
  await user.type(height, '30');
  expect(await screen.findByText(invalid)).toBeInTheDocument();
  expect(save).toBeDisabled();
  // 没落地的纸张改动开着时不给新增图注，免得一次保存顶掉另一份草稿。
  expect(screen.getByRole('button', { name: '新增图注' })).toBeDisabled();
  await user.clear(height);
  await user.type(height, '297');
  await user.clear(bleed);
  await user.type(bleed, '26');
  expect(screen.getByText(invalid)).toBeInTheDocument();
  expect(save).toBeDisabled();
  await user.clear(bleed);
  await user.type(bleed, '3');
  // 边距 150mm 在契约里是合法数字，但两倍的边距已经吃掉 210mm 的短边——本地就拦下，不占后端一次往返。
  await user.clear(margin);
  await user.type(margin, '150');
  expect(screen.getByText(invalid)).toBeInTheDocument();
  expect(save).toBeDisabled();
  await user.click(discard);
  expect(screen.queryByText(invalid)).toBeNull();
  expect(margin).toHaveValue(15);
  expect(repo.editPage).not.toHaveBeenCalled();
});

it('后端拒掉纸张改动时说清原因，草稿留在屏幕上不冒充已保存', async () => {
  const user = userEvent.setup();
  const repo = repository({ editPage: vi.fn().mockRejectedValue(new Error('页面尺寸、边距或页码无效')) });
  render(<PublicationStudio {...props(repo)} />);
  await user.selectOptions(await screen.findByRole('combobox', { name: '开本' }), 'a5');
  await user.click(screen.getByRole('button', { name: '保存纸张' }));
  expect(await screen.findByRole('status')).toHaveTextContent('页面尺寸、边距或页码无效');
  expect(screen.queryByText('纸张尺寸已保存')).toBeNull();
  expect(screen.getByRole('spinbutton', { name: '页面宽度（mm）' })).toHaveValue(148);
});

it('翻回历史版本时纸张只读：表单不出现，也不发命令', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  await screen.findByRole('combobox', { name: '开本' });
  await user.click(screen.getByRole('button', { name: /送印稿/ }));
  expect(screen.queryByText('纸张与边距')).toBeNull();
  expect(screen.queryByRole('combobox', { name: '开本' })).toBeNull();
  expect(repo.editPage).not.toHaveBeenCalled();
});

it('图注草稿没落地时纸张输入是锁着的，一次只改一份草稿', async () => {
  const user = userEvent.setup();
  const repo = repository({ document: vi.fn().mockResolvedValue(mixedBook()) });
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: '新增图注' }));
  expect(screen.getByRole('spinbutton', { name: '页面宽度（mm）' })).toBeDisabled();
  expect(screen.getByRole('combobox', { name: '开本' })).toBeDisabled();
  expect(screen.getByRole('button', { name: '保存纸张' })).toBeDisabled();
  // 同一条规矩也管住整本统一：草稿没落地时批量改纸张会顶掉那份草稿。
  expect(screen.getByRole('button', { name: apply })).toBeDisabled();
  expect(repo.editPage).not.toHaveBeenCalled();
  expect(repo.editPages).not.toHaveBeenCalled();
});

it('整本一次统一开本：一次点完，每页都发出当前页面的四个毫米数', async () => {
  const user = userEvent.setup();
  const repo = repository({ document: vi.fn().mockResolvedValue(mixedBook()), editPages: vi.fn().mockResolvedValue(undefined) });
  render(<PublicationStudio {...props(repo)} />);
  const button = await screen.findByRole('button', { name: apply });
  // 中间那页还是 A5，整本不一致，所以按钮可用；参照数是当前选中的封面。
  expect(button).toBeEnabled();
  await user.click(button);
  expect(await screen.findByText('整本纸张已统一')).toBeInTheDocument();
  expect(repo.editPages).toHaveBeenCalledWith('project', 'book', 4, [
    { pageId: 'p1', widthMm: 210, heightMm: 297, marginMm: 15, bleedMm: 3 },
    { pageId: 'p2', widthMm: 210, heightMm: 297, marginMm: 15, bleedMm: 3 },
    { pageId: 'p3', widthMm: 210, heightMm: 297, marginMm: 15, bleedMm: 3 },
  ]);
  // 批量走一条命令，不该再补一次逐页改动。
  expect(repo.editPage).not.toHaveBeenCalled();
});

it('整本按草稿里的新尺寸统一，放不下的那几页要当场点名', async () => {
  const user = userEvent.setup();
  const repo = repository({ document: vi.fn().mockResolvedValue(mixedBook()), editPages: vi.fn().mockResolvedValue(undefined) });
  render(<PublicationStudio {...props(repo)} />);
  const button = await screen.findByRole('button', { name: apply });
  const blocked = '封面、第 2 页在新尺寸下放不下，先调整这些页上的照片或图注';
  expect(screen.queryByText(blocked)).toBeNull();
  const preset = screen.getByRole('combobox', { name: '开本' });
  await user.selectOptions(preset, 'a5');
  // 后端只会回一句"内容超出页面及出血范围"，不说是哪一页，所以这里必须禁用并报名字。
  expect(button).toBeDisabled();
  expect(await screen.findByText(blocked)).toBeInTheDocument();
  expect(repo.editPages).not.toHaveBeenCalled();
  // 换回封面原本的 A4：A5 那页本来就窄，整本都放得下，按钮重新可用。
  await user.selectOptions(preset, 'a4');
  expect(button).toBeEnabled();
  expect(screen.queryByText(blocked)).toBeNull();
});

it('整本本来就一致时不给统一：不改任何东西，也不白记一个版本', async () => {
  const user = userEvent.setup();
  const repo = repository({ document: vi.fn().mockResolvedValue(sameBook()), editPages: vi.fn().mockResolvedValue(undefined) });
  render(<PublicationStudio {...props(repo)} />);
  const button = await screen.findByRole('button', { name: apply });
  expect(button).toBeDisabled();
  await user.click(button);
  expect(repo.editPages).not.toHaveBeenCalled();
});

it('只有一页的书不给统一入口', async () => {
  const repo = repository({ editPages: vi.fn().mockResolvedValue(undefined) });
  render(<PublicationStudio {...props(repo)} />);
  await screen.findByText('纸张与边距');
  expect(screen.queryByRole('button', { name: apply })).toBeNull();
});

it('没有批量写通道时这一区不出现：不给按下去什么都不会发生的按钮', async () => {
  const repo = repository({ document: vi.fn().mockResolvedValue(mixedBook()), editPages: undefined });
  render(<PublicationStudio {...props(repo)} />);
  await screen.findByText('纸张与边距');
  expect(screen.queryByRole('button', { name: apply })).toBeNull();
});

it('只读视图里的统一入口按不动，也不发批量命令', async () => {
  const user = userEvent.setup();
  const repo = repository({ document: vi.fn().mockResolvedValue(mixedBook()), editPages: vi.fn().mockResolvedValue(undefined) });
  render(<PublicationStudio {...{ ...props(repo), readOnly: true }} />);
  const button = await screen.findByRole('button', { name: apply });
  expect(button).toBeDisabled();
  await user.click(button);
  expect(repo.editPages).not.toHaveBeenCalled();
});

it('页数超了一条命令能带的操作数时提示逐页改，不发一条注定被契约拒掉的命令', async () => {
  const user = userEvent.setup();
  // 101 页，其中第 1 页开本不同（否则"已经一致"会先把按钮禁掉，测不到这条上限）。
  const repo = repository({
    document: vi.fn().mockResolvedValue(withBook(Array.from({ length: 101 }, (_unused, index) => sizeOf(`p${index}`, index, index === 1 ? 215 : 210, 297)))),
    editPages: vi.fn().mockResolvedValue(undefined),
  });
  render(<PublicationStudio {...props(repo)} />);
  const button = await screen.findByRole('button', { name: apply });
  expect(button).toBeDisabled();
  expect(await screen.findByText('整本的页数超了一条命令能带的操作数，请一页一页改')).toBeInTheDocument();
  await user.click(button);
  expect(repo.editPages).not.toHaveBeenCalled();
});

it('后端拒掉整本统一时说清原因，不冒充纸张已统一', async () => {
  const user = userEvent.setup();
  const repo = repository({
    document: vi.fn().mockResolvedValue(mixedBook()),
    editPages: vi.fn().mockRejectedValue(new Error('内容超出页面及出血范围，请调整位置和尺寸')),
  });
  render(<PublicationStudio {...props(repo)} />);
  await user.click(await screen.findByRole('button', { name: apply }));
  expect(await screen.findByRole('status')).toHaveTextContent('内容超出页面及出血范围，请调整位置和尺寸');
  expect(screen.queryByText('整本纸张已统一')).toBeNull();
});

// 审校状态区必须能在非草稿的本子上按下去：它一旦被 writable 挡住，定稿就变成回不来的死路。
const atState = (state: string): PublicationDocument => {
  const base = draft();
  return { ...base, layout: { ...base.layout, publication: { ...base.layout.publication, state } } };
};
const stateSection = async () => (await screen.findByRole('heading', { name: '审校状态' })).parentElement!;

it('草稿只给一个"送审"按钮，点它只发一条状态命令', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  const section = await stateSection();
  expect(within(section).getByText('草稿')).toBeInTheDocument();
  expect(within(section).getAllByRole('button').map(button => button.textContent)).toEqual(['送审']);
  await user.click(within(section).getByRole('button', { name: '送审' }));
  expect(repo.changeState).toHaveBeenCalledWith('project', 'book', 4, 'in_review');
  expect(await screen.findByRole('status')).toHaveTextContent('审校状态已更新');
  // 流转不是版面改动：不该顺手发出任何编辑命令。
  expect(repo.editFrame).not.toHaveBeenCalled();
  expect(repo.editPages).not.toHaveBeenCalled();
  expect(repo.saveVersion).not.toHaveBeenCalled();
});

it('审阅中的本子版面只读，但状态区还能把它退回草稿', async () => {
  const user = userEvent.setup();
  const repo = repository({ document: vi.fn().mockResolvedValue(atState('in_review')) });
  render(<PublicationStudio {...props(repo)} />);
  const section = await stateSection();
  expect(within(section).getByText('审阅中')).toBeInTheDocument();
  expect(within(section).getByText('这本已不在草稿状态，版面是只读的；逐级退回草稿后才能改动。')).toBeInTheDocument();
  expect(within(section).getAllByRole('button').map(button => button.textContent)).toEqual(['退回草稿', '定稿']);
  expect(screen.getByRole('button', { name: '保存纸张' })).toBeDisabled();
  await user.click(within(section).getByRole('button', { name: '退回草稿' }));
  expect(repo.changeState).toHaveBeenCalledWith('project', 'book', 4, 'draft');
});

it('已交付不再给任何按钮，只把话说完', async () => {
  const repo = repository({ document: vi.fn().mockResolvedValue(atState('delivered')) });
  render(<PublicationStudio {...props(repo)} />);
  const section = await stateSection();
  expect(within(section).getByText('已交付')).toBeInTheDocument();
  expect(within(section).queryAllByRole('button')).toEqual([]);
  expect(repo.changeState).not.toHaveBeenCalled();
});

it('定稿的本子只能往前后各退一步，不会凭空出现"退回草稿"', async () => {
  const repo = repository({ document: vi.fn().mockResolvedValue(atState('finalized')) });
  render(<PublicationStudio {...props(repo)} />);
  const section = await stateSection();
  expect(within(section).getAllByRole('button').map(button => button.textContent)).toEqual(['送审', '交付']);
  expect(within(section).queryByRole('button', { name: '退回草稿' })).toBeNull();
});

it('看历史版本、只读打开、以及没有状态通道时都不出现状态区', async () => {
  const user = userEvent.setup();
  const repo = repository();
  const { unmount } = render(<PublicationStudio {...props(repo)} />);
  await stateSection();
  await user.click(await screen.findByRole('button', { name: /送印稿/ }));
  await screen.findByRole('button', { name: '返回当前版面' });
  expect(screen.queryByRole('heading', { name: '审校状态' })).toBeNull();
  unmount();

  const locked = repository();
  const second = render(<PublicationStudio {...props(locked)} readOnly />);
  await screen.findByText('封面');
  expect(screen.queryByRole('heading', { name: '审校状态' })).toBeNull();
  second.unmount();

  const noChannel = repository({ changeState: undefined });
  render(<PublicationStudio {...props(noChannel)} />);
  await screen.findByText('封面');
  expect(screen.queryByRole('heading', { name: '审校状态' })).toBeNull();
});

it('有一条没确认的操作或手里有未保存改动时，状态按钮先按住', async () => {
  const user = userEvent.setup();
  const pending = repository({ pendingCommands: vi.fn().mockReturnValue([{ publicationId: 'book' }]) });
  const { unmount } = render(<PublicationStudio {...props(pending)} />);
  const section = await stateSection();
  expect(within(section).getByRole('button', { name: '送审' })).toBeDisabled();
  unmount();

  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  const margin = await screen.findByRole('spinbutton', { name: '安全边距（mm）' });
  await user.clear(margin);
  await user.type(margin, '12');
  expect(within(await stateSection()).getByRole('button', { name: '送审' })).toBeDisabled();
  expect(repo.changeState).not.toHaveBeenCalled();
});

it('恢复入口跟着书现在的状态走，不跟着列表里那份旧副本', async () => {
  const user = userEvent.setup();
  // 列表还写着草稿，库里已经送审：这时不该给一个点下去必被后端拒掉的恢复。
  const staleList = repository({ document: vi.fn().mockResolvedValue(atState('in_review')) });
  const first = render(<PublicationStudio {...props(staleList)} />);
  await user.click(await screen.findByRole('button', { name: /送印稿/ }));
  await screen.findByRole('button', { name: '返回当前版面' });
  expect(screen.queryByRole('button', { name: '恢复此版本' })).toBeNull();
  first.unmount();

  // 反过来：刚在这个窗口里把本子退回草稿，列表那份副本还没刷新，恢复要立刻可用。
  const repo = repository({ document: vi.fn().mockResolvedValueOnce(atState('in_review')).mockResolvedValue(draft()) });
  const listed = props(repo);
  render(<PublicationStudio {...listed} publication={{ ...listed.publication, state: 'in_review' }} />);
  await user.click(await screen.findByRole('button', { name: '退回草稿' }));
  expect(repo.changeState).toHaveBeenCalledWith('project', 'book', 4, 'draft');
  await user.click(await screen.findByRole('button', { name: /送印稿/ }));
  await user.click(await screen.findByRole('button', { name: '恢复此版本' }));
  expect(screen.getByRole('button', { name: '确认恢复' })).toBeEnabled();
});



const sheetPath = '/Users/me/清单/worklist.csv';
const sheetReceipt = { path: sheetPath, sha256: 'c'.repeat(64), bytes: 2048 };
const exportButtonName = '导出作品清单（CSV）';
const exportSection = async () => (await screen.findByRole('heading', { name: '作品清单导出' })).parentElement as HTMLElement;

it('导出作品清单只发一条只读命令，条数与落盘路径都上屏', async () => {
  const user = userEvent.setup();
  // 一页一图一注：清单的条数必须把图框和图注都算进去，否则照着这张表点货会少一件。
  const repo = repository({ document: vi.fn().mockResolvedValue(withCaption()), exportWorklist: vi.fn().mockResolvedValue(sheetReceipt) });
  render(<PublicationStudio {...props(repo)} />);
  const section = await exportSection();
  await user.click(within(section).getByRole('button', { name: exportButtonName }));
  expect(repo.exportWorklist).toHaveBeenCalledWith('project', 'book');
  expect(await within(section).findByRole('status')).toHaveTextContent(`已导出 2 条：${sheetPath}`);
  expect(within(section).getByText(`SHA256 ${'c'.repeat(64)}`)).toBeInTheDocument();
  // 导出不是编辑：不该顺手发出任何写命令，也不动状态。
  expect(repo.editFrame).not.toHaveBeenCalled();
  expect(repo.changeState).not.toHaveBeenCalled();
  expect(repo.saveVersion).not.toHaveBeenCalled();
});

it('保存对话框里取消说清没写文件，不报成失败也不留半个回执', async () => {
  const user = userEvent.setup();
  const repo = repository({ exportWorklist: vi.fn().mockResolvedValue(null) });
  render(<PublicationStudio {...props(repo)} />);
  const section = await exportSection();
  await user.click(within(section).getByRole('button', { name: exportButtonName }));
  expect(await screen.findByRole('status')).toHaveTextContent('已取消导出，没有写入文件。');
  expect(within(section).queryByText(/已导出/)).toBeNull();
});

it('导出失败与回执读不懂都只说没导成，不给半个路径', async () => {
  const user = userEvent.setup();
  for (const failure of [new Error('作品清单导出失败：磁盘已满'), new Error('mock 通道不可用')]) {
    const repo = repository({ exportWorklist: vi.fn().mockRejectedValue(failure) });
    render(<PublicationStudio {...props(repo)} />);
    const section = await exportSection();
    await user.click(within(section).getByRole('button', { name: exportButtonName }));
    const notice = await screen.findByRole('status');
    expect(notice.textContent).not.toContain(sheetPath);
    expect(notice.textContent?.length).toBeGreaterThan(0);
    cleanup();
  }
});

it('看历史版本或仓库没有这条通道时不出现导出区', async () => {
  const user = userEvent.setup();
  const repo = repository();
  render(<PublicationStudio {...props(repo)} />);
  const section = await exportSection();
  expect(within(section).getByText('把整本摊成一张表：每页图框的尺寸、实际送印的像素与有效 PPI，图注原样带上，可交给印厂或展陈逐条核对。')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: /送印稿/ }));
  expect(screen.queryByRole('heading', { name: '作品清单导出' })).toBeNull();
  cleanup();
  render(<PublicationStudio {...props(repository({ exportWorklist: undefined }))} />);
  await screen.findByRole('heading', { name: '审校状态' });
  expect(screen.queryByRole('heading', { name: '作品清单导出' })).toBeNull();
  expect(screen.queryByRole('button', { name: exportButtonName })).toBeNull();
});

it('只读视图也能出货：导出不走编辑那道闸门', async () => {
  const user = userEvent.setup();
  const repo = repository({ exportWorklist: vi.fn().mockResolvedValue(sheetReceipt) });
  render(<PublicationStudio {...{ ...props(repo), readOnly: true }} />);
  const section = await exportSection();
  await user.click(within(section).getByRole('button', { name: exportButtonName }));
  expect(repo.exportWorklist).toHaveBeenCalledTimes(1);
  expect(await within(section).findByRole('status')).toHaveTextContent('已导出 1 条');
  expect(repo.editFrame).not.toHaveBeenCalled();
});

it('导出后版面又动了，旧回执不留在屏幕上冒充当前这本', async () => {
  const user = userEvent.setup();
  const repo = repository({ exportWorklist: vi.fn().mockResolvedValue(sheetReceipt) });
  render(<PublicationStudio {...props(repo)} />);
  const section = await exportSection();
  await user.click(within(section).getByRole('button', { name: exportButtonName }));
  await within(section).findByText(/已导出 1 条/);
  await user.click(await screen.findByRole('button', { name: '送审' }));
  await screen.findByText('审校状态已更新');
  expect(screen.queryByText(/已导出 1 条/)).toBeNull();
});

it('连点两次也只导一次，不会开出两个保存对话框', async () => {
  const user = userEvent.setup();
  let release: (value: typeof sheetReceipt) => void = () => {};
  const repo = repository({ exportWorklist: vi.fn(() => new Promise<typeof sheetReceipt>(resolve => { release = resolve; })) });
  render(<PublicationStudio {...props(repo)} />);
  const section = await exportSection();
  const button = within(section).getByRole('button', { name: exportButtonName });
  await user.click(button);
  await user.click(button);
  expect(repo.exportWorklist).toHaveBeenCalledTimes(1);
  release(sheetReceipt);
  expect(await within(section).findByRole('status')).toHaveTextContent('已导出 1 条');
});

// 展览规格存的是"这面墙怎么看"，库里那份 spec_json 是唯一依据：测试用一个会随写入变化的文档夹具，
// 免得保存成功后屏幕上还停在旧数上，看不出改动到底有没有落地。
const specStore = (initial: PublicationDocument = wall()) => {
  const store = { doc: initial };
  const repo = repository({
    document: vi.fn(async () => store.doc),
    editSpec: vi.fn(async (_projectId: string, _publicationId: string, revision: number, spec: PublicationSpecUpdate) => {
      const json = {
        width_mm: 210,
        height_mm: 297,
        ...(spec.viewingDistanceMm === null ? {} : { viewing_distance_mm: spec.viewingDistanceMm }),
        ...(spec.hangCenterMm === null ? {} : { hang_center_mm: spec.hangCenterMm }),
      };
      store.doc = { ...store.doc, revision: revision + 1, layout: { ...store.doc.layout, publication: { ...store.doc.layout.publication, specJson: JSON.stringify(json) } } };
    }),
  });
  return { store, repo };
};
const specSection = async () => (await screen.findByRole('heading', { name: '展览规格' })).parentElement!;

it('展览规格区只给展览册：摄影书没有"看多远"这件事，没有写通道也不给入口', async () => {
  const repo = repository({ document: vi.fn().mockResolvedValue(wall()), editSpec: vi.fn().mockResolvedValue(undefined) });
  const { unmount } = render(<PublicationStudio {...props(repo)} />);
  const section = await specSection();
  expect(screen.getByRole('spinbutton', { name: distance })).toHaveValue(2500);
  expect(screen.getByRole('spinbutton', { name: hang })).toHaveValue(1450);
  expect(within(section).getByText('留空表示不记录。没有记录观看距离时，印前检查仍按印刷基线判定。')).toBeInTheDocument();
  unmount();

  // 摄影书：库里根本没有这两个数，给它一个填了也只会撞上后端那句"只用于展览出版物"。
  const book = repository({ editSpec: vi.fn().mockResolvedValue(undefined) });
  render(<PublicationStudio {...props(book)} />);
  await screen.findByText('纸张与边距');
  expect(screen.queryByRole('heading', { name: '展览规格' })).toBeNull();
});

it('没有规格写通道时这一区整个不出现：不给按下去什么都不会发生的入口', async () => {
  const repo = repository({ document: vi.fn().mockResolvedValue(wall()) });
  render(<PublicationStudio {...props(repo)} />);
  await screen.findByText('纸张与边距');
  expect(repo.editSpec).toBeUndefined();
  expect(screen.queryByRole('heading', { name: '展览规格' })).toBeNull();
});

it('改观看距离只发一条规格命令，版面、纸张与状态都不跟着动', async () => {
  const user = userEvent.setup();
  const { store, repo } = specStore();
  render(<PublicationStudio {...props(repo)} />);
  const section = await specSection();
  expect(within(section).getByRole('button', { name: saveSpec })).toBeDisabled();
  const field = screen.getByRole('spinbutton', { name: distance });
  await user.clear(field);
  await user.type(field, '3200');
  expect(within(section).getByRole('button', { name: saveSpec })).toBeEnabled();
  await user.click(within(section).getByRole('button', { name: saveSpec }));
  expect(repo.editSpec).toHaveBeenCalledWith('project', 'book', 4, { viewingDistanceMm: 3200, hangCenterMm: 1450 });
  expect(await screen.findByRole('status')).toHaveTextContent('展览规格已保存');
  // 保存后回到库里那份：新值上屏、草稿清空、按钮重新按下去也不会重复发一条。
  expect(store.doc.revision).toBe(5);
  expect(screen.getByRole('spinbutton', { name: distance })).toHaveValue(3200);
  expect(within(section).getByRole('button', { name: saveSpec })).toBeDisabled();
  // 规格不是版面改动：不该顺手发出任何编辑命令，也不该多记一个版本。
  expect(repo.editFrame).not.toHaveBeenCalled();
  expect(repo.editPage).not.toHaveBeenCalled();
  expect(repo.editPages).not.toHaveBeenCalled();
  expect(repo.changeState).not.toHaveBeenCalled();
  expect(repo.saveVersion).not.toHaveBeenCalled();
});

it('留空就是不记录：清掉两个数发出去的是 null，不是 0', async () => {
  const user = userEvent.setup();
  const { repo } = specStore();
  render(<PublicationStudio {...props(repo)} />);
  const section = await specSection();
  await user.clear(screen.getByRole('spinbutton', { name: distance }));
  await user.clear(screen.getByRole('spinbutton', { name: hang }));
  await user.click(within(section).getByRole('button', { name: saveSpec }));
  expect(repo.editSpec).toHaveBeenCalledWith('project', 'book', 4, { viewingDistanceMm: null, hangCenterMm: null });
  expect(await screen.findByRole('status')).toHaveTextContent('展览规格已保存');
});

it('超出物理范围的数不让出门：按钮按住，并当场说清范围', async () => {
  const user = userEvent.setup();
  const { repo } = specStore();
  render(<PublicationStudio {...props(repo)} />);
  const section = await specSection();
  const field = screen.getByRole('spinbutton', { name: distance });
  await user.clear(field);
  await user.type(field, '20001');
  expect(await screen.findByText('观看距离须为 300–20000 毫米，悬挂中心线须为 300–3000 毫米，或留空。')).toBeInTheDocument();
  const button = within(section).getByRole('button', { name: saveSpec });
  expect(button).toBeDisabled();
  await user.click(button);
  expect(repo.editSpec).not.toHaveBeenCalled();
  // 改回范围内，提示消失、按钮恢复——它跟着的是这一份草稿，不是永久状态。
  await user.clear(field);
  await user.type(field, '20000');
  expect(screen.queryByText('观看距离须为 300–20000 毫米，悬挂中心线须为 300–3000 毫米，或留空。')).toBeNull();
  expect(button).toBeEnabled();
});

it('撤销改动把两个数回到库里那份；只读和看历史版本都不给编辑', async () => {
  const user = userEvent.setup();
  const { repo } = specStore();
  const { unmount } = render(<PublicationStudio {...props(repo)} />);
  const section = await specSection();
  const field = screen.getByRole('spinbutton', { name: distance });
  await user.clear(field);
  await user.type(field, '800');
  await user.click(within(section).getByRole('button', { name: '放弃调整' }));
  expect(screen.getByRole('spinbutton', { name: distance })).toHaveValue(2500);
  expect(within(section).getByRole('button', { name: saveSpec })).toBeDisabled();
  expect(repo.editSpec).not.toHaveBeenCalled();
  unmount();

  const readOnly = specStore();
  render(<PublicationStudio {...{ ...props(readOnly.repo), readOnly: true }} />);
  await specSection();
  expect(screen.getByRole('spinbutton', { name: distance })).toBeDisabled();
  expect(screen.getByRole('button', { name: saveSpec })).toBeDisabled();
  unmount();

  // 历史版本是已经印过的那一面墙，不能在本窗口里改它的观看距离。
  const history = specStore();
  render(<PublicationStudio {...props(history.repo)} />);
  await specSection();
  await user.click(await screen.findByRole('button', { name: /送印稿/ }));
  await screen.findByRole('button', { name: '返回当前版面' });
  expect(screen.queryByRole('heading', { name: '展览规格' })).toBeNull();
});

it('一次只留一份草稿：图注开着时规格读不进，规格草稿挂着时纸张让位', async () => {
  const user = userEvent.setup();
  const { repo } = specStore();
  render(<PublicationStudio {...props(repo)} />);
  await specSection();
  // 图注草稿开着：这两个框只是看一眼的，读不进新数，也不会被顺手改掉。
  await user.click(screen.getByRole('button', { name: '新增图注' }));
  expect(screen.getByRole('spinbutton', { name: distance })).toBeDisabled();
  expect(screen.getByRole('spinbutton', { name: hang })).toBeDisabled();
  await user.click(within(screen.getByRole('heading', { name: '图注' }).parentElement!).getByRole('button', { name: '放弃调整' }));
  // 反过来：规格草稿没落地时，改纸张会顶掉它，所以纸张那一侧整个按住。
  const field = screen.getByRole('spinbutton', { name: distance });
  await user.clear(field);
  await user.type(field, '3000');
  expect(screen.getByRole('spinbutton', { name: '页面宽度（mm）' })).toBeDisabled();
  expect(screen.getByRole('combobox', { name: '开本' })).toBeDisabled();
  expect(screen.getByRole('button', { name: '保存纸张' })).toBeDisabled();
  expect(screen.getByRole('button', { name: '新增图注' })).toBeDisabled();
  expect(repo.editPage).not.toHaveBeenCalled();
  expect(repo.addText).not.toHaveBeenCalled();
});

it('后端拒掉规格时说清原因，不冒充已保存', async () => {
  const user = userEvent.setup();
  const repo = repository({
    document: vi.fn().mockResolvedValue(wall()),
    editSpec: vi.fn().mockRejectedValue(new Error('观看距离须为 300–20000 毫米，或留空表示不记录')),
  });
  render(<PublicationStudio {...props(repo)} />);
  const section = await specSection();
  const field = screen.getByRole('spinbutton', { name: distance });
  await user.clear(field);
  await user.type(field, '4000');
  await user.click(within(section).getByRole('button', { name: saveSpec }));
  expect(await screen.findByRole('status')).toHaveTextContent('观看距离须为 300–20000 毫米，或留空表示不记录');
  expect(screen.queryByText('展览规格已保存')).toBeNull();
});
