import fs from 'node:fs/promises';
import path from 'node:path';
import { root, origin, languages, pages, pagePath, readCatalog, translateHTML, escapeHTML, plainText, structuredData, translator } from './site-content.mjs';

const messages = await readCatalog();
const output = '// 由 locales/messages.tsv 与 locales/workspace.json 生成。\nwindow.FOCALROOM_MESSAGES = Object.freeze(' + JSON.stringify(messages, null, 2) + ');\n';
await fs.writeFile(path.join(root, 'dist/locales.js'), output);

const seo = {};
const urls = [];
const names = { zh: '中文', en: 'English', ja: '日本語', ko: '한국어' };
for (const [page, template] of Object.entries(pages)) {
  let source = await fs.readFile(path.join(root, `templates/${template}.html`), 'utf8');
  source = source.replace(/<link\b[^>]*\brel="alternate"[^>]*>/g, '')
    .replace(/<meta property="og:[^"]+"[^>]*>/g, '')
    .replace('<script src="i18n.js" defer>', '<script src="page-data.js" defer></script><script src="i18n.js" defer>')
    .replace('</head>', '<link rel="stylesheet" href="workspace.css"></head>');
  const chineseText = source.replace(/<select\b[^>]*data-language-select[\s\S]*?<\/select>/g, '');
  const keys = [...chineseText.matchAll(/>([^<>]*[\u3400-\u9fff][^<>]*)</g)].map(match => plainText(match[1]));
  const missing = [...new Set(keys.filter(key => !Object.hasOwn(messages, key)))];
  if (missing.length) throw new Error(`${template}: missing translations: ${JSON.stringify(missing)}`);
  seo[page] = {};
  for (const language of Object.keys(languages)) {
    let html = translateHTML(source, messages, language);
    html = html.replace(/<html lang="[^"]*"/, `<html lang="${languages[language]}" data-page-locale="${language}" data-page="${page}"`);
    html = html.replace(/\b(href|src)="([^"]+)"/g, (match, attribute, href) => {
      if (/^(#|\/|[a-z]+:)/i.test(href)) return match;
      const [, file, suffix = ''] = href.match(/^([^?#]+)(.*)$/) ?? [];
      const destinationPage = Object.entries(pages).find(([, value]) => `${value}.html` === file)?.[0];
      return `${attribute}="${destinationPage ? pagePath(destinationPage, language) : `/${file}`}${suffix}"`;
    });
    const url = origin + pagePath(page, language);
    const title = plainText(html.match(/<title>(.*?)<\/title>/s)[1]);
    const description = plainText(html.match(/<meta name="description" content="([^"]+)"/)[1]);
    const data = structuredData(page, language, html, title, description, messages);
    seo[page][language] = { url, title, description, data };
    const alternates = Object.entries(languages).map(([key, code]) => `<link rel="alternate" hreflang="${code}" href="${origin + pagePath(page, key)}">`).join('\n');
    const metadata = `<link rel="canonical" href="${url}">\n${alternates}\n<link rel="alternate" hreflang="x-default" href="${origin + pagePath(page, 'zh')}">\n<meta name="robots" content="index,follow,max-image-preview:large">\n<meta property="og:type" content="website"><meta property="og:site_name" content="FocalRoom"><meta property="og:url" content="${url}"><meta property="og:locale" content="${{ zh: 'zh_CN', en: 'en_US', ja: 'ja_JP', ko: 'ko_KR' }[language]}"><meta property="og:title" content="${escapeHTML(title)}"><meta property="og:description" content="${escapeHTML(description)}"><meta property="og:image" content="${origin}/assets/focalroom-mark.png"><meta property="og:image:alt" content="FocalRoom"><meta name="twitter:card" content="summary"><meta name="twitter:title" content="${escapeHTML(title)}"><meta name="twitter:description" content="${escapeHTML(description)}"><meta name="twitter:image" content="${origin}/assets/focalroom-mark.png">\n<script type="application/ld+json" id="structured-data">${JSON.stringify(data).replaceAll('<', '\\u003c')}</script>\n`;
    html = html.replace('</head>', `${metadata}</head>`);
    const languageLinks = `<nav class="locale-links" aria-label="${escapeHTML(translator(messages, language)('选择语言'))}" data-no-translate>${Object.entries(names).map(([key, label]) => `<a href="${pagePath(page, key)}" lang="${languages[key]}" hreflang="${languages[key]}" data-locale-link="${key}"${key === language ? ' aria-current="page"' : ''}>${label}</a>`).join('')}</nav>`;
    html = html.replace('<div class="footer-meta">', `${languageLinks}<div class="footer-meta">`);
    // 详细手册仅有中文，不伪造四语版本。
    html = html.replace(/href="\/user-guide.html\?lang=[^"]*"/g, 'href="/user-guide.html"');
    const directory = path.join(root, 'dist', pagePath(page, language));
    await fs.mkdir(directory, { recursive: true });
    await fs.writeFile(path.join(directory, 'index.html'), html);
    // 保留旧的 .html 入口供现有书签使用，canonical 指向新地址。
    if (language === 'zh' && page !== 'home') await fs.writeFile(path.join(root, `dist/${template}.html`), html);
    urls.push({ url, page, language });
  }
}
await fs.writeFile(path.join(root, 'dist/page-data.js'), `// 页面元信息与可见正文同时生成。\nwindow.FOCALROOM_PAGE_SEO = ${JSON.stringify(seo).replaceAll('<', '\\u003c')};\n`);
const sitemap = '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n' + urls.map(({ url }) => `  <url><loc>${url}</loc></url>`).join('\n') + '\n</urlset>\n';
await fs.writeFile(path.join(root, 'dist/sitemap.xml'), sitemap);
await fs.writeFile(path.join(root, 'dist/robots.txt'), `User-agent: *\nAllow: /\n\nSitemap: ${origin}/sitemap.xml\n`);
await fs.writeFile(path.join(root, 'dist/_redirects'), '/index.html / 301\n/zh/ / 301\n/guide.html /guide/ 301\n/privacy.html /privacy/ 301\n/workflow.html /workflow/ 301\n');
await fs.writeFile(path.join(root, 'dist/_headers'), '/*\n  X-Content-Type-Options: nosniff\n  Referrer-Policy: strict-origin-when-cross-origin\n/user-guide*\n  X-Robots-Tag: noindex\n');
await fs.writeFile(path.join(root, 'dist/404.html'), '<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex"><title>Page not found · FocalRoom</title><link rel="stylesheet" href="/styles.css"></head><body><main class="document-page wrap"><h1>Page not found</h1><p>FocalRoom</p><nav><a href="/">中文</a> · <a href="/en/">English</a> · <a href="/ja/">日本語</a> · <a href="/ko/">한국어</a></nav></main></body></html>\n');
// 可选阅读索引，不作为搜索或生成式引用的排名承诺，也不替代 HTML 正文。
for (const language of Object.keys(languages)) {
  const t = translator(messages, language);
  const content = `# FocalRoom\n\n${t('FocalRoom 是面向摄影创作者的 AI 原生创作工作空间。它关注照片如何共同构成作品，以项目连接人工判断、叙事与共同评审；最终决定由创作者作出。')}\n\n## ${t('现有功能')}\n${t('项目与照片导入、人工选片与比较、作品序列、版本记录、演示项目，以及 Web 登录和私密评片。')}\n\n## ${t('AI 选片草案')}\n${t('先导入照片，再告诉 AI 创作目标。真实分析依赖可用服务、账户额度与本次任务范围；草案经你采纳才改变正式选片。官网演示使用预设建议。')}\n\n## ${t('正在开发')}\n${t('叙事画布、完整的画布 Agent、Lightroom Classic / Capture One 往返、共享画布，以及摄影书和专业色彩输出。')}\n\n## ${t('规划中')}\n${t('Research & Moodboard：允许从无照片项目开始，整理创作意图、参考来源与研究板，并与后续编辑上下文连接。')}\n\n## Pages\n${Object.keys(pages).map(page => `- [${seo[page][language].title}](${seo[page][language].url})`).join('\n')}\n\n## Languages\n${Object.entries(names).map(([key, label]) => `- [${label}](${origin + pagePath('workflow', key)})`).join('\n')}\n`;
  await fs.writeFile(path.join(root, 'dist', language === 'zh' ? 'llms.txt' : `${language}/llms.txt`), content);
}
console.log(JSON.stringify({ languages: Object.keys(languages), messages: Object.keys(messages).length, staticPages: urls.length, canonicalOrigin: origin }));
