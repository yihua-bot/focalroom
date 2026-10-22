import { installAccountFixture } from './account-fixture';
// M5 Beta 完整旅程：意图 → 人工选片与备注 → Agent 委托 → 局部采纳 →
// 记忆假设纠正 → 导出 → 重启一致性。真实 Rust 后端（stdio harness）。
import { test, expect } from '@playwright/test';
import { mkdir, readFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';

const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
  const response = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
  if (!response.ok) throw new Error(response.error.message);
  return response.value;
};
type Project = { id: string; revision: number; sequence: string[]; photos: { id: string; status: string; note: string; rating: number }[] };

test('Beta 旅程：选片、委托、采纳、纠正假设、导出与重启一致', async ({ page }) => {
  const screenshots = resolve('../docs/screenshots/v1-m5');
  await mkdir(screenshots, { recursive: true });
  await page.route('**/*', route => {
    if (['localhost', '127.0.0.1', '[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
    return route.abort('internetdisconnected');
  });
  await installAccountFixture(page);
 await page.addInitScript(() => {
    Object.assign(globalThis, { isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} }, __TAURI_INTERNALS__: {
      invoke: async (command: string, args: unknown) => {
        if (command.startsWith('plugin:event|')) return 0;
        const r = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
        if (!r.ok) throw r.error;
        return r.value;
      },
      convertFileSrc: (path: string) => 'http://localhost:1422/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
    } });
  });
  // 1. 建项目 + 写创作意图。
  const project = await rpc<Project>('create_project', { title: '成长的摄影书', brief: '克制、日常、有时间感的家庭影像' });
  const token = await rpc<string>('choose_import', { projectId: project.id });
  const jobId = await rpc<string>('import_photos', { projectId: project.id, selectionToken: token });
  await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 120000 }).toBe('completed');
  let current = await rpc<Project>('get_project', { projectId: project.id });
  expect(current.photos).toHaveLength(200);
  // 2. 人工保留 8 张，其中 3 张带共同关键词备注（记忆素材）。
  const picked = current.photos.slice(0, 8).map(p => p.id);
  for (const photoId of picked) {
    current = await rpc<Project>('execute_gui_command', { request: { commandId: `beta-${photoId}`, projectId: project.id, expectedProjectRevision: current.revision, causationId: null, idempotencyKey: `beta-${photoId}`, command: { type: 'SetPhotoDecision', photoIds: [photoId], status: 'keep' } } });
  }
  for (const photoId of picked.slice(0, 3)) {
    current = await rpc<Project>('execute_gui_command', { request: { commandId: `beta-note-${photoId}`, projectId: project.id, expectedProjectRevision: current.revision, causationId: null, idempotencyKey: `beta-note-${photoId}`, command: { type: 'SetPhotoNote', photoId, note: '窗边的光线有层次，日常但克制' } } });
  }
  // 3. 委托 Agent（本地 Mock，零图像）。
  await page.goto('/#/project/' + project.id);
  await expect(page.getByRole('heading', { name: '成长的摄影书' })).toBeVisible();
  await page.getByRole('main').getByRole('button', { name: 'AI 智能选片', exact: true }).click();
  await page.getByRole('textbox', { name: '选片消息' }).fill('整理成长的摄影书，突出窗边的光线');
  await page.getByRole('button', { name: '发送', exact: true }).click();
  await expect(page.locator('.chat-task-status')).toHaveText('待审核', { timeout: 30000 });
  await expect(page.locator('.chat-draft-card')).toHaveCount(2);
  await page.screenshot({ path: resolve(screenshots, 'journey-drafts.png') });
  // 4. 记忆面板出现人工证据与假设，纠正一条假设（AG-11 旅程级）。
  await page.getByRole('button', { name: '审美记忆', exact: true }).click();
  await expect(page.getByText('人工证据')).toBeVisible();
  const memory = await rpc<{ evidence: unknown[]; hypotheses: { id: number; statement: string }[] }>('get_memory_projection', { projectId: project.id });
  expect(memory.evidence.length).toBeGreaterThanOrEqual(11);
  const target = memory.hypotheses.find(h => h.statement.includes('层次') || memory.hypotheses[0]);
  expect(target).toBeTruthy();
  await page.getByRole('button', { name: '纠正此假设' }).first().click();
  await page.getByRole('textbox', { name: '纠正说明' }).fill('光线只是巧合，不是我的稳定偏好');
  await page.getByRole('button', { name: '提交纠正' }).click();
  await expect(page.getByText('已纠正').first()).toBeVisible({ timeout: 10000 });
  const corrected = await rpc<{ hypotheses: { id: number; status: string; disabled: boolean }[] }>('get_memory_projection', { projectId: project.id });
  const after = corrected.hypotheses.find(h => h.id === target!.id);
  expect(after?.status).toBe('corrected');
  expect(after?.disabled).toBe(true);
  await page.screenshot({ path: resolve(screenshots, 'journey-memory.png') });
  // 5. 局部采纳一份草案的部分照片。
  await page.getByRole('main').getByRole('button', { name: 'AI 智能选片', exact: true }).click();
  await page.getByRole('button', { name: '查看方案' }).first().click();
  await page.getByRole('button', { name: '清除选择' }).click();
  const checkboxes = page.locator('.draft-items input[type="checkbox"]');
  const total = await checkboxes.count();
  expect(total).toBeGreaterThan(3);
  for (let i = 0; i < 3; i++) await checkboxes.nth(i).check();
  await page.getByRole('button', { name: '采纳所选 3 项' }).click();
  await page.waitForTimeout(2000);
  await page.screenshot({ path: resolve(screenshots, 'journey-applied.png') });
  // 6. 导出 CSV（默认私密）。
  const exported = await rpc<{ path: string }>('export_project', { projectId: project.id, format: 'csv', includePaths: false });
  expect(await readFile(exported.path, 'utf8')).not.toContain('original_path');
  // 6.1 关闭选片抽屉后从工作台导出。
  await page.getByRole('button', { name: '关闭 AI 智能选片' }).click();
  // 印刷参考走当前 GUI、IPC 和 Rust 导出器。
  await page.getByRole('button', { name: '导出 ↗' }).click();
  await page.getByRole('button', { name: '印刷参考', exact: true }).click();
  await expect(page.locator('.export-receipt')).toContainText('已导出：');
  const printPath = (await page.locator('.export-receipt p').innerText()).replace('已导出：', '');
  expect(printPath).toMatch(/\.html$/);
  const printHtml = await readFile(printPath, 'utf8');
  expect(printHtml).toContain('class="sheet"');
  // 实际排版成 PDF：HTML 节点数正确仍可能因页脚溢出而多印空白页。
  const printPage = await page.context().newPage();
  try {
    await printPage.setContent(printHtml);
    const pdf = await printPage.pdf({ preferCSSPageSize: true, printBackground: true });
    const pages = pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length ?? 0;
    expect(pages).toBe(1 + (printHtml.match(/class="sheet"/g)?.length ?? 0));
  } finally { await printPage.close(); }
  // 侧车不再是前端选项，后端兼容导出仍验证文件数量与路径脱敏。
  const sidecar = await rpc<{ path: string }>('export_project', { projectId: project.id, format: 'sidecar', includePaths: false });
  const sidecarPath = sidecar.path;
  const manifest = JSON.parse(await readFile(join(sidecarPath, 'manifest.json'), 'utf8'));
  expect(manifest.photos).toBe(200);
  expect(manifest.files).toHaveLength(200);
  expect(JSON.parse(await readFile(join(sidecarPath, manifest.files[0].file), 'utf8'))).not.toHaveProperty('originalPath');
  // 7. 重启一致性：重开数据库后证据/纠正/正式状态保持。
  const beforeRestart = await rpc<Project>('get_project', { projectId: project.id });
  await rpc('test_restart');
  expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(beforeRestart);
  const memoryAfter = await rpc<{ evidence: unknown[]; hypotheses: { id: number; status: string }[] }>('get_memory_projection', { projectId: project.id });
  expect(memoryAfter.hypotheses.find(h => h.id === target!.id)?.status).toBe('corrected');
  // 诊断为 harness 全局计数（同 run 前序 spec 的数据也计入），断言用下界。
  const diagnostics = await rpc<{ database: { photos: number; evidence: number }; provider: { providerId: string } }>('get_diagnostics');
  expect(diagnostics.database.photos).toBeGreaterThanOrEqual(200);
  expect(diagnostics.database.evidence).toBeGreaterThanOrEqual(11);
  expect(diagnostics.provider.providerId).toBe('mock');
});
