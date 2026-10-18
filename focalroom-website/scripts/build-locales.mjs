import fs from 'node:fs/promises';
import path from 'node:path';
const root = path.resolve(import.meta.dirname, '..');
const input = await fs.readFile(path.join(root, 'locales/messages.tsv'), 'utf8');
const messages = {};
for (const [index, line] of input.split('\n').entries()) {
  if (!line) continue;
  const [zh, en, ja, ko, ...extra] = line.split('\t');
  if (!zh || extra.length || [en, ja, ko].some(value => value === undefined)) throw new Error(`Malformed locale row ${index + 1}`);
  if (Object.hasOwn(messages, zh)) throw new Error(`Duplicate locale key: ${zh}`);
  messages[zh] = { en, ja, ko };
}
const output = '// 由 locales/messages.tsv 生成；请修改源文案后运行 npm run build。\nwindow.FOCALROOM_MESSAGES = Object.freeze(' + JSON.stringify(messages, null, 2) + ');\n';
await fs.writeFile(path.join(root, 'dist/locales.js'), output);
console.log(JSON.stringify({ languages: ['zh', 'en', 'ja', 'ko'], messages: Object.keys(messages).length }));
