import { test, expect, type Page } from '@playwright/test';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cpus, totalmem, release, arch } from 'node:os';
import type { ImportJob } from '../src/domain/models';

type StoredProject = { id: string; revision: number; photos: { id: string; name: string; status: string; rating: number; note: string; originalPath: string; contentSha256: string; width: number; height: number }[]; sequence: string[]; history: { undoCount: number }; events: unknown[] };
type TestHost = typeof globalThis & { __M1_FIXTURE_SET__: string; __M1_JOBS__: string[]; __M1_COMMAND_MS__: number[]; __M1_FRAME_MS__: number[] };
const acceptance = process.env.BF_M1_ACCEPTANCE_SLICE || 'v2-m1-500';
const screenshots = resolve('../docs/screenshots', acceptance);
const evidence = resolve('../docs/evidence', acceptance);
const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
  const response = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  if (!response.ok) throw new Error(response.error.message);
  return response.value;
};
async function installBridge(page: Page) {
  const errors: string[] = [];
  const outside: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/*', route => {
    if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
    outside.push(route.request().url()); return route.abort('internetdisconnected');
  });
  await page.addInitScript(() => {
    const host = globalThis as TestHost;
    host.__M1_FIXTURE_SET__ = 'primary'; host.__M1_JOBS__ = []; host.__M1_COMMAND_MS__ = [];
    Object.assign(host, { isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} }, __TAURI_INTERNALS__: {
      invoke: async (command: string, args: Record<string, unknown>) => {
        if (command.startsWith('plugin:event|')) return 0;
        const started = performance.now();
        const actual = command === 'choose_import' ? { ...args, fixtureSet: host.__M1_FIXTURE_SET__ } : args;
        const result = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args: actual }) }).then(r => r.json());
        if (!result.ok) throw result.error;
        if (command === 'import_photos' || command === 'retry_import') host.__M1_JOBS__.push(result.value);
        if (command === 'execute_gui_command') host.__M1_COMMAND_MS__.push(performance.now() - started);
        return result.value;
      },
      convertFileSrc: (path: string) => 'http://localhost:1422/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
    } });
  });
  return { errors, outside };
}
async function startImport(page: Page, folder = true) {
  await page.getByRole('button', { name: '导入照片', exact: true }).first().click();
  await page.getByRole('button', { name: folder ? '选择文件夹' : '选择照片', exact: true }).click();
}
async function restartProcess(page: Page) {
  const url = page.url();
  // 关闭页面轮询后退出测试子进程，再以同一临时目录启动全新的 Rust 进程。
  await page.goto('about:blank');
  const restarted = await rpc<{ previousPid: number; pid: number }>('test_process_restart');
  expect(restarted.pid).not.toBe(restarted.previousPid);
  await page.goto(url);
  return restarted;
}
function processStats(pid: number) {
  try { return execFileSync('ps', ['-o', 'rss=', '-o', 'time=', '-p', String(pid)], { encoding: 'utf8' }).trim(); }
  catch { return null; }
}
async function snapshot(page: Page, name: string) {
  await mkdir(screenshots, { recursive: true });
  await page.screenshot({ path: resolve(screenshots, name + '.png') });
}
const distribution = (samples: number[]) => {
  const sorted = [...samples].sort((a, b) => a - b);
  const percentile = (p: number) => sorted[Math.max(0, Math.ceil(p * sorted.length) - 1)];
  return { count: samples.length, p50: percentile(.5), p95: percentile(.95), max: sorted.at(-1), samples };
};

// 主流程所有写入通过页面；RPC 只读断言、fixture hash 和模拟退出后重开数据库。
test('M1：500 张 PNG/JPEG 的完整 GUI 编辑、离线重启、导出与性能采样', async ({ page, browser }, testInfo) => {
  const { errors, outside } = await installBridge(page);
  const fixtures = await rpc<{ profile: string; count: number; jpeg: number; png: number; bytes: number; dimensions: string; provenance: string; harnessPid: number }>('test_fixture_info');
  expect(fixtures).toMatchObject({ profile: 'm1-500', count: 500, jpeg: 250, png: 250, dimensions: '1200x800' });
  const hashes = await rpc<string[][]>('test_fixture_checksums');
  expect(hashes).toHaveLength(500);
  expect(new Set(hashes.map(([, hash]) => hash)).size).toBe(500);
  await page.goto('/');
  await page.getByRole('button', { name: '新建项目', exact: true }).first().click();
  await page.locator('#project-title').fill('M1 · 500 张混合集');
  await page.getByRole('textbox', { name: '创作意图' }).fill('不使用 AI，完成 500 张照片的人工编辑与导出。');
  await page.getByRole('button', { name: '创建并打开' }).click();
  await expect(page.getByRole('heading', { name: '暂无照片' })).toBeVisible();
  const projectId = new URL(page.url()).hash.split('/').pop()!;
  const importStart = performance.now();
  await startImport(page);
  await expect(page.getByText('导入完成 · 500 / 500', { exact: true })).toBeVisible({ timeout: 180_000 });
  const importMs = performance.now() - importStart;
  const jobId = await page.evaluate(() => (globalThis as TestHost).__M1_JOBS__.at(-1)!);
  const job = await rpc<ImportJob>('get_import_job', { jobId });
  expect(job).toMatchObject({ total: 500, completed: 500, imported: 500, skipped: 0, failures: [] });
  await page.getByRole('button', { name: '完成', exact: true }).click();
  await expect(page.getByRole('button', { name: /^全部\s*500$/ })).toBeVisible();
  const imported = await rpc<StoredProject>('get_project', { projectId });
  expect(imported.photos).toHaveLength(500);
  expect(imported.photos.every(photo => photo.width === 1200 && photo.height === 800)).toBe(true);
  // 集成 harness 的 Mock 可执行，但此人工旅程不得调用模型或具备视觉外发能力。
  const model = await rpc<{ providerId: string; capabilities: string[] }>('get_ai_config');
  expect(model.providerId).toBe('mock');
  expect(model.capabilities).not.toContain('vision');
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Performance.enable');
  const beforeScroll = await cdp.send('Performance.getMetrics');
  const scroll = [];
  for (const fraction of [0, .25, .5, .75, 1, .5, 0]) {
    const frameMs = await page.locator('.workspace-scroll').evaluate(async (element, fraction) => {
      const started = performance.now(); element.scrollTop = (element.scrollHeight - element.clientHeight) * fraction;
      await new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
      return performance.now() - started;
    }, fraction);
    const mountedTiles = await page.locator('.tile').count();
    expect(mountedTiles).toBeGreaterThan(0);
    expect(mountedTiles).toBeLessThan(100);
    if (fraction === 1) await expect(page.getByTestId('photo-500')).toBeVisible();
    scroll.push({ fraction, mountedTiles, twoFrameMs: frameMs });
  }
  const afterScroll = await cdp.send('Performance.getMetrics');
  await page.locator('.workspace-scroll').evaluate(element => { element.scrollTop = element.scrollHeight; });
  await page.getByTestId('photo-500').getByRole('button', { name: /^查看第/ }).click();
  await page.evaluate(() => {
    const host = globalThis as TestHost;
    host.__M1_FRAME_MS__ = [];
    let pending: { started: number; status: string } | null = null;
    window.addEventListener('keydown', event => {
      if (['x', 'u'].includes(event.key)) pending = { started: performance.now(), status: event.key === 'x' ? 'reject' : 'undecided' };
    }, true);
    const observer = new MutationObserver(records => {
      if (!pending || !records.some(record => record.target instanceof HTMLElement && record.target.dataset.testid === 'photo-500' && record.target.classList.contains(pending!.status))) return;
      const started = pending.started; pending = null;
      requestAnimationFrame(() => host.__M1_FRAME_MS__.push(performance.now() - started));
    });
    observer.observe(document.querySelector('.workspace-scroll')!, { attributes: true, attributeFilter: ['class'], subtree: true });
  });
  const decisions = [];
  for (let index = 0; index < 40; index++) {
    const started = performance.now();
    await page.keyboard.press(index % 2 ? 'u' : 'x');
    await expect(page.getByTestId('photo-500')).toHaveClass(index % 2 ? /\bundecided\b/ : /\breject\b/);
    decisions.push(performance.now() - started);
  }
  await expect.poll(() => page.evaluate(() => (globalThis as TestHost).__M1_FRAME_MS__.length)).toBe(40);
  const keyToNextFrameMs = await page.evaluate(() => (globalThis as TestHost).__M1_FRAME_MS__);
  const afterDecisions = await cdp.send('Performance.getMetrics');
  const fullSnapshotBytes = Buffer.byteLength(JSON.stringify(await rpc('get_project', { projectId })), 'utf8');
  const compactSnapshotBytes = Buffer.byteLength(JSON.stringify(await rpc('get_project', { projectId, compact: true })), 'utf8');
  expect(compactSnapshotBytes).toBeLessThan(fullSnapshotBytes / 5);
  await snapshot(page, 'grid-end');
  await page.locator('.workspace-scroll').evaluate(element => { element.scrollTop = 0; });
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  for (let index = 1; index < 25; index++) await page.keyboard.press('Shift+ArrowRight');
  await expect(page.getByRole('group', { name: '批量选片' })).toContainText('已选 25 张');
  await page.getByRole('button', { name: '保留选区' }).click();
  await expect(page.getByRole('button', { name: /^已保留\s*25$/ })).toBeVisible();
  await page.getByRole('combobox', { name: '批量人工评分' }).selectOption('4');
  await expect(page.locator('.tile-rating').first()).toContainText('★★★★');
  await page.getByRole('button', { name: /^已保留\s*25$/ }).click();
  await expect(page.locator('.tile')).toHaveCount(25);
  await page.getByRole('button', { name: /^全部\s*500$/ }).click();
  await page.locator('.workspace-scroll').evaluate(element => { element.scrollTop = 0; });
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.getByRole('textbox', { name: '照片备注' }).fill('500 张中，保留这张作为序列的停顿。');
  await page.getByRole('button', { name: '保存备注' }).click();
  await expect(page.getByRole('button', { name: '保存备注' })).toBeDisabled();
  await expect(page.getByRole('button', { name: '人工评分 1 星', exact: true })).toBeEnabled();
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('Space');
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: '大图100% 像素' }).click();
  await expect(dialog.locator('.photo-viewport')).toHaveAttribute('data-source', 'original');
  await expect(dialog.locator('.photo-viewport img')).toHaveJSProperty('naturalWidth', 1200);
  expect(Math.round((await dialog.locator('.photo-viewport img').boundingBox())!.width)).toBe(1200);
  expect(await dialog.locator('.review-filmstrip img').evaluateAll(images => images.filter(image => (image as HTMLImageElement).naturalWidth > 0).length)).toBeLessThan(500);
  const filmstrip = dialog.getByLabel('胶片条导航');
  const filmstripBounds = await filmstrip.boundingBox();
  expect(filmstripBounds!.height).toBeLessThanOrEqual(65);
  const thumbBounds = await filmstrip.getByRole('button').first().boundingBox();
  expect(thumbBounds!.width).toBe(52);
  expect(thumbBounds!.height).toBe(40);
  expect(await filmstrip.evaluate(element => element.scrollWidth > element.clientWidth)).toBe(true);
  await page.keyboard.press('ArrowRight');
  await expect(dialog.getByRole('heading', { name: 'frame-001.jpg' })).toBeVisible();
  await snapshot(page, 'loupe');
  await page.keyboard.press('Escape');
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.getByRole('button', { name: '两图对比', exact: true }).click();
  await dialog.getByRole('button', { name: '左图100% 像素' }).click();
  await expect(dialog.locator('.photo-viewport').nth(1)).toHaveAttribute('data-mode', 'actual');
  await dialog.getByRole('button', { name: '左图放大' }).click();
  await expect(dialog.locator('.photo-viewport').nth(1)).toHaveAttribute('data-scale', '1.5');
  await snapshot(page, 'compare');
  await dialog.getByRole('button', { name: '不确定', exact: true }).click();
  await expect(dialog).toHaveCount(0);
  await page.getByRole('button', { name: '作品序列', exact: true }).click();
  await expect(page.getByTestId('sequence-card')).toHaveCount(25);
  const firstName = imported.photos[0].name;
  const initialSequence = (await rpc<StoredProject>('get_project', { projectId })).sequence;
  await page.getByRole('button', { name: `下移 ${firstName}` }).click();
  await expect(page.getByTestId('sequence-card').nth(1)).toContainText(firstName);
  for (let step = 0; step < 3; step++) {
    await page.getByRole('button', { name: '撤销', exact: true }).click();
    await expect(page.getByTestId('sequence-card').first()).toContainText(firstName);
    expect((await rpc<StoredProject>('get_project', { projectId })).sequence).toEqual(initialSequence);
    await page.getByRole('button', { name: '重做', exact: true }).click();
    await expect(page.getByTestId('sequence-card').nth(1)).toContainText(firstName);
    const ordered = (await rpc<StoredProject>('get_project', { projectId })).sequence;
    expect(new Set(ordered).size).toBe(25);
  }
  await page.getByRole('button', { name: /^作品版本/ }).click();
  await page.getByRole('textbox', { name: '版本名称' }).fill('500 选 25 · 人工版本');
  await page.getByRole('button', { name: '保存当前版本' }).click();
  await expect(page.getByRole('button', { name: /500 选 25 · 人工版本/ })).toBeVisible();
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: `下移 ${firstName}` }).click();
  await expect(page.getByTestId('sequence-card').nth(2)).toContainText(firstName);
  await page.getByRole('button', { name: /^作品版本/ }).click();
  await page.getByRole('button', { name: /500 选 25 · 人工版本/ }).click();
  await expect(page.getByRole('region', { name: '版本恢复预览' })).toContainText('位置变化 2 张');
  await page.getByRole('button', { name: '确认恢复此版本' }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByTestId('sequence-card').nth(1)).toContainText(firstName);
  const ipcMs = await page.evaluate(() => (globalThis as TestHost).__M1_COMMAND_MS__);
  const before = await rpc<StoredProject>('get_project', { projectId });
  const backendBeforeRestart = processStats(fixtures.harnessPid);
  const processRestart = await restartProcess(page);
  await expect(page.getByRole('button', { name: /^已保留\s*25$/ })).toBeVisible();
  expect(await rpc<StoredProject>('get_project', { projectId })).toEqual(before);
  await page.getByRole('button', { name: '作品序列', exact: true }).click();
  await snapshot(page, 'sequence-restarted');
  await page.getByRole('button', { name: '导出 ↗' }).click();
  // 正式界面只展示面向摄影用户的输出；兼容 JSON/CSV 继续验证后端契约。
  for (const format of ['json', 'csv'] as const) {
    const receipt = await rpc<{ path: string; sha256: string }>('export_project', { projectId, format, includePaths: false });
    const bytes = await readFile(receipt.path);
    expect(createHash('sha256').update(bytes).digest('hex')).toBe(receipt.sha256);
    expect(bytes.toString()).not.toContain(before.photos[0].originalPath);
    if (format === 'json') {
      const data = JSON.parse(bytes.toString());
      expect(data.photos).toHaveLength(500); expect(data.sequence).toEqual(before.sequence); expect(data.includesLocalPaths).toBe(false);
    } else expect(bytes.toString().split('\r\n')).toHaveLength(502);
  }
  await page.getByRole('button', { name: '照片总览', exact: true }).click();
  await expect(page.locator('.export-receipt')).toContainText('已导出：');
  const exportedPath = (await page.locator('.export-receipt p').innerText()).replace('已导出：', '');
  const exportedBytes = await readFile(exportedPath);
  await expect(page.locator('.export-receipt')).toContainText(createHash('sha256').update(exportedBytes).digest('hex'));
  expect(exportedBytes.toString()).not.toContain(before.photos[0].originalPath);
  expect(exportedBytes.toString().match(/<figure>/g)).toHaveLength(25);
  await page.keyboard.press('Escape');
  expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(hashes);
  const projection = await rpc<{ tasks: unknown[] }>('get_agent_projection', { projectId });
  expect(projection.tasks).toEqual([]);
  const system = await browser.newBrowserCDPSession();
  const gpu = await system.send('SystemInfo.getInfo');
  const finalMetrics = await cdp.send('Performance.getMetrics');
  const backendProcess = processStats(processRestart.pid);
  const processes = await system.send('SystemInfo.getProcessInfo');
  const rss = processes.processInfo.map(p => {
    try { return { type: p.type, id: p.id, cpuSeconds: p.cpuTime, rssKiB: Number(execFileSync('ps', ['-o', 'rss=', '-p', String(p.id)], { encoding: 'utf8' }).trim()) }; }
    catch { return { type: p.type, id: p.id, cpuSeconds: p.cpuTime, rssKiB: null }; }
  });
  const metrics = { fullSnapshotBytes, compactSnapshotBytes, fixture: fixtures, backend: 'release Rust/SQLite over loopback; native picker is simulated', browser: browser.version(), platform: process.platform, hardware: { cpu: cpus()[0]?.model, cores: cpus().length, totalMemoryBytes: totalmem(), architecture: arch(), osRelease: release() }, trace: testInfo.project.use.trace, importMs, automationDecisionMs: distribution(decisions), keyToNextFrameMs: distribution(keyToNextFrameMs), ipcMs: distribution(ipcMs), scroll, beforeScroll: beforeScroll.metrics, afterScroll: afterScroll.metrics, afterDecisions: afterDecisions.metrics, finalMetrics: finalMetrics.metrics, backendProcessRssKiBAndCpuTime: backendProcess, backendBeforeRestartRssKiBAndCpuTime: backendBeforeRestart, processRestart, processes: rss, gpuDevices: gpu.gpu.devices, gpuUtilization: null, gpuUtilizationNotice: 'CDP does not expose hardware utilization; device/process metrics are not GPU utilization', originalHashesMatched: 500, restoredSnapshotEqual: true, aiTasks: 0, externalRequests: outside, pageErrors: errors };
  await mkdir(evidence, { recursive: true });
  await writeFile(resolve(evidence, 'gui-metrics.json'), JSON.stringify(metrics, null, 2) + '\n');
  await testInfo.attach('500-photo-metrics', { body: JSON.stringify(metrics, null, 2), contentType: 'application/json' });
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});

test('M1 异常 GUI：重复、损坏、编码异常、超大图、取消保留已提交和失联重连', async ({ page }) => {
  const { errors, outside } = await installBridge(page);
  const project = await rpc<StoredProject>('create_project', { title: 'M1 · 异常恢复', brief: '' });
  const hashes = await rpc<string[][]>('test_fixture_checksums');
  await page.goto('/#/project/' + project.id);
  await startImport(page);
  await expect.poll(() => page.evaluate(() => (globalThis as TestHost).__M1_JOBS__.length)).toBeGreaterThan(0);
  const jobId = await page.evaluate(() => (globalThis as TestHost).__M1_JOBS__.at(-1)!);
  await expect.poll(async () => (await rpc<ImportJob>('get_import_job', { jobId })).imported).toBeGreaterThan(1);
  await page.getByRole('button', { name: '取消导入', exact: true }).click();
  await expect(page.getByText(/已取消；已完成照片保留/)).toBeVisible();
  const cancelled = await rpc<ImportJob>('get_import_job', { jobId });
  expect(cancelled.status).toBe('cancelled'); expect(cancelled.imported).toBeGreaterThan(1); expect(cancelled.imported).toBeLessThan(500);
  await page.getByRole('button', { name: '完成', exact: true }).click();
  const partial = await rpc<StoredProject>('get_project', { projectId: project.id });
  expect(partial.photos).toHaveLength(cancelled.imported);
  const processRestart = await restartProcess(page);
  expect(await rpc<StoredProject>('get_project', { projectId: project.id })).toEqual(partial);
  // 素材 000 已提交，异常组中同内容文件须跳过；其余三张逐张说明失败。
  await page.evaluate(() => { (globalThis as TestHost).__M1_FIXTURE_SET__ = 'failures'; });
  await startImport(page, false);
  await expect(page.getByText('导入完成 · 4 / 4', { exact: true })).toBeVisible();
  await expect(page.getByText('新增 0 张 · 重复跳过 1 张 · 失败 3 张', { exact: true })).toBeVisible();
  await expect(page.locator('.import-failures li')).toHaveCount(3);
  const failedJobId = await page.evaluate(() => (globalThis as TestHost).__M1_JOBS__.at(-1)!);
  const failed = await rpc<ImportJob>('get_import_job', { jobId: failedJobId });
  await snapshot(page, 'import-failures');
  await page.getByRole('button', { name: '重试失败照片', exact: true }).click();
  await expect(page.getByText('导入完成 · 3 / 3', { exact: true })).toBeVisible();
  await expect(page.getByText('新增 0 张 · 重复跳过 0 张 · 失败 3 张', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: '完成', exact: true }).click();
  expect(await rpc<StoredProject>('get_project', { projectId: project.id })).toEqual(partial);
  await rpc('test_move_source', { projectId: project.id });
  await page.reload();
  await expect(page.getByText('原图失联，缓存预览仍可查看。')).toBeVisible();
  await page.getByRole('button', { name: '重新定位原图' }).click();
  await expect(page.getByText('原图失联，缓存预览仍可查看。')).toHaveCount(0);
  expect(await rpc<string[][]>('test_fixture_checksums', { includeRelocated: true })).toEqual(hashes);
  const restored = await rpc<StoredProject>('get_project', { projectId: project.id });
  expect(restored.photos).toHaveLength(cancelled.imported);
  expect(restored.photos[0].contentSha256).toBe(partial.photos[0].contentSha256);
  await mkdir(evidence, { recursive: true });
  await writeFile(resolve(evidence, 'exception-results.json'), JSON.stringify({
    cancelled: { total: cancelled.total, processed: cancelled.completed, committed: cancelled.imported, status: cancelled.status },
    restartSnapshotEqual: true, processRestart,
    failures: failed.failures.map(failure => ({ file: failure.path.split('/').pop(), code: failure.code, message: failure.message })),
    duplicatesSkipped: failed.skipped,
    retry: { imported: 0, failed: 3 },
    relinkPreservedHash: true, originalHashesMatched: 500, pageErrors: errors, externalRequests: outside,
  }, null, 2) + '\n');
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});


test.describe('Retina 原图检视', () => {
  test.use({ deviceScaleFactor: 2 });
  test('M1 原图：2400px JPEG/PNG 按需检视、同步对比、失联回退与原图不变', async ({ page }) => {
  const { errors, outside } = await installBridge(page);
  const project = await rpc<StoredProject>('create_project', { title: 'M1 · 原图像素', brief: '' });
  await page.goto('/#/project/' + project.id);
  await page.evaluate(() => { (globalThis as TestHost).__M1_FIXTURE_SET__ = 'inspection'; });
  await startImport(page);
  await expect(page.getByText('导入完成 · 2 / 2', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: '完成', exact: true }).click();
  const imported = await rpc<StoredProject>('get_project', { projectId: project.id });
  expect(imported.photos.map(photo => [photo.width, photo.height])).toEqual([[2400, 1600], [2400, 1600]]);
  const hashes = await Promise.all(imported.photos.map(async photo => createHash('sha256').update(await readFile(photo.originalPath)).digest('hex')));
  const sourceRequests: string[] = [];
  page.on('request', request => { if (request.url().includes('fixture-inspection')) sourceRequests.push(request.url()); });
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('Space');
  const dialog = page.getByRole('dialog');
  const viewport = dialog.getByTestId('photo-viewport');
  await expect(viewport.locator('img')).toHaveJSProperty('naturalWidth', 1400);
  expect(sourceRequests).toHaveLength(0);
  await dialog.getByRole('button', { name: '大图100% 像素' }).click();
  await expect(viewport.locator('img')).toHaveJSProperty('naturalWidth', 2400);
  await expect(viewport).toHaveAttribute('data-source', 'original');
  const ratio = await page.evaluate(() => window.devicePixelRatio);
  await expect(viewport.locator('img')).toHaveCSS('width', `${2400 / ratio}px`);
  await snapshot(page, 'original-loupe');
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: '两图对比', exact: true }).click();
  await dialog.getByRole('button', { name: '左图100% 像素' }).click();
  for (const item of await dialog.getByTestId('photo-viewport').all()) {
    await expect(item).toHaveAttribute('data-source', 'original');
    await expect(item.locator('img')).toHaveJSProperty('naturalWidth', 2400);
  }
  await snapshot(page, 'original-compare');
  await dialog.getByRole('button', { name: '不确定', exact: true }).click();
  for (const [index, photo] of imported.photos.entries()) expect(createHash('sha256').update(await readFile(photo.originalPath)).digest('hex')).toBe(hashes[index]);
  await rpc('test_move_source', { projectId: project.id });
  await page.reload();
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('Space');
  await dialog.getByRole('button', { name: '大图100% 像素' }).click();
  await expect(dialog.getByRole('alert')).toContainText('原图失联');
  await expect(viewport).toHaveAttribute('data-source', 'preview');
  await expect(viewport.locator('img')).toHaveJSProperty('naturalWidth', 1400);
  await snapshot(page, 'original-missing');
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: '重新定位原图' }).click();
  await expect(page.getByText('原图失联，缓存预览仍可查看。')).toHaveCount(0);
  const relinked = await rpc<StoredProject>('get_project', { projectId: project.id });
  expect(relinked.photos.map(photo => photo.contentSha256)).toEqual(hashes);
  for (const [index, photo] of relinked.photos.entries()) expect(createHash('sha256').update(await readFile(photo.originalPath)).digest('hex')).toBe(hashes[index]);
  await mkdir(evidence, { recursive: true });
  await writeFile(resolve(evidence, 'original-inspection.json'), JSON.stringify({ dimensions: '2400x1600', cachedWidth: 1400, originalWidth: 2400, devicePixelRatio: ratio, cssWidth: 2400 / ratio, formats: ['JPEG', 'PNG'], linkedCompare: true, missingFallback: true, originalsUnchanged: true, pageErrors: errors, externalRequests: outside }, null, 2) + '\n');
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});

});


test('M1 RAW 格式边界：派生预览有界且不冒充原图或嵌入预览', async ({ page }) => {
  const { errors, outside } = await installBridge(page);
  const project = await rpc<StoredProject>('create_project', { title: 'RAW 格式边界', brief: '' });
  await page.goto('/#/project/' + project.id);
  await page.evaluate(() => { (globalThis as TestHost).__M1_FIXTURE_SET__ = 'raw'; });
  await startImport(page);
  await expect(page.getByText('导入完成 · 1 / 1', { exact: true })).toBeVisible();
  await page.getByRole('button', { name: '完成', exact: true }).click();
  await page.getByTestId('photo-1').getByRole('button', { name: /^查看第/ }).click();
  await page.keyboard.press('Space');
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: '大图100% 像素' }).click();
  const viewport = dialog.getByTestId('photo-viewport');
  await expect(viewport).toHaveAttribute('data-source', 'derived-preview');
  await expect(dialog.getByRole('note')).toHaveText('RAW 派生预览 · 非全分辨率');
  await expect(dialog.locator('.inspection-source')).toContainText('RAW 派生预览');
  await expect(dialog.locator('.inspection-source')).not.toContainText('原图');
  await expect(viewport.locator('img')).toHaveJSProperty('naturalWidth', 1200);
  await snapshot(page, 'raw-derived-preview');
  expect(errors).toEqual([]); expect(outside).toEqual([]);
});
