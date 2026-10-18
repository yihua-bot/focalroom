import fs from 'node:fs/promises';
import path from 'node:path';
import vm from 'node:vm';

const root = path.resolve(import.meta.dirname, '../dist');
const errors = [];
let links = 0;
const localeRows = (await fs.readFile(path.resolve(root, '../locales/messages.tsv'), 'utf8')).split('\n').filter(Boolean).map(line => line.split('\t'));
const messages = Object.fromEntries(localeRows.map(([zh, ...values]) => [zh, values]));
const localePages = ['index.html', 'privacy.html', 'guide.html'];
const decode = value => value.replaceAll('&amp;', '&').replaceAll('&quot;', '"').replaceAll('&#39;', "'").replaceAll('&lt;', '<').replaceAll('&gt;', '>');
for (const file of [...localePages, 'user-guide.html']) {
  const text = await fs.readFile(path.join(root, file), 'utf8');
  const ids = [...text.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
  if (new Set(ids).size !== ids.length) errors.push(`${file}: duplicate IDs`);
  for (const [, value] of text.matchAll(/\b(?:href|src)="([^"]+)"/g)) {
    if (/^(data:|https?:|mailto:|tel:)/.test(value)) continue;
    links++;
    const [queryTarget, anchor] = value.split('#');
    const target = queryTarget.split('?')[0];
    const destination = target ? path.resolve(root, target) : path.join(root, file);
    if (!destination.startsWith(`${root}/`)) errors.push(`Escaped directory: ${value}`);
    try {
      const contents = await fs.readFile(destination);
      if (anchor && !new RegExp(`\\bid="${anchor.replace(/[.*+?^$\{\}()|[\]\\]/g, '\\$&')}"`).test(contents.toString())) errors.push(`Missing anchor: ${file} -> ${value}`);
    } catch { errors.push(`Missing file: ${file} -> ${value}`); }
  }
  if (file === 'index.html') {
    if (/加入内测|种子用户|access-dialog|<form|®/.test(text)) errors.push('Unrequested recruitment, form, or trademark claim');
    if ((text.match(/<h1\b/g) ?? []).length !== 1) errors.push('Expected one main heading');
    if (!text.includes('样片由 AI 生成') || !text.includes('建议来自预设方案')) errors.push('Missing demo provenance');
  }
  if (localePages.includes(file)) {
    if (!text.includes('data-language-select')) errors.push(`${file}: missing language control`);
    const content = text.replace(/<select[^>]*data-language-select[\s\S]*?<\/select>/g, '');
    const keys = [
      ...Array.from(content.matchAll(/>([^<>]*[\u3400-\u9fff][^<>]*)</g), match => decode(match[1].trim())),
      ...Array.from(content.matchAll(/(?:aria-label|alt|content)="([^"]*[\u3400-\u9fff][^"]*)"/g), match => decode(match[1])),
    ];
    for (const key of keys) if (!Object.hasOwn(messages, key)) errors.push(`Missing translation: ${file} / ${key}`);
  }
}
for (const file of ['app.js', 'config.js', 'i18n.js', 'locales.js']) {
  const code = await fs.readFile(path.join(root, file), 'utf8');
  new vm.Script(code, { filename: file });
  if (/\bfetch\s*\(|XMLHttpRequest|document\.cookie/.test(code)) errors.push(`${file}: unexpected network or cookies`);
  if (file !== 'i18n.js' && /localStorage/.test(code)) errors.push(`${file}: unexpected persistent storage`);
}
for (const [key, ...values] of localeRows) {
  const expected = Array.from(key.matchAll(/\{\w+\}/g), match => match[0]).sort().join(',');
  if (values.length !== 3) errors.push(`Invalid locale row: ${key}`);
  for (const [index, value] of values.entries()) {
    if ((!value && key !== '到') || typeof value !== 'string') errors.push(`Empty translation ${index}: ${key}`);
    const placeholders = Array.from(value.matchAll(/\{\w+\}/g), match => match[0]).sort().join(',');
    if (placeholders !== expected) errors.push(`Placeholder mismatch: ${key}`);
  }
}
const app = await fs.readFile(path.join(root, 'app.js'), 'utf8');
for (const [, key] of app.matchAll(/'([^'\n]*[\u3400-\u9fff][^'\n]*)'/g)) if (!Object.hasOwn(messages, key)) errors.push(`Untranslated runtime string: ${key}`);
const generated = '// 由 locales/messages.tsv 生成；请修改源文案后运行 npm run build。\nwindow.FOCALROOM_MESSAGES = Object.freeze(' + JSON.stringify(Object.fromEntries(localeRows.map(([zh, en, ja, ko]) => [zh, {en, ja, ko}])), null, 2) + ');\n';
if (await fs.readFile(path.join(root, 'locales.js'), 'utf8') !== generated) errors.push('Locale build is stale; run npm run build');
const configuration = JSON.parse(await fs.readFile(path.resolve(root, '../.openai/hosting.json'), 'utf8'));
if (!configuration.project_id || configuration.static?.directory !== 'dist') errors.push('Invalid hosting manifest');
console.log(JSON.stringify({ pages: 4, languages: ['zh','en','ja','ko'], translationKeys: localeRows.length, localLinksAndResources: links, errors }, null, 2));
if (errors.length) process.exitCode = 1;
