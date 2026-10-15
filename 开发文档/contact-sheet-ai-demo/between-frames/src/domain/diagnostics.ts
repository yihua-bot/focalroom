export interface FileUsage { bytes: number; files: number; skippedEntries: number; complete: boolean }
export interface Diagnostics {
  schemaVersion: 1; collectedAt: string; appVersion: string;
  provider: { providerId: string; ready: boolean; keySource: string | null };
  database: { projects: number; photos: number; decisions: number; agentTasks: number; drafts: number; toolCalls: number; observations: number; evidence: number; hypotheses: number };
  failedAgentTasks: number; migrationsApplied: number;
  storage: { database: FileUsage; wal: FileUsage; sharedMemory: FileUsage; cache: FileUsage; backups: FileUsage; totalBytes: number; complete: boolean; cacheLimitBytes: number; scanLimitEntries: number };
  notice: string;
}
export interface DiagnosticsRepository { query(): Promise<Diagnostics> }
