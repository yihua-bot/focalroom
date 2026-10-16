import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { resolve, join } from 'node:path';
import { evaluate, inputSchema, snapshotDigest } from './evaluation.ts';
import { z } from 'zod';
import { embedPreviews } from './preview.ts';

async function main() {
  const args = process.argv.slice(2);
  const previewIndex = args.indexOf('--previews');
  let previewDirectory: string | undefined;
  if (previewIndex >= 0) { previewDirectory = args[previewIndex + 1]; if (!previewDirectory) throw new Error('Missing previews'); args.splice(previewIndex, 2); }
  const [inputPath, outputPath, keyPath] = args;
  if (!inputPath || !outputPath || args.length > 3) throw new Error('Usage: npm run agent:eval -- INPUT.json NEW_OUTPUT_DIRECTORY [PREVIOUS_PRIVATE_KEY.json] [--previews CACHE_DIRECTORY]');
  const input = inputSchema.parse(JSON.parse(await readFile(resolve(inputPath), 'utf8')));
  let seed = randomBytes(32).toString('hex');
  const digest = snapshotDigest(input);
  if (keyPath) {
    const previous = z.object({ seed: z.string().regex(/^[a-f0-9]{64}$/), snapshotSha256: z.string() }).parse(JSON.parse(await readFile(resolve(keyPath), 'utf8')));
    if (previous.snapshotSha256 !== digest) throw new Error('评审材料已变化，需要重新建立盲评批次');
    seed = previous.seed;
  } else if (input.reviews.length) throw new Error('汇总评语必须提供原始 private-key 文件以固定候选映射');
  const result = evaluate(input, seed);
  const html = await readFile(new URL('./review.html', import.meta.url), 'utf8');
  const readyHtml = previewDirectory ? await embedPreviews(html, result.pack.publicPack, resolve(previewDirectory)) : null;
  // 必须新建目录，不覆盖既有评测与私人映射。评委只接收 blind-pack.json。
  const directory = resolve(outputPath);
  await mkdir(directory, { mode: 0o700 });
  for (const [name, data] of Object.entries({
    'input.snapshot.json': result.input,
    'report.json': result.report,
    'blind-pack.json': result.pack.publicPack,
    'private-key.json': { seed, snapshotSha256: digest, mapping: result.pack.privateKey },
    'review-template.json': { instructions: '填写真人 reviewerId、五维 1–5 分和至少一条可核验理由；将 reviews 合并回 input.snapshot.json，使用原 private-key.json 重新汇总。不要把 private-key 发给评委。', reviews: [] },
  })) await writeFile(join(directory, name), `${JSON.stringify(data, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  await writeFile(join(directory, 'review.html'), html, { flag: 'wx', mode: 0o600 });
  if (readyHtml) await writeFile(join(directory, 'review-ready.html'), readyHtml, { flag: 'wx', mode: 0o600 });
  process.stdout.write(`${JSON.stringify({ status: result.report.status, issueCount: result.report.issues.length, output: directory })}\n`);
  if (result.report.status === 'partial') process.exitCode = 2;
}
main().catch(error => {
  // 不回显输入内容（可能含私人目标或评语）。
  process.stderr.write(`${error instanceof Error && error.message.startsWith('Usage:') ? error.message : '评测失败：请检查输入格式、目标目录是否已存在及文件权限。'}\n`);
  process.exitCode = 1;
});
