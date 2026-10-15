import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import type { DiagnosticsRepository } from '../domain/diagnostics';

const count = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
const usage = z.object({ bytes: count, files: count, skippedEntries: count, complete: z.boolean() }).refine(v => v.complete === (v.skippedEntries === 0), '统计完整状态不一致');
const counts = z.object({ projects: count, photos: count, decisions: count, agentTasks: count, drafts: count, toolCalls: count, observations: count, evidence: count, hypotheses: count });
const schema = z.object({
  schemaVersion: z.literal(1), collectedAt: z.iso.datetime({ offset: true }), appVersion: z.string(),
  provider: z.object({ providerId: z.enum(['mock', 'openai']), ready: z.boolean(), keySource: z.enum(['env', 'keychain']).nullable() }),
  database: counts, failedAgentTasks: count, migrationsApplied: count,
  storage: z.object({ database: usage, wal: usage, sharedMemory: usage, cache: usage, backups: usage, totalBytes: count, complete: z.boolean(), cacheLimitBytes: count, scanLimitEntries: count }),
  notice: z.string(),
}).refine(value => {
  const s = value.storage;
  const sections = [s.database, s.wal, s.sharedMemory, s.cache, s.backups];
  return s.totalBytes === sections.reduce((sum, part) => sum + part.bytes, 0) && s.complete === sections.every(part => part.complete);
}, '统计合计或完整状态不一致');

export class TauriDiagnosticsRepository implements DiagnosticsRepository {
  async query() {
    let raw: unknown;
    try { raw = await invoke('get_diagnostics'); }
    catch { throw new Error('诊断读取失败，请重试；项目数据未修改。'); }
    // 白名单解析同时过滤多余字段，复制内容仅来自已校验的投影。
    return schema.parse(raw);
  }
}
