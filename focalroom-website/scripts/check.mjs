import fs from 'node:fs/promises';
import path from 'node:path';
import vm from 'node:vm';
import { spawnSync } from 'node:child_process';
import { root, readCatalog } from './site-content.mjs';

const messages = await readCatalog();
const errors = [];
for (const file of ['app.js', 'config.js', 'i18n.js', 'locales.js', 'page-data.js']) {
  const code = await fs.readFile(path.join(root, 'dist', file), 'utf8');
  new vm.Script(code, { filename: file });
  if (/\bfetch\s*\(|XMLHttpRequest|document\.cookie/.test(code)) errors.push(`${file}: unexpected network or cookies`);
  if (file !== 'i18n.js' && /localStorage/.test(code)) errors.push(`${file}: unexpected persistent storage`);
}
const app = await fs.readFile(path.join(root, 'dist/app.js'), 'utf8');
for (const [, key] of app.matchAll(/'([^'\n]*[\u3400-\u9fff][^'\n]*)'/g)) {
  if (!Object.hasOwn(messages, key)) errors.push(`Untranslated runtime string: ${key}`);
}
const generated = '// 由 locales/messages.tsv 与 locales/workspace.json 生成。\nwindow.FOCALROOM_MESSAGES = Object.freeze(' + JSON.stringify(messages, null, 2) + ');\n';
if (await fs.readFile(path.join(root, 'dist/locales.js'), 'utf8') !== generated) errors.push('Locale build is stale; run npm run build');
const configuration = JSON.parse(await fs.readFile(path.join(root, '.openai/hosting.json'), 'utf8'));
if (!configuration.project_id || configuration.static?.directory !== 'dist') errors.push('Invalid retained hosting manifest');
console.log(JSON.stringify({ translationKeys: Object.keys(messages).length, errors }, null, 2));
if (errors.length) process.exitCode = 1;
else {
  const result = spawnSync('python3', [path.join(root, 'scripts/check-static.py')], { stdio: 'inherit' });
  if (result.error) throw result.error;
  process.exitCode = result.status ?? 1;
}
