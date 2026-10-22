import { installAccountFixture } from './account-fixture';
import { test, expect } from '@playwright/test';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { defaultAgentBudget } from '../src/domain/agent';
import type { AgentProjection } from '../src/domain/agent';

const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
 const response = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
 if (!response.ok) throw new Error(response.error.message);
 return response.value;
};
type Project = { id: string; revision: number; sequence: string[]; photos: { id: string; status: string; note: string; rating: number }[] };
test('M2 真实后端：预算暂停、恢复、两份草案、人工冲突、局部采纳/撤销与重启', async ({ page }) => {
 const screenshots = resolve('../docs/v3/evidence/ai-budget-recovery-2026-10-22/browser');
 await mkdir(screenshots, { recursive: true });
 const errors: string[] = []; const outside: string[] = [];
 page.on('pageerror', error => errors.push(error.message));
 await page.route('**/*', route => {
  if (['localhost','127.0.0.1','[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
  outside.push(route.request().url()); return route.abort('internetdisconnected');
 });
 await installAccountFixture(page);
 await page.addInitScript(() => {
  Object.assign(globalThis, { isTauri: true, __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} }, __TAURI_INTERNALS__: {
   invoke: async (command: string, args: unknown) => {
    if (command.startsWith('plugin:event|')) return 0;
    const r = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
    if (!r.ok) throw r.error; return r.value;
   },
   convertFileSrc: (path: string) => 'http://localhost:1422/asset?path=' + encodeURIComponent(path), transformCallback: () => 0,
  } });
 });
 let project = await rpc<Project>('create_project', { title: 'M2 · 草案与人工决定', brief: '依据人工线索给两版作品序列；在采纳前保留我的正式编辑。' });
 const beforeHash = await rpc<string[][]>('test_fixture_checksums');
 const token = await rpc<string>('choose_import', { projectId: project.id });
 const jobId = await rpc<string>('import_photos', { projectId: project.id, selectionToken: token });
 await expect.poll(async () => (await rpc<{ status: string }>('get_import_job', { jobId })).status, { timeout: 100000 }).toBe('completed');
 project = await rpc<Project>('get_project', { projectId: project.id });
 expect(project.photos).toHaveLength(200);
 await page.goto('/#/project/' + project.id);
 await expect(page.getByRole('heading', { name: 'M2 · 草案与人工决定' })).toBeVisible();
 await page.screenshot({ path: resolve(screenshots, 'workspace.png') });
 await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
 // 低预算通过测试 RPC 创建，生产界面不暴露模型轮次参数。
 await rpc('agent_request', { request: { requestId: crypto.randomUUID(), projectId: project.id, command: { type: 'StartTask', goal: '给两版 25 张序列', selectedPhotoIds: [], targetCount: 25, budget: { ...defaultAgentBudget, maxModelTurns: 2 } } } });
 await expect(page.locator('.chat-task-status')).toHaveText('已暂停', { timeout: 15000 });
 const paused = (await rpc<AgentProjection>('get_agent_projection', { projectId: project.id })).tasks[0];
 expect(paused.budgetIssue?.kind).toBe('rounds');
 await expect(page.getByText('这次分析需要更多步骤。已保留进度，可以继续分析。')).toBeVisible();
 await expect(page.locator('.chat-task-status')).toBeInViewport();
 await page.screenshot({ path: resolve(screenshots, 'paused.png') });
 await page.reload();
 await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
 await page.getByRole('button', { name: '继续', exact: true }).click();
 await expect(page.locator('.chat-task-status')).toHaveText('待审核', { timeout: 20000 });
 const resumed = (await rpc<AgentProjection>('get_agent_projection', { projectId: project.id })).tasks[0];
 expect(resumed.id).toBe(paused.id);
 expect(resumed.budget.maxModelTurns).toBeGreaterThan(paused.usage.modelTurns);
 expect(resumed.budget.maxCloudImages).toBe(paused.budget.maxCloudImages);
 expect(resumed.budget.maxCostMinorUsd).toBe(paused.budget.maxCostMinorUsd);
 await expect(page.locator('.chat-draft-card')).toHaveCount(2);
 expect((await rpc<Project>('get_project', { projectId: project.id })).sequence).toHaveLength(0);
 await page.screenshot({ path: resolve(screenshots, 'drafts.png') });
 const saved = await rpc<AgentProjection>('get_agent_projection', { projectId: project.id });
 expect(saved.tasks[0].usage.cloudImages).toBe(0);
 expect(saved.tasks[0].usage.costMinorUsd).toBe(0);
 expect(saved.tasks[0].steps.filter(s => s.kind === 'tool').length).toBeGreaterThanOrEqual(7);
 // 在任务完成后人工保留一张未在候选中的照片，两个草案必须过期。
 project = await rpc<Project>('update_photo_decision', { projectId: project.id, expectedRevision: project.revision, photoId: project.photos[199].id, status: 'keep' });
 await page.reload();
 await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
 await expect(page.locator('.chat-task-status')).toHaveText('项目已更新');
 await page.getByRole('button', { name: '查看方案' }).first().click();
 const dialog = page.getByRole('dialog').filter({ has: page.getByRole('button', { name: '清除选择', exact: true }) });
 await expect(dialog.getByRole('button', { name: /^采纳所选/ })).toBeDisabled();
 await page.screenshot({ path: resolve(screenshots, 'stale.png') });
 await dialog.getByRole('button', { name: '基于当前版本重新比较' }).click();
 await expect(dialog.getByRole('button', { name: /^采纳所选/ })).toBeEnabled();
 await dialog.getByRole('button', { name: '清除选择', exact: true }).click();
 await dialog.getByRole('checkbox').first().check();
 await dialog.getByRole('button', { name: '采纳所选 1 项' }).click();
 await expect(dialog).toHaveCount(0);
 await expect(page.getByRole('button', { name: /^已保留\s*2$/ })).toBeVisible();
 const applied = await rpc<Project>('get_project', { projectId: project.id });
 expect(applied.photos[199].status).toBe('keep');
 await page.getByRole('button', { name: '撤销', exact: true }).click();
 await expect(page.getByRole('button', { name: /^已保留\s*1$/ })).toBeVisible();
 await page.getByRole('button', { name: '重做', exact: true }).click();
 await expect(page.getByRole('button', { name: /^已保留\s*2$/ })).toBeVisible();
 await page.screenshot({ path: resolve(screenshots, 'partial-approved.png') });
 // 第二份草案显式 rebase 后全量替换，再验证单组撤销。
 await page.getByRole('button', { name: '查看方案' }).nth(1).click();
 await dialog.getByRole('button', { name: '基于当前版本重新比较' }).click();
 await expect(dialog.getByRole('button', { name: '替换正式序列', exact: true })).toBeEnabled();
 await dialog.getByRole('button', { name: '替换正式序列', exact: true }).click();
 await dialog.getByRole('button', { name: '确认全量替换' }).click();
 await expect(page.getByRole('button', { name: /^已保留\s*25$/ })).toBeVisible();
 await page.getByRole('button', { name: '撤销', exact: true }).click();
 await expect(page.getByRole('button', { name: /^已保留\s*2$/ })).toBeVisible();
 await page.getByRole('button', { name: '重做', exact: true }).click();
 await expect(page.getByRole('button', { name: /^已保留\s*25$/ })).toBeVisible();
 await page.getByRole('button', { name: '作品序列', exact: true }).click();
 await expect(page.getByTestId('sequence-card')).toHaveCount(25);
 await page.screenshot({ path: resolve(screenshots, 'sequence.png') });
 const before = await rpc<Project>('get_project', { projectId: project.id });
 const agentBefore = await rpc<AgentProjection>('get_agent_projection', { projectId: project.id });
 await rpc('test_restart');
 expect(await rpc<Project>('get_project', { projectId: project.id })).toEqual(before);
 expect(await rpc<AgentProjection>('get_agent_projection', { projectId: project.id })).toEqual(agentBefore);
 expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(beforeHash);
 await page.reload();
 await page.getByRole('button', { name: '作品序列', exact: true }).click();
 await expect(page.getByTestId('sequence-card')).toHaveCount(25);
 await page.screenshot({ path: resolve(screenshots, 'restarted.png') });
 expect(errors).toEqual([]);expect(outside).toEqual([]);
 // 再启动明确的小预算任务，暂停后取消，GUI 仍可编辑。
 const response = await rpc<{ok: boolean}>('agent_request', { request: { requestId: crypto.randomUUID(), projectId: project.id, command: { type: 'StartTask', goal: '预算测试', selectedPhotoIds: [], targetCount: 25, budget: { ...defaultAgentBudget, maxModelTurns: 1 } } } });
 expect(response.ok).toBe(true);
 await expect.poll(async () => (await rpc<AgentProjection>('get_agent_projection', { projectId: project.id })).tasks[0].state).toBe('paused');
 const task = (await rpc<AgentProjection>('get_agent_projection', { projectId: project.id })).tasks[0];
 await rpc('agent_request', { request: { requestId: crypto.randomUUID(), projectId: project.id, command: { type: 'CancelTask', taskId: task.id } } });
 expect((await rpc<AgentProjection>('get_agent_projection', { projectId: project.id })).tasks[0].state).toBe('cancelled');
 // 选片对话不再提供独立的授权管理；本地 Mock 任务仍保持零图片外发。
 await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
 await expect(page.getByRole('button', { name: '管理图像授权' })).toHaveCount(0);
 expect((await rpc<AgentProjection>('get_agent_projection', {projectId: project.id})).tasks[0].usage.cloudImages).toBe(0);
 expect(await rpc<string[][]>('test_fixture_checksums')).toEqual(beforeHash);
 expect(errors).toEqual([]); expect(outside).toEqual([]);
});
