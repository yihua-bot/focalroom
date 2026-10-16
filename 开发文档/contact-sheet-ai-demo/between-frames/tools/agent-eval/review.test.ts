// @vitest-environment node
import { expect, it } from 'vitest';
import { chromium } from '@playwright/test';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { embedPreviews } from './preview.ts';

it('offline blind review requires photos and human scores, exports alias-only records without network', async () => {
  const directory = mkdtempSync(join(tmpdir(), 'bf-blind-review-'));
  const previews = join(directory, 'previews'); mkdirSync(previews);
  const previewFilename = `${'a'.repeat(64)}-1400.jpg`;
  writeFileSync(join(previews, previewFilename), Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jL1sAAAAASUVORK5CYII=', 'base64'));
  const pack = { protocolVersion: 'bf-m3-eval-1' as const, cases: [{ caseId: 'case-1', goal: '</script><img src=https://example.invalid/private>', targetCount: 1,
    photos: [{ photoId: 'photo-1', previewFilename }], candidates: [{ candidateId: 'candidate-1', orderedPhotoIds: ['photo-1'], explanation: '待人工核对的顺序' }] }] };
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage({ acceptDownloads: true });
    const requests: string[] = []; page.on('request', r => { if (/^https?:/.test(r.url())) requests.push(r.url()); });
    await page.goto(pathToFileURL(resolve('tools/agent-eval/review.html')).href);
    await page.locator('#pack').setInputFiles({ name: 'blind-pack.json', mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(pack)) });
    await page.locator('#reviewer').fill('real-human-id');
    await page.locator('#save').click();
    expect(await page.locator('#status').textContent()).toContain('请载入候选预览');
    expect(await page.locator('#goal').textContent()).toContain(pack.cases[0].goal);
    expect(await page.locator('#goal img').count()).toBe(0);
    await page.locator('#assets').setInputFiles(previews);
    await page.locator('#photos button').first().click();
    expect(await page.locator('#loupe').isVisible()).toBe(true);
    await page.keyboard.press('Escape'); expect(await page.locator('#loupe').isVisible()).toBe(false);
    for (const key of ['visual_facts', 'creative_intent', 'sequence_rhythm', 'explanation', 'collaboration']) await page.locator(`#${key}`).selectOption('4');
    await page.locator('#reason').fill('第 1 张的主体位置与序列目标一致。');
    await page.locator('#checked').fill('1'); await page.locator('#incorrect').fill('2');
    await page.locator('#save').click(); expect(await page.locator('#progress').textContent()).toContain('0 条');
    await page.locator('#incorrect').fill('0'); await page.locator('#save').click();
    expect(await page.locator('#progress').textContent()).toContain('1 条');
    const downloadEvent = page.waitForEvent('download'); await page.locator('#export').click();
    const download = await downloadEvent; const saved = join(directory, 'reviews.json'); await download.saveAs(saved);
    const result = JSON.parse(readFileSync(saved, 'utf8'));
    expect(result.reviews).toHaveLength(1);
    expect(result.reviews[0]).toMatchObject({ caseId: 'case-1', candidateId: 'candidate-1', reviewerId: 'real-human-id', factCheck: { checkedClaims: 1, incorrectClaims: 0 } });
    expect(Object.keys(result.reviews[0])).not.toContain('system');
    expect(requests).toEqual([]);
    await page.screenshot({ path: join(directory, 'review.png'), fullPage: true });
    writeFileSync(join(previews, previewFilename), await page.screenshot({ type: 'jpeg', clip: { x: 0, y: 0, width: 4, height: 4 } }));
    const template = readFileSync(resolve('tools/agent-eval/review.html'), 'utf8');
    const readyPath = join(directory, 'ready.html');
    writeFileSync(readyPath, await embedPreviews(template, pack, previews));
    await page.goto(pathToFileURL(readyPath).href);
    expect(await page.locator('#status').textContent()).toContain('1 张本地预览');
    expect(await page.locator('#photos img').evaluate((image: HTMLImageElement) => image.naturalWidth)).toBe(4);
    expect(await page.locator('#goal').textContent()).toContain(pack.cases[0].goal);
    expect(await page.locator('#goal img').count()).toBe(0);
    expect(requests).toEqual([]);
    writeFileSync(join(previews, previewFilename), 'not an image');
    await expect(embedPreviews(template, pack, previews)).rejects.toThrow('JPEG');
  } finally { await browser.close(); }
}, 30000);
