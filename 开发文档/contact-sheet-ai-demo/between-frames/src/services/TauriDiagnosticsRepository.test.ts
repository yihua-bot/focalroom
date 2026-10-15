import { beforeEach, expect, it, vi } from 'vitest';
import { diagnosticsFixture } from '../features/diagnostics/diagnostics.fixture';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc }));
import { TauriDiagnosticsRepository } from './TauriDiagnosticsRepository';
beforeEach(() => { ipc.mockReset(); });

it('只使用无路径参数命令，白名单过滤多余敏感字段', async () => {
  ipc.mockResolvedValue({ ...diagnosticsFixture(), originalPath: '/private/path', note: 'private', provider: { ...diagnosticsFixture().provider, apiKeyTail: 'secret' } });
  expect(await new TauriDiagnosticsRepository().query()).toEqual(diagnosticsFixture());
  expect(ipc).toHaveBeenCalledWith('get_diagnostics');
});
it('拒绝负数、错误合计、缺失状态与虚构的完整标记', async () => {
  const repo = new TauriDiagnosticsRepository();
  const bad = diagnosticsFixture(); bad.storage.totalBytes = -1;
  ipc.mockResolvedValueOnce(bad);
  await expect(repo.query()).rejects.toThrow();
  ipc.mockResolvedValueOnce({ ...diagnosticsFixture(), storage: { ...diagnosticsFixture().storage, totalBytes: 5 } });
  await expect(repo.query()).rejects.toThrow();
  ipc.mockResolvedValueOnce({ ...diagnosticsFixture(), storage: { ...diagnosticsFixture().storage, complete: false } });
  await expect(repo.query()).rejects.toThrow();
  const partial = diagnosticsFixture(); partial.storage.cache.skippedEntries = 1;
  ipc.mockResolvedValueOnce(partial);
  await expect(repo.query()).rejects.toThrow();
});
it('读取错误不将路径或 SQL 错误回显至界面', async () => {
  ipc.mockRejectedValue({ message: '/private/project.sqlite bad query', code: 'DATABASE' });
  await expect(new TauriDiagnosticsRepository().query()).rejects.toThrow('诊断读取失败');
  expect(ipc).toHaveBeenCalledTimes(1);
});
