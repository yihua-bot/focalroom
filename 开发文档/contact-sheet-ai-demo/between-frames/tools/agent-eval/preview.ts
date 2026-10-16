import { lstat, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import type { BlindPack } from './evaluation.ts';

// 离线分发包只读已派生 JPEG，不访问原图，不写入输入目录。
export async function embedPreviews(html: string, pack: BlindPack['publicPack'], directory: string) {
  const assets: Record<string, string> = {};
  let bytes = 0;
  for (const name of new Set(pack.cases.flatMap(c => c.photos.map(p => p.previewFilename)))) {
    if (!/^[a-f0-9]{64}-1400\.jpg$/.test(name)) throw new Error('Invalid preview filename');
    const path = join(directory, name);
    const stat = await lstat(path);
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 16 * 1024 * 1024) throw new Error('Invalid preview file');
    bytes += stat.size;
    if (bytes > 128 * 1024 * 1024) throw new Error('Preview package too large');
    const data = await readFile(path);
    if (data[0] !== 0xff || data[1] !== 0xd8 || data[2] !== 0xff) throw new Error('Preview must be JPEG');
    assets[name] = `data:image/jpeg;base64,${data.toString('base64')}`;
  }
  const payload = JSON.stringify({ pack, assets }).replace(/</g, '\\u003c');
  return html.replace('<script>', `<script id="embedded-data" type="application/json">${payload}</script>\n<script>`);
}
