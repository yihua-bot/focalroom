import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';
import { z } from 'zod';
import { commandSchema, approvalSchema, mediaPolicySchema, toolResultSchema } from '../packages/contracts/src/schema.ts';

const root = fileURLToPath(new URL('../', import.meta.url));
const schema = z.toJSONSchema(commandSchema);
schema.$id = 'https://betweenframes.example/schemas/command-envelope.v2.json';
schema.title = 'Between Frames v2.0 validated command envelope';
// ID 数组的唯一性同时由 TS/Rust parseCommand 保护；JSON Schema 供独立客户端校验。
function uniqueIdArrays(value) {
  if (!value || typeof value !== 'object') return;
  if (value.properties) for (const [name, field] of Object.entries(value.properties)) {
    if (name.endsWith('_ids') && field.type === 'array') field.uniqueItems = true;
  }
  for (const child of Object.values(value)) {
    if (Array.isArray(child)) child.forEach(uniqueIdArrays);
    else uniqueIdArrays(child);
  }
}
uniqueIdArrays(schema);
const variants = schema.oneOf.map(branch => branch.properties.kind.const);
const rustName = name => name.split(/[._]/).map(part => part[0].toUpperCase() + part.slice(1)).join('');
const rust = '// 自动生成；请修改 packages/contracts/src/schema.ts 后运行 contracts:v2:generate。\n'
  + '#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]\n'
  + 'pub enum CommandKind {\n'
  + variants.map(kind => '    #[serde(rename = "' + kind + '")]\n    ' + rustName(kind) + ',\n').join('') + '}\n';
for (const [path, content] of [
  ['packages/contracts/generated/command-envelope.schema.json', JSON.stringify(schema, null, 2) + '\n'],
  ...[['approval', approvalSchema], ['media-policy', mediaPolicySchema], ['tool-result', toolResultSchema]].map(([name, definition]) => {
    const schema = z.toJSONSchema(definition);
    uniqueIdArrays(schema);
    return ['packages/contracts/generated/' + name + '.schema.json', JSON.stringify(schema, null, 2) + '\n'];
  }),
  ['crates/local-domain/src/kind_generated.rs', rust],
]) {
  const destination = root + path;
  if (process.argv.includes('--check')) {
    if (await readFile(destination, 'utf8') !== content) throw new Error('v2 契约漂移：' + path);
  } else {
    await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, content);
  }
}
console.log('PASS: v2 JSON Schema / Rust kind 映射一致（9 commands）');
