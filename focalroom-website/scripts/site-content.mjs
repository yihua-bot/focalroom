import fs from 'node:fs/promises';
import path from 'node:path';

export const root = path.resolve(import.meta.dirname, '..');
export const origin = 'https://focroom.cc';
export const languages = { zh: 'zh-CN', en: 'en', ja: 'ja', ko: 'ko' };
export const pages = { home: 'index', workflow: 'workflow', guide: 'guide', privacy: 'privacy' };
export const pagePath = (page, language) => `${language === 'zh' ? '/' : `/${language}/`}${page === 'home' ? '' : `${page}/`}`;
export const escapeHTML = value => String(value).replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
export const decodeHTML = value => value.replaceAll('&amp;', '&').replaceAll('&quot;', '"').replaceAll('&#39;', "'").replaceAll('&lt;', '<').replaceAll('&gt;', '>');

export async function readCatalog() {
  const messages = {};
  const lines = (await fs.readFile(path.join(root, 'locales/messages.tsv'), 'utf8')).split('\n').filter(Boolean);
  const additions = JSON.parse(await fs.readFile(path.join(root, 'locales/workspace.json'), 'utf8'));
  for (const rows of [lines.map(line => line.split('\t')), additions]) {
    const seen = new Set();
    for (const row of rows) {
      const [zh, en, ja, ko] = row;
      if (row.length !== 4 || row.some(value => typeof value !== 'string') || !zh || seen.has(zh)) throw new Error(`Invalid or duplicate translation: ${zh}`);
      seen.add(zh);
      for (const value of [en, ja, ko]) {
        if (!value && zh !== '到') throw new Error(`Empty translation: ${zh}`);
        const placeholders = text => [...text.matchAll(/\{\w+\}/g)].map(match => match[0]).sort().join(',');
        if (placeholders(value) !== placeholders(zh)) throw new Error(`Placeholder mismatch: ${zh}`);
      }
      messages[zh] = { en, ja, ko };
    }
  }
  return messages;
}

export function translator(messages, language) {
  return key => language === 'zh' ? key : messages[key]?.[language] ?? key;
}

// 模板不包含内联脚本；只翻译完整文本节点和受控属性，保留标签与链接结构。
export function translateHTML(html, messages, language) {
  const t = translator(messages, language);
  return html.split(/(<[^>]+>)/g).map(part => {
    if (part.startsWith('<')) return part.replace(/\b(aria-label|alt|content)="([^"]*)"/g, (_, attribute, value) => `${attribute}="${escapeHTML(t(decodeHTML(value)))}"`);
    const key = decodeHTML(part.trim());
    if (!Object.hasOwn(messages, key)) return part;
    return part.match(/^\s*/)[0] + escapeHTML(t(key)) + part.match(/\s*$/)[0];
  }).join('');
}

export const plainText = html => decodeHTML(html.replace(/<[^>]*>/g, '')).trim();

export function structuredData(page, language, html, title, description, messages) {
  const url = origin + pagePath(page, language);
  const graph = [
    { '@type': 'Organization', '@id': `${origin}/#organization`, name: 'FocalRoom', url: `${origin}/`, logo: `${origin}/assets/focalroom-mark.png`, email: 'sliqiang85@gmail.com', sameAs: ['https://www.instagram.com/yihuaphoto/'] },
    { '@type': 'WebSite', '@id': `${origin}/#website`, name: 'FocalRoom', url: `${origin}/`, inLanguage: Object.values(languages), publisher: { '@id': `${origin}/#organization` } },
    { '@type': 'WebPage', '@id': `${url}#webpage`, url, name: title, description, inLanguage: languages[language], isPartOf: { '@id': `${origin}/#website` } },
  ];
  if (page === 'home') {
    graph.push({ '@type': 'SoftwareApplication', '@id': `${origin}/#application`, name: 'FocalRoom', applicationCategory: 'MultimediaApplication', operatingSystem: 'macOS 11 or later (Apple Silicon)', url, description, featureList: translator(messages, language)('项目与照片导入、人工选片与比较、作品序列、版本记录、演示项目，以及 Web 登录和私密评片。'), publisher: { '@id': `${origin}/#organization` } });
    const questions = [...html.matchAll(/<details\b[^>]*>\s*<summary>([\s\S]*?)<\/summary>\s*<p>([\s\S]*?)<\/p>\s*<\/details>/g)];
    graph.push({ '@type': 'FAQPage', '@id': `${url}#faq`, inLanguage: languages[language], mainEntity: questions.map(([, question, answer]) => ({ '@type': 'Question', name: plainText(question.replace(/<span[^>]*>[\s\S]*?<\/span>/g, '')), acceptedAnswer: { '@type': 'Answer', text: plainText(answer) } })) });
  }
  return { '@context': 'https://schema.org', '@graph': graph };
}
