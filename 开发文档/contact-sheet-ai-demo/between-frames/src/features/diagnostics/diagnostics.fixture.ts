import type { Diagnostics } from '../../domain/diagnostics';
// 仅测试使用，不含本地路径或业务文本。
const usage = (bytes: number, files = 1) => ({ bytes, files, skippedEntries: 0, complete: true });
export function diagnosticsFixture(): Diagnostics {
  return {
    schemaVersion: 1, collectedAt: '2026-10-15T02:00:00Z', appVersion: '0.1.0',
    provider: { providerId: 'mock', ready: false, keySource: null },
    database: { projects: 2, photos: 28, decisions: 28, agentTasks: 3, drafts: 0, toolCalls: 4, observations: 0, evidence: 6, hypotheses: 1 },
    failedAgentTasks: 1, migrationsApplied: 9,
    storage: { database: usage(4096), wal: usage(1024), sharedMemory: usage(1024), cache: usage(2048, 56), backups: usage(0, 0), totalBytes: 8192, complete: true, cacheLimitBytes: 2 * 1024 ** 3, scanLimitEntries: 100000 },
    notice: '仅包含统计；不会自动发送。',
  };
}
