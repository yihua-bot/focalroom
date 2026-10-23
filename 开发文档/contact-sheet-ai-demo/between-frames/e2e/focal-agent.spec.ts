import { test, expect } from '@playwright/test';
import { installAccountFixture, installDesktopBridge } from './account-fixture';
import { mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';

const rpc = async <T>(command: string, args: Record<string, unknown> = {}): Promise<T> => {
 const response = await fetch('http://localhost:1422/rpc', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ command, args }) }).then(r => r.json());
 if (!response.ok) throw new Error(response.error.message);
 return response.value;
};
test('同一会话自然回答、持续记录、真实草案入口与重启恢复', async ({ page }) => {
 const errors: string[] = []; const outside: string[] = [];
 page.on('pageerror', error => errors.push(error.message));
 await page.route('**/*', route => {
  if (['localhost','127.0.0.1','[::1]'].includes(new URL(route.request().url()).hostname)) return route.continue();
  outside.push(route.request().url()); return route.abort('internetdisconnected');
 });
 await installAccountFixture(page); await installDesktopBridge(page);
 const project = await rpc<{ id: string; revision: number; photos: unknown[]; sequence: unknown[] }>('create_project', { title: 'Focal AI 会话验收', brief: '寻找不同观看距离之间的节奏' });
 await page.goto('/#/project/' + project.id);
 await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
 const dialog = page.getByRole('dialog', { name: 'Focal AI', exact: true });
 await expect(dialog.getByRole('tablist')).toHaveCount(0);
 await dialog.getByRole('textbox', { name: '消息', exact: true }).fill('摄影中的节奏是什么？');
 await dialog.getByRole('button', { name: '发送', exact: true }).click();
 await expect(dialog.getByText('可以从观看距离与画面密度的变化开始理解。具体照片需要查看后再讨论。')).toBeVisible();
 await expect(dialog.getByText(/已查看.*张照片/)).toHaveCount(0);
 await dialog.getByRole('textbox', { name: '消息', exact: true }).fill('创建一个独立画布，给我一条编辑说明');
 await dialog.getByRole('button', { name: '发送', exact: true }).click();
 await expect(dialog.getByRole('button', { name: '查看画布方案' })).toBeVisible();
 const unchanged = await rpc<typeof project>('get_project', { projectId: project.id });
 expect(unchanged.revision).toBe(project.revision);
 expect(unchanged.sequence).toEqual([]);
 const directory = resolve('../docs/v3/evidence/focal-ai-v2/browser'); await mkdir(directory, { recursive: true });
 await page.screenshot({ path: resolve(directory, 'unified-conversation.png') });
 await dialog.getByRole('button', { name: '查看画布方案' }).click();
 const review = page.getByRole('dialog', { name: '智能编辑审稿', exact: true });
 await expect(review).toBeVisible();
 await expect(review.getByRole('complementary', { name: '改动详情' }).getByText('保留原版，在此比较空间关系', { exact: true })).toBeVisible();
 // 独立草案没有原画布；可查看实际节点，不能误合并到其他作品。
 await expect(review.getByRole('button', { name: '采纳选中改动', exact: true })).toBeDisabled();
 await review.getByRole('button', { name: '并排对比', exact: true }).click();
 await expect(review.locator('.canvas-sticky-node')).toHaveText('保留原版，在此比较空间关系');
 await expect(review.locator('.canvas-sticky-node')).toBeVisible();
 await page.screenshot({ path: resolve(directory, 'board-draft.png') });
 await rpc('test_process_restart');
 await page.reload();
 await page.getByRole('main').getByRole('button', { name: 'Focal AI', exact: true }).click();
 await expect(page.getByText('摄影中的节奏是什么？')).toBeVisible();
 await expect(page.getByRole('button', { name: '查看画布方案' })).toBeVisible();
 expect(outside).toEqual([]); expect(errors).toEqual([]);
});
