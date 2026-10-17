import 'reflect-metadata';
import { Controller, All, Req, Res, Inject, Module } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';
import { json, raw, type Request, type Response } from 'express';
import { Pool } from 'pg';
import { pathToFileURL } from 'node:url';
import { z, ZodError } from 'zod';
import { CloudService } from './service.ts';
import { ApiError, assert } from './errors.ts';
import { LocalPreviewStore, readR2Config, type PreviewStore } from './storage.ts';
import { attachRealtime } from './realtime.ts';
import { migrate } from './database.ts';

class ApiController {
  readonly service: CloudService;
  constructor(service: CloudService) { this.service = service; }
  async handle(req: Request, res: Response) {
    res.set({ 'Cache-Control': 'private, no-store', 'X-Content-Type-Options': 'nosniff', 'Referrer-Policy': 'no-referrer', Vary: 'Authorization, Cookie', 'Content-Security-Policy': "default-src 'none'; frame-ancestors 'none'" });
    try {
      const value = await this.route(req, res); if (!res.headersSent) res.json(value);
    } catch (error) {
      const typed = error instanceof ApiError ? error : error instanceof ZodError ? new ApiError(400, 'VALIDATION_ERROR', '请求不符合接口契约') : (error as { code?: string }).code === '23505' ? new ApiError(409, 'CONFLICT', '记录已存在') : new ApiError(500, 'INTERNAL', '服务暂时不可用');
      res.status(typed.status).json({ error: { code: typed.code, message: typed.message } });
    }
  }
  async route(req: Request, res: Response): Promise<unknown> {
    const { service: s } = this, headers = req.headers, path = req.path.replace(/^\/api\/v2/, '');
    s.auth.checkOrigin(headers);
    if (req.method === 'GET' && path === '/health') { await s.pool.query('SELECT 1'); return { ok: true, authority: 'POSTGRESQL', protocol: '2.0' }; }
    if (req.method === 'POST' && ['/auth/register', '/auth/login'].includes(path)) {
      const response = await s.auth.account(req.body, path.endsWith('register'), req.socket.remoteAddress ?? 'unknown'); res.setHeader('Set-Cookie', s.auth.cookie(response.token)); return response;
    }
    if (path === '/auth/me' && req.method === 'GET') { const u = await s.auth.authenticate(headers, false); return { id: u.id, email: u.email, name: u.name, csrf: u.csrf }; }
    if (path === '/auth/logout' && req.method === 'POST') { const u = await s.auth.authenticate(headers, true); await s.pool.query('DELETE FROM fr_sessions WHERE digest=$1', [u.session]); res.setHeader('Set-Cookie', s.auth.cookie('', true)); return { logged_out: true }; }
    if (path === '/workspaces' && req.method === 'GET') return s.workspaces(headers);
    if (path === '/agent/comments' && req.method === 'GET') return s.agentComments(headers);
    if (path === '/projects' && req.method === 'GET') return s.projects(headers);
    if (path === '/promotions' && req.method === 'POST') return s.promote(headers, req.body);
    if (path === '/invitations/accept' && req.method === 'POST') return s.acceptInvite(headers, req.body);
    const workspace = path.match(/^\/workspaces\/([^/]+)\/invitations(?:\/([^/]+))?$/);
    if (workspace) {
      if (req.method === 'POST' && !workspace[2]) return s.invite(headers, workspace[1], req.body);
      if (req.method === 'DELETE' && workspace[2]) return s.revokeInvite(headers, workspace[1], workspace[2]);
    }
    const media = path.match(/^\/media\/([a-f0-9]{64})$/);
    if (media && req.method === 'GET') { res.type('image/jpeg').send(await s.media(media[1])); return; }
    const project = path.match(/^\/projects\/([^/]+)\/(.+)$/);
    if (project) {
      const [, id, action] = project;
      if (action === 'snapshot' && req.method === 'GET') return s.snapshot(headers, id);
      if (action === 'ai-comments' && req.method === 'GET') return s.aiComments(headers, id);
      if (action === 'agent-scopes' && req.method === 'POST') return s.issueAgentScope(headers, id);
      if (action.startsWith('agent-scopes/') && req.method === 'DELETE') return s.revokeAgentScope(headers, id, action.slice(13));
      if (action === 'commands' && req.method === 'POST') return s.authority.command(headers, id, req.body);
      if (action === 'finalize' && req.method === 'POST') return s.finalize(headers, id, req.body);
      if (action === 'abort' && req.method === 'POST') return s.abort(headers, id, req.body);
      if (action === 'ws-ticket' && req.method === 'POST') return s.ticket(headers, id);
      if (action === 'history' && req.method === 'GET') return s.authority.scope(headers, id, false, async ({ tx }) => (await tx.query('SELECT revision,actor_id,command_id,created_at FROM fr_versions WHERE project_id=$1 ORDER BY revision DESC LIMIT 100', [id])).rows);
      if (action.startsWith('grants/')) {
        const target = action.slice(7); if (req.method === 'DELETE') return s.grants(headers, id, target); if (req.method === 'PATCH') return s.grants(headers, id, target, req.body);
      }
      const preview = action.match(/^previews\/([^/]+)(\/ticket)?$/);
      if (preview) {
        if (req.method === 'PUT' && !preview[2]) { assert(req.is('image/jpeg') && Buffer.isBuffer(req.body), 400, 'MEDIA_POLICY', '上传必须为 JPEG 派生预览'); return s.upload(headers, id, decodeURIComponent(preview[1]), req.body); }
        if (req.method === 'POST' && preview[2]) return s.ticket(headers, id, decodeURIComponent(preview[1]));
      }
      const reply = action.match(/^comments\/([^/]+)\/replies$/);
      if (reply && req.method === 'POST') return s.reply(headers, id, reply[1], req.body);
    }
    throw new ApiError(404, 'NOT_FOUND', '接口不存在');
  }
}
// 显式应用标准 Nest 元数据，兼容 Node 原生 TypeScript；不依赖运行时编译器。
Controller('api/v2')(ApiController);
Inject(CloudService)(ApiController, undefined, 0);
All(['/', '{*path}'])(ApiController.prototype, 'handle', Object.getOwnPropertyDescriptor(ApiController.prototype, 'handle')!);
Req()(ApiController.prototype, 'handle', 0); Res()(ApiController.prototype, 'handle', 1);
class ApiModule {}
export async function createApi(options: { pool: Pool; store: PreviewStore; origin: string }) {
  const service = new CloudService(options.pool, options.origin, options.store);
  Module({ controllers: [ApiController], providers: [{ provide: CloudService, useValue: service }] })(ApiModule);
  const app = await NestFactory.create(ApiModule, { logger: false, bodyParser: false });
  app.use(json({ limit: '2mb' })); app.use(raw({ type: 'image/jpeg', limit: '8mb' }));
  await app.init(); const closeRealtime = attachRealtime(app.getHttpServer(), service);
  return { app, service, close: async () => { await closeRealtime(); await app.close(); } };
}
export async function main() {
  const config = z.strictObject({ database_url: z.string().url(), origin: z.string().url(), preview_directory: z.string().optional(), r2_config: z.string().optional(), port: z.coerce.number().int().min(1).max(65535).default(3002), host: z.string().default('127.0.0.1') }).parse({ database_url: process.env.FOCALROOM_DATABASE_URL, origin: process.env.FOCALROOM_WEB_ORIGIN, preview_directory: process.env.FOCALROOM_PREVIEW_DIRECTORY, r2_config: process.env.FOCALROOM_R2_CONFIG, port: process.env.FOCALROOM_API_PORT, host: process.env.FOCALROOM_API_HOST });
  assert(new URL(config.origin).origin === config.origin, 400, 'CONFIG_INVALID', 'Web Origin 必须为精确来源');
  const pool = new Pool({ connectionString: config.database_url, max: 10 });
  if (process.argv.includes('--migrate')) { await migrate(pool); await pool.end(); return; }
  assert(!!config.preview_directory !== !!config.r2_config, 400, 'CONFIG_INVALID', '须显式选择本地私有目录或 R2 配置');
  const store = config.r2_config ? await readR2Config(config.r2_config) : new LocalPreviewStore(config.preview_directory!);
  const server = await createApi({ pool, store, origin: config.origin });
  await server.app.listen(config.port, config.host); process.stdout.write(`FocalRoom API listening ${config.host}:${config.port}\n`);
  const stop = async () => { await server.close(); await pool.end(); };
  process.once('SIGTERM', () => void stop()); process.once('SIGINT', () => void stop());
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
