import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { resolve, join } from 'node:path';
import { evaluate, inputSchema, snapshotDigest } from './evaluation.ts';
import { z } from 'zod';
import { embedPreviews } from './preview.ts';

async function main() {
  const args = process.argv.slice(2);
  const reviewPaths: string[] = [];
  for (let index = args.indexOf('--reviews'); index >= 0; index = args.indexOf('--reviews')) {
    const path = args[index + 1];
    if (!path || path.startsWith('--')) throw new Error('Missing reviews');
    reviewPaths.push(path); args.splice(index, 2);
  }
  const previewIndex = args.indexOf('--previews');
  let previewDirectory: string | undefined;
  if (previewIndex >= 0) { previewDirectory = args[previewIndex + 1]; if (!previewDirectory) throw new Error('Missing previews'); args.splice(previewIndex, 2); }
  const [inputPath, outputPath, keyPath] = args;
  if (!inputPath || !outputPath || args.length > 3) throw new Error('Usage: npm run agent:eval -- INPUT.json NEW_OUTPUT_DIRECTORY [PREVIOUS_PRIVATE_KEY.json] [--reviews EXPORTED_REVIEWS.json] [--previews CACHE_DIRECTORY]');
  let input = inputSchema.parse(JSON.parse(await readFile(resolve(inputPath), 'utf8')));
  for (const path of reviewPaths) {
    const exported = z.strictObject({ reviews: z.array(z.unknown()) }).parse(JSON.parse(await readFile(resolve(path), 'utf8')));
    input = inputSchema.parse({ ...input, reviews: [...input.reviews, ...exported.reviews] });
  }
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
    'review-template.json': { instructions: '使用本批次页面导出，保留每条记录的 packetId。填写 reviewerKind（human / simulated_ai）和 reviewerModel（真人 null，模拟须填模型），以及 reviewerId、五维 1–5 分和可核验理由。AI 模拟单独汇总，不计作者/独立真人门槛。组织者用原 input.snapshot.json / private-key.json 加 --reviews 导入。不要把 private-key 发给评委。', reviews: [] },
  })) await writeFile(join(directory, name), `${JSON.stringify(data, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  await writeFile(join(directory, 'review.html'), html, { flag: 'wx', mode: 0o600 });
  if (readyHtml) await writeFile(join(directory, 'review-ready.html'), readyHtml, { flag: 'wx', mode: 0o600 });
  process.stdout.write(`${JSON.stringify({ status: result.report.status, issueCount: result.report.issues.length, reviewSources: result.report.reviewSources, output: directory })}\n`);
  if (result.report.status === 'partial') process.exitCode = 2;
}
main().catch(error => {
  // 不回显输入内容（可能含私人目标或评语）。
  process.stderr.write(`${error instanceof Error && error.message.startsWith('Usage:') ? error.message : '评测失败：请检查输入格式、目标目录是否已存在及文件权限。'}\n`);
  process.exitCode = 1;
});
