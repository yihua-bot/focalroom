import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtemp, writeFile, symlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { S3Client } from '@aws-sdk/client-s3';
import sharp from 'sharp';
import { LocalPreviewStore, R2PreviewStore, sanitizePreview, readR2Config } from '../src/storage.ts';
const key = 'a'.repeat(64) + '.jpg';
test('M4 R2 adapter signs S3 put/get against an isolated local protocol server', async () => {
  let stored, signed = 0; const server = createServer(async (req, res) => { if (req.headers.authorization?.startsWith('AWS4-HMAC-SHA256 ')) signed++; assert.equal(req.url.split('?')[0], `/private-previews/${key}`); if (req.method === 'PUT') { const chunks = []; for await (const chunk of req) chunks.push(chunk); stored = Buffer.concat(chunks); res.writeHead(200, { ETag: '"test-etag"' }); res.end(); } else { res.writeHead(200, { 'Content-Type': 'image/jpeg', 'Content-Length': stored.length }); res.end(stored); } });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const client = new S3Client({ region: 'auto', endpoint: `http://127.0.0.1:${server.address().port}`, forcePathStyle: true, credentials: { accessKeyId: 'local-test-id', secretAccessKey: 'local-test-secret' }, requestChecksumCalculation: 'WHEN_REQUIRED', responseChecksumValidation: 'WHEN_REQUIRED' });
  try { const store = new R2PreviewStore(client, 'private-previews'); const bytes = await sharp({ create: { width: 24, height: 18, channels: 3, background: '#448bad' } }).jpeg().toBuffer(); await store.put(key, bytes); assert.deepEqual(await store.get(key), bytes); assert.equal(signed, 2); await assert.rejects(store.get('../private-file')); } finally { client.destroy(); await new Promise(resolve => server.close(resolve)); }
});
test('M4 local preview store rejects links, foreign object paths and content collisions', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'fr-preview-boundary-')); const store = new LocalPreviewStore(directory); await store.put(key, Buffer.from('first')); assert.equal((await store.get(key)).toString(), 'first'); await assert.rejects(store.put(key, Buffer.from('different'))); await assert.rejects(store.get('../escape.jpg'));
  const original = join(directory, 'original.txt'); await writeFile(original, 'not a preview'); const linkKey = 'b'.repeat(64) + '.jpg'; await symlink(original, join(directory, linkKey)); await assert.rejects(store.get(linkKey)); await assert.rejects(store.put(linkKey, Buffer.from('replacement')));
});
test('M4 media policy rejects oversized, PNG-disguised and corrupt sources', async () => {
  const image = sharp({ create: { width: 2500, height: 2, channels: 3, background: '#fff' } }); await assert.rejects(sanitizePreview(await image.jpeg().toBuffer())); await assert.rejects(sanitizePreview(await sharp({ create: { width: 20, height: 20, channels: 3, background: '#fff' } }).png().toBuffer())); await assert.rejects(sanitizePreview(Buffer.from('bad jpeg'))); await assert.rejects(sanitizePreview(Buffer.alloc(8388609)));
});
test('M4 R2 configuration requires explicit official endpoint and complete credentials', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'fr-r2-config-')); const file = join(directory, 'r2.json');
  await writeFile(file, JSON.stringify({ endpoint: 'https://example.com', bucket: 'test', accessKeyId: 'local', secretAccessKey: 'local' })); await assert.rejects(readR2Config(file));
  await writeFile(file, JSON.stringify({ endpoint: 'https://abcdef.r2.cloudflarestorage.com', bucket: 'test', accessKeyId: 'local', secretAccessKey: 'local' })); const store = await readR2Config(file); assert.ok(store instanceof R2PreviewStore); store.client.destroy();
});
