import { test, expect, type Page } from '@playwright/test';
import { installAccountFixture } from './account-fixture';

type Project = { id: string; photos: Array<{ id: string }>; revision: number };
async function rpc<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const response = await fetch('http://localhost:1422/rpc', {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }),
  }).then(value => value.json());
  if (!response.ok) throw new Error(response.error.message);
  return response.value as T;
}

async function installDesktopBridge(page: Page) {
  await page.route('**/*', route => {
    const url = new URL(route.request().url());
    return ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname) ? route.continue() : route.abort();
  });
  await page.addInitScript(() => {
    Object.assign(globalThis, {
      isTauri: true,
      __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
      __TAURI_INTERNALS__: {
        invoke: async (command: string, args: unknown) => {
          if (command.startsWith('plugin:event|')) return 0;
          const result = await fetch('http://localhost:1422/rpc', {
            method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }),
          }).then(response => response.json());
          if (!result.ok) throw result.error;
          return result.value;
        },
        convertFileSrc: (path: string) => 'http://localhost:1422/asset?path=' + encodeURIComponent(path),
        transformCallback: () => 0,
      },
    });
  });
}

test('内置演示项目：创建画布、放照片、便签、撤销重做与重启恢复', async ({ page }) => {
  await rpc('test_seed_demo');
  const projectId = 'demo-first-walk';
  const project = await rpc<Project>('get_project', { projectId });
  expect(project.photos).toHaveLength(1);
  await installDesktopBridge(page);
  await page.goto('/#/project/' + projectId);
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await page.getByRole('button', { name: '新建画布', exact: true }).click();
  const boards = page.getByRole('combobox', { name: '选择画布' });
  await expect(boards).toHaveValue(/.+/);
  const boardId = await boards.inputValue();
  await page.getByRole('button', { name: /放入选中照片/ }).click();
  await expect(page.locator('.canvas-photo-node img')).toHaveCount(1);
  await expect.poll(() => page.locator('.canvas-photo-node img').evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await page.getByRole('button', { name: '便签', exact: true }).click();
  await page.locator('.canvas-surface')().click({ position: { x: 300, y: 200 } });
  await page.getByRole('textbox', { name: '便签内容' }).fill('演示项目也可以编排');
  await page.getByRole('button', { name: '保存', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveText('演示项目也可以编排');
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(0);
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveText('演示项目也可以编排');
  const before = await rpc('canvas_snapshot', { projectId, boardId });
  await page.goto('about:blank');
  await rpc('test_process_restart');
  await page.goto('/#/project/' + projectId);
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await expect(page.locator('.canvas-photo-node img')).toHaveCount(1);
  await expect(page.locator('.canvas-sticky-node')).toHaveText('演示项目也可以编排');
  expect(await rpc('canvas_snapshot', { projectId, boardId })).toEqual(before);
  await page.screenshot({ path: '../docs/v3/evidence/canvas-demo-fix-2026-10-22/demo-canvas.png' });
});

test('G3/G7 真实 Store：画布照片、便签、连接、出版生成与进程重启', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await installDesktopBridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: '新建项目', exact: true }).first().click();
  await page.getByRole('textbox', { name: '项目名称' }).fill('叙事与出版');
  await page.getByRole('button', { name: '创建并打开' }).click();
  await expect(page).toHaveURL(/#\/project\/[^/]+$/);
  const projectId = new URL(page.url()).hash.split('/').pop()!;
  // 前面的离线旅程会重定位一个源文件；本旅程按当前实际夹具验证完整导入。
  // 固定 200 张数量的断言继续由 local-core.spec.ts 覆盖。
  const fixtures = await rpc<string[][]>('test_fixture_checksums');
  const fixtureCount = fixtures.length;
  expect(fixtureCount).toBeGreaterThan(1);
  await page.getByRole('button', { name: '导入照片', exact: true }).first().click();
  await page.getByRole('button', { name: '选择文件夹' }).click();
  await expect(page.getByRole('status').filter({ hasText: new RegExp(`^导入完成 · ${fixtureCount} / ${fixtureCount}$`) })).toBeVisible({ timeout: 100_000 });
  await page.getByRole('button', { name: '完成', exact: true }).click();
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('k');
  await expect(page.getByTestId('photo-1')).toHaveClass(/keep/);
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await expect(page.getByText('画布功能需要桌面版。')).toHaveCount(0);
  await expect(page.locator('.sequence-board')).toHaveCount(0);
  await page.getByRole('button', { name: '新建画布', exact: true }).click();
  await expect(page.getByRole('combobox', { name: '选择画布' })).toHaveValue(/.+/);
  await page.getByRole('button', { name: /放入选中照片/ }).click();
  await expect(page.locator('.canvas-photo-node img')).toHaveCount(1);
  await expect.poll(() => page.locator('.canvas-photo-node img').evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await page.getByRole('button', { name: '便签', exact: true }).click();
  await page.locator('.canvas-surface')().click({ position: { x: 300, y: 200 } });
  await page.getByRole('textbox', { name: '便签内容' }).fill('用这张照片作为开篇');
  await page.getByRole('button', { name: '保存', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(1);
  await expect(page.locator('.canvas-sticky-node')).toHaveText('用这张照片作为开篇');
  await expect(page.locator('.canvas-photo-node')).toHaveCount(1);
  await page.getByRole('button', { name: '连接', exact: true }).click();
  await page.locator('.canvas-photo-node').click();
  await page.locator('.canvas-sticky-node').click();
  await expect(page.locator('.canvas-board line')).toHaveCount(1);
  expect(await page.locator('.canvas-board line').evaluate(line => (line as SVGElement).ownerSVGElement!.getBoundingClientRect().width)).toBeGreaterThan(0);
  // 实际鼠标画线，验证持久笔迹具有可绘制视口，再撤销以继续原历史旅程。
  await page.getByRole('button', { name: '画笔', exact: true }).click();
  const drawArea = await page.locator('.canvas-surface').boundingBox();
  expect(drawArea).not.toBeNull();
  await page.mouse.move(drawArea!.x + drawArea!.width - 140, drawArea!.y + 50);
  await page.mouse.down();
  await page.mouse.move(drawArea!.x + drawArea!.width - 60, drawArea!.y + 80, { steps: 5 });
  await page.mouse.up();
  await expect(page.locator('.canvas-board path')).toHaveCount(1);
  expect(await page.locator('.canvas-board path').evaluate(path => (path as SVGElement).ownerSVGElement!.getBoundingClientRect().width)).toBeGreaterThan(0);
  await page.screenshot({ path: '../docs/v3/evidence/agent-canvas-scope-2026-10-22/vector-rendering.png' });
  await page.getByRole('button', { name: '画笔', exact: true }).click();
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(page.locator('.canvas-board path')).toHaveCount(0);
  // 摄影师角色模拟：多步 Undo/Redo 必须恢复同一顺序，使用真实 Rust Store。
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(page.locator('.canvas-board line')).toHaveCount(0);
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(0);
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(1);
  await expect(page.locator('.canvas-board line')).toHaveCount(0);
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect(page.locator('.canvas-board line')).toHaveCount(1);
  const photo = page.locator('.canvas-photo-node');
  const previousWidth = await photo.evaluate(node => parseFloat((node as HTMLElement).style.width));
  const handle = await photo.locator('.canvas-resize-handle').boundingBox();
  expect(handle).not.toBeNull();
  await page.mouse.move(handle!.x + handle!.width / 2, handle!.y + handle!.height / 2);
  await page.mouse.down();
  await page.mouse.move(handle!.x + handle!.width / 2 + 40, handle!.y + handle!.height / 2 + 30, { steps: 5 });
  await page.mouse.up();
  await expect.poll(() => photo.evaluate(node => parseFloat((node as HTMLElement).style.width))).toBe(previousWidth + 40);
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect.poll(() => photo.evaluate(node => parseFloat((node as HTMLElement).style.width))).toBe(previousWidth);
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect.poll(() => photo.evaluate(node => parseFloat((node as HTMLElement).style.width))).toBe(previousWidth + 40);
  // 多选移动保留两个节点各自的原点，并用同一撤销操作恢复。
  const sticky = page.locator('.canvas-sticky-node');
  const positions = async () => Promise.all([photo, sticky].map(node => node.evaluate(element => ({ x: parseFloat((element as HTMLElement).style.left), y: parseFloat((element as HTMLElement).style.top) }))));
  const beforeMove = await positions();
  await photo.click();
  await sticky.click({ modifiers: ['Shift'] });
  const photoBox = await photo.boundingBox();
  expect(photoBox).not.toBeNull();
  await page.mouse.move(photoBox!.x + 20, photoBox!.y + 20);
  await page.mouse.down();
  await page.mouse.move(photoBox!.x + 50, photoBox!.y + 40, { steps: 5 });
  await page.mouse.up();
  await expect.poll(positions).toEqual(beforeMove.map(point => ({ x: point.x + 30, y: point.y + 20 })));
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect.poll(positions).toEqual(beforeMove);
  await photo.click();
  await page.getByRole('button', { name: '锁定 / 解锁', exact: true }).click();
  await expect(photo.locator('.canvas-lock-badge')).toHaveCount(1);
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(photo.locator('.canvas-lock-badge')).toHaveCount(0);
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect(photo.locator('.canvas-lock-badge')).toHaveCount(1);
  await page.screenshot({ path: '../docs/v3/evidence/canvas-commands-2026-10-22/canvas-history.png' });
  await page.getByRole('button', { name: '出版物', exact: true }).click();
  await expect(page.getByText('出版功能需要桌面版。')).toHaveCount(0);
  await expect(page.locator('.canvas-wrapper')).toHaveCount(0);
  await page.getByRole('textbox', { name: '出版物标题' }).fill('光与停顿');
  await page.getByRole('button', { name: '创建', exact: true }).click();
  await expect(page.locator('.publication-card')).toHaveCount(1);
  await page.getByRole('button', { name: /从序列生成/ }).click();
  await expect(page.getByRole('status').filter({ hasText: '已生成 1 页初稿' })).toBeVisible();
  await page.getByRole('button', { name: '查看页面', exact: true }).click();
  await expect(page.locator('.publication-page-preview')).toHaveCount(2);
  await expect.poll(() => page.locator('.publication-frame img').first().evaluate(image => (image as HTMLImageElement).naturalWidth)).toBeGreaterThan(0);
  await page.screenshot({ path: '../docs/v3/evidence/canvas-commands-2026-10-22/publication.png' });
  await page.getByRole('button', { name: /从序列生成/ }).click();
  await expect(page.getByRole('status').filter({ hasText: /已有页面/ })).toBeVisible();
  await page.goto('about:blank');
  await rpc('test_process_restart');
  await page.goto('/');
  await page.getByText('叙事与出版', { exact: true }).first().click();
  await page.getByRole('button', { name: '出版物', exact: true }).click();
  await expect(page.getByText('光与停顿', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await expect(page.locator('.canvas-photo-node img')).toHaveCount(1);
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(1);
  const reopened = await rpc<Project>('get_project', { projectId });
  expect(reopened.photos.length).toBeGreaterThan(0);
  expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(fixtures);
  expect(errors).toEqual([]);
});


test('SIMULATED 保存确认：提交后丢失回执、重启恢复及过期编辑冲突', async ({ page }) => {
  await installDesktopBridge(page);
  await page.goto('/');
  await page.getByRole('button', { name: '新建项目', exact: true }).first().click();
  await page.getByRole('textbox', { name: '项目名称' }).fill('保存恢复模拟');
  await page.getByRole('button', { name: '创建并打开' }).click();
  await expect(page).toHaveURL(/#\/project\/[^/]+$/);
  const projectUrl = page.url();
  const projectId = new URL(projectUrl).hash.split('/').pop()!;
  const sourceHashes = await rpc<string[][]>('test_fixture_checksums');
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await page.getByRole('button', { name: '新建画布', exact: true }).click();
  const choose = page.getByRole('combobox', { name: '选择画布' });
  await expect(choose).toHaveValue(/.+/);
  const boardId = await choose.inputValue();
  let lost = false;
  await page.route('http://localhost:1422/rpc', async route => {
    const data = route.request().postDataJSON();
    if (!lost && data?.command === 'canvas_execute' && data.args?.request?.operations?.[0]?.action?.type === 'AddText') {
      lost = true;
      const result = await (await route.fetch()).json();
      expect(result.ok).toBe(true); // 确实已由 Rust 提交，随后只丢失客户端回执。
      await route.fulfill({ json: { ok: false, error: { code: 'DATABASE', message: 'connection lost after commit', retriable: true } } });
    } else await route.continue();
  });
  await page.getByRole('button', { name: '便签', exact: true }).click();
  await page.locator('.canvas-surface')().click({ position: { x: 300, y: 200 } });
  await page.getByRole('textbox', { name: '便签内容' }).fill('重启后只保留一张便签');
  await page.getByRole('button', { name: '保存', exact: true }).click();
  await expect(page.getByRole('button', { name: '确认上次操作', exact: true })).toBeVisible();
  const saved = await rpc<{ board: { revision: number }; nodes: unknown[] }>('canvas_snapshot', { projectId, boardId });
  expect(saved.board.revision).toBe(2); expect(saved.nodes).toHaveLength(1);
  await page.goto('about:blank');
  const restarted = await rpc<{ previousPid: number; pid: number }>('test_process_restart');
  expect(restarted.previousPid).not.toBe(restarted.pid);
  await page.goto(projectUrl);
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(1);
  await expect(page.getByRole('button', { name: '便签', exact: true })).toBeDisabled();
  await page.screenshot({ path: '../docs/v3/evidence/canvas-commands-2026-10-22/pending-after-restart.png' });
  await page.getByRole('button', { name: '确认上次操作', exact: true }).click();
  await expect(page.getByRole('button', { name: '确认上次操作', exact: true })).toHaveCount(0);
  const confirmed = await rpc<{ board: { revision: number }; nodes: unknown[] }>('canvas_snapshot', { projectId, boardId });
  expect(confirmed).toEqual(saved);
  await page.getByRole('button', { name: '撤销', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(0);
  await page.getByRole('button', { name: '重做', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(1);
  // 另一编辑者角色模拟：真实后端推进版本，当前页面保持旧快照。
  await rpc('canvas_execute', { request: {
    schemaVersion: 3, commandId: crypto.randomUUID(), idempotencyKey: crypto.randomUUID(),
    projectId, boardId, baseRevision: 4,
    operations: [{ opId: crypto.randomUUID(), action: { type: 'AddText', data: {
      boardId, nodeType: 'sticky', x: 400, y: 100, width: 200, height: 120, content: '另一位编辑者的内容',
    } } }],
  } });
  await page.getByRole('button', { name: '便签', exact: true }).click();
  await page.locator('.canvas-surface')().click({ position: { x: 300, y: 200 } });
  await page.getByRole('textbox', { name: '便签内容' }).fill('保留我的未提交想法');
  await page.getByRole('button', { name: '保存', exact: true }).click();
  await expect(page.locator('.canvas-notice')).toHaveText('画布已更新，请刷新后重新操作');
  const noticeBounds = await page.locator('.canvas-notice').boundingBox();
  const canvasBounds = await page.locator('.canvas-wrapper').boundingBox();
  expect(noticeBounds).not.toBeNull(); expect(canvasBounds).not.toBeNull();
  expect(noticeBounds!.y + noticeBounds!.height).toBeLessThanOrEqual(canvasBounds!.y + canvasBounds!.height);
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(2);
  await expect(page.getByRole('textbox', { name: '便签内容' })).toHaveValue('保留我的未提交想法');
  await page.screenshot({ path: '../docs/v3/evidence/canvas-commands-2026-10-22/conflict-preserves-input.png' });
  // 显式重新点击才能基于新快照保存，不偷偷改版本重试。
  await page.getByRole('button', { name: '保存', exact: true }).click();
  await expect(page.locator('.canvas-sticky-node')).toHaveCount(3);
  const final = await rpc<{ board: { revision: number } }>('canvas_snapshot', { projectId, boardId });
  expect(final.board.revision).toBe(6);
  expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(sourceHashes);
  await page.screenshot({ path: '../docs/v3/evidence/canvas-commands-2026-10-22/recovered.png' });
});

test('SIMULATED Agent 画布委托：真实权限、独立分支、原布局对照、人工采纳与重启', async ({ page }) => {
  await installDesktopBridge(page);
  await installAccountFixture(page);
  const outside: string[] = [];
  page.on('request', request => { if (!['localhost', '127.0.0.1', '[::1]'].includes(new URL(request.url()).hostname)) outside.push(request.url()); });
  const project = await rpc<Project>('create_project', { title: '画布提案模拟', brief: '保留原版，仅在独立方案中探索空间关系。' });
  const hashes = await rpc<string[][]>('test_fixture_checksums');
  const selectionToken = await rpc<string>('choose_import', { projectId: project.id });
  const jobId = await rpc<string>('import_photos', { projectId: project.id, selectionToken });
  await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 100_000 }).toBe('completed');
  await page.goto('/#/project/' + project.id);
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.getByRole('button', { name: '叙事画布', exact: true }).click();
  await page.getByRole('button', { name: '新建画布', exact: true }).click();
  await page.getByRole('button', { name: /放入选中照片/ }).click();
  await expect(page.locator('.canvas-photo-node')).toHaveCount(1);
  await page.getByRole('button', { name: '便签', exact: true }).click();
  await page.locator('.canvas-surface')().click({ position: { x: 300, y: 200 } });
  await page.getByRole('textbox', { name: '便签内容' }).fill('原版意图：用留白维持节奏');
  await page.getByRole('button', { name: '保存', exact: true }).click();
  const sourceId = await page.getByRole('combobox', { name: '选择画布' }).inputValue();
  const source = await rpc('canvas_snapshot', { projectId: project.id, boardId: sourceId });
  await page.getByRole('button', { name: 'AI 编排', exact: true }).click();
  await expect(page.getByText('基于当前画布', { exact: true })).toBeVisible();
  await expect(page.locator('.canvas-wrapper')).toBeVisible();
  await page.getByRole('textbox', { name: '消息' }).fill('在独立方案中尝试空间呼应');
  await page.getByRole('button', { name: '发送', exact: true }).click();
  await expect(page.getByRole('button', { name: '查看画布方案', exact: true })).toBeEnabled({ timeout: 30_000 });
  expect(await rpc('canvas_snapshot', { projectId: project.id, boardId: sourceId })).toEqual(source);
  await page.getByRole('button', { name: '查看画布方案', exact: true }).click();
  const review = page.getByRole('dialog', { name: '智能编辑审稿', exact: true });
  await expect(review).toBeVisible();
  await expect(review.getByRole('button', { name: '采纳选中改动', exact: true })).toBeDisabled();
  await review.getByRole('button', { name: '关闭对话框', exact: true }).click();
  await expect(page.getByText('方案变化', { exact: true })).toBeVisible();
  await expect(page.getByText('调整 2', { exact: true })).toBeVisible();
  await expect(page.getByText('新增 2', { exact: true })).toBeVisible();
  await expect(page.getByTestId('canvas-baseline-node')).toHaveCount(2);
  await page.getByRole('button', { name: '适合窗口', exact: true }).click();
  const surface = await page.locator('.canvas-surface').boundingBox();
  expect(surface).not.toBeNull();
  for (const item of await page.locator('.canvas-photo-node,.canvas-sticky-node,.canvas-baseline-node').all()) {
    const box = await item.boundingBox(); expect(box).not.toBeNull();
    expect(box!.x).toBeGreaterThanOrEqual(surface!.x);
    expect(box!.y).toBeGreaterThanOrEqual(surface!.y);
    expect(box!.x + box!.width).toBeLessThanOrEqual(surface!.x + surface!.width);
    expect(box!.y + box!.height).toBeLessThanOrEqual(surface!.y + surface!.height);
  }
  const draftId = await page.getByRole('combobox', { name: '选择画布' }).inputValue();
  expect(draftId).not.toBe(sourceId);
  await expect(page.getByRole('button', { name: /放入选中照片/ })).toBeDisabled();
  await page.screenshot({ path: '../docs/v3/evidence/agent-canvas-scope-2026-10-22/proposal-with-baseline.png' });
  await page.getByRole('button', { name: '另存为独立方案', exact: true }).click();
  await page.getByRole('button', { name: '取消', exact: true }).click();
  const pending = await rpc<{ board: { owning_task_id: string | null } }>('canvas_snapshot', { projectId: project.id, boardId: draftId });
  expect(pending.board.owning_task_id).not.toBeNull();
  await page.getByRole('button', { name: '另存为独立方案', exact: true }).click();
  await page.getByRole('dialog').getByRole('button', { name: '另存为独立方案', exact: true }).click();
  await expect(page.getByRole('button', { name: '另存为独立方案', exact: true })).toHaveCount(0);
  const accepted = await rpc<{ board: { owning_task_id: string | null; state: string } }>('canvas_snapshot', { projectId: project.id, boardId: draftId });
  expect(accepted.board).toMatchObject({ owning_task_id: null, state: 'official' });
  expect(await rpc('canvas_snapshot', { projectId: project.id, boardId: sourceId })).toEqual(source);
  await page.goto('about:blank');
  await rpc('test_process_restart');
  const after = await rpc<{ tasks: Array<{ boardDrafts: Array<{ id: string; state: string }> }> }>('get_agent_projection', { projectId: project.id });
  expect(after.tasks[0].boardDrafts).toEqual(expect.arrayContaining([expect.objectContaining({ id: draftId, state: 'official' })]));
  expect(await rpc('canvas_snapshot', { projectId: project.id, boardId: draftId })).toEqual(accepted);
  expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(hashes);
  expect(outside).toEqual([]);
});
