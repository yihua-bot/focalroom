import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { root, origin, languages, pages, pagePath } from './site-content.mjs';

// 可复用本机已有的 Playwright，不给静态站点引入运行时依赖。
const modulePath = process.argv[2];
const { chromium } = await import(modulePath ? pathToFileURL(modulePath).href : 'playwright');
const base = 'http://127.0.0.1:1441';
const results = [];
const errors = [];
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ locale: 'en-US' });
const page = await context.newPage();
page.on('pageerror', error => errors.push(error.message));
page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
page.on('request', request => {
  if (!request.url().startsWith(base)) errors.push(`Unexpected external request: ${request.url()}`);
});
const qa = path.join(root, 'qa/2026-10-21');
await fs.mkdir(qa, { recursive: true });
try {
  for (const language of Object.keys(languages)) {
    for (const width of [1440, 768, 375]) {
      await page.setViewportSize({ width, height: width === 1440 ? 960 : 900 });
      await page.goto(base + pagePath('home', language));
      await page.waitForFunction(() => window.FocalI18n && document.querySelectorAll('.demo-photo').length === 6);
      assert.equal(await page.locator('html').getAttribute('lang'), languages[language]);
      const size = await page.evaluate(() => ({ viewport: innerWidth, content: document.documentElement.scrollWidth }));
      assert.ok(size.content <= size.viewport + 1, `${language}/${width}: horizontal overflow ${JSON.stringify(size)}`);
      await page.screenshot({ path: path.join(qa, `${language}-${width}.png`), fullPage: true });
      results.push(`responsive:${language}:${width}`);
    }
  }
  await page.setViewportSize({ width: 1440, height: 960 });
  await page.goto(`${base}/en/`);
  await page.locator('#demo-suggest').click();
  assert.equal(await page.locator('#demo-draft li').count(), 3);
  assert.equal((await page.locator('#sequence-count').innerText()).trim(), '0 photos');
  await page.locator('#demo-draft button').click();
  assert.equal((await page.locator('#sequence-count').innerText()).trim(), '3 photos');
  for (const language of ['ja', 'ko', 'zh', 'en']) {
    await page.locator('[data-language-select]').selectOption(language);
    assert.equal(new URL(page.url()).pathname, pagePath('home', language));
    assert.equal(await page.locator('.sequence-item').count(), 3);
    const metadata = await page.evaluate(() => ({ canonical: document.querySelector('link[rel=canonical]').href, title: document.title, data: JSON.parse(document.querySelector('#structured-data').textContent) }));
    assert.equal(metadata.canonical, origin + pagePath('home', language));
    assert.equal(metadata.data['@graph'].find(x => x['@type'] === 'WebPage').name, metadata.title);
    const faq = metadata.data['@graph'].find(x => x['@type'] === 'FAQPage');
    const question = await page.locator('.faq-list summary').first().innerText();
    assert.equal(question.replace('+', '').trim(), faq.mainEntity[0].name);
  }
  await page.locator('#demo-undo').click();
  assert.equal(await page.locator('.sequence-item').count(), 0);
  results.push('demo:draft-accept-language-switch-undo-preserves-state-and-metadata');
  await page.locator('[data-open="workspace-dialog"]').click();
  assert.equal(await page.locator('#workspace-action').getAttribute('href'), 'https://team.focroom.cc/shared');
  await page.keyboard.press('Escape');
  assert.equal(await page.locator('#workspace-dialog').evaluate(node => node.open), false);
  results.push('workspace:real-https-link-dialog-escape');
  await page.goto(`${base}/ko/workflow/`);
  await page.locator('[data-language-select]').selectOption('ja');
  assert.equal(new URL(page.url()).pathname, '/ja/workflow/');
  assert.equal(await page.locator('a[href="/ja/guide/"]').count(), 1);
  results.push('documents:language-switch-retains-document-and-localizes-links');
  for (const language of Object.keys(languages)) {
    for (const width of [1440, 375]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(base + pagePath('workflow', language));
      const size = await page.evaluate(() => ({ viewport: innerWidth, content: document.documentElement.scrollWidth }));
      assert.ok(size.content <= size.viewport + 1, `workflow/${language}/${width}: horizontal overflow`);
      await page.screenshot({ path: path.join(qa, `workflow-${language}-${width}.png`), fullPage: true });
      results.push(`workflow-responsive:${language}:${width}`);
    }
  }
  await page.setViewportSize({ width: 375, height: 812 });
  await page.goto(`${base}/en/`);
  await page.locator('#menu-toggle').click();
  assert.equal(await page.locator('#menu-toggle').getAttribute('aria-expanded'), 'true');
  await page.locator('#main-nav a[href="#approach"]').click();
  assert.equal(await page.locator('#menu-toggle').getAttribute('aria-expanded'), 'false');
  results.push('mobile:menu-open-navigate-close');
  const noJS = await browser.newContext({ javaScriptEnabled: false, locale: 'ko-KR' });
  const plain = await noJS.newPage();
  for (const language of Object.keys(languages)) {
    for (const name of Object.keys(pages)) {
      const response = await plain.goto(base + pagePath(name, language));
      assert.equal(response.status(), 200);
      assert.equal(await plain.locator('html').getAttribute('lang'), languages[language]);
      assert.ok((await plain.locator('main').innerText()).length > 200);
      assert.equal(await plain.locator(`a[data-locale-link="${language}"]`).count(), 1);
      results.push(`no-javascript:${language}:${name}`);
    }
  }
  const notFound = await plain.goto(`${base}/not-a-real-page/`);
  assert.equal(notFound.status(), 404);
  await noJS.close();
  assert.deepEqual(errors, []);
  const report = { date: '2026-10-21', browser: 'Chromium headless', checks: results.length, results, errors, limits: ['Cloudflare redirects and headers require hosting verification', 'Not a native Safari or physical-device acceptance test'] };
  await fs.writeFile(path.join(qa, 'browser-results.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report, null, 2));
} finally {
  await browser.close();
}
