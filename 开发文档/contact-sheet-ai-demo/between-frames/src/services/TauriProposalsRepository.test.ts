import { beforeEach, expect, it, vi } from 'vitest';
import { commandSchema } from '@betweenframes/contracts';
const ipc = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke: ipc }));
import { TauriProposalsRepository } from './TauriProposalsRepository';
beforeEach(() => { ipc.mockReset(); });
it('提交与审批使用规范 wire 契约和当前项目版本', async () => {
  ipc.mockImplementation((_name, { envelope }) => ({ ok: true, command_id: envelope.command_id, error: null, outcome: { applied: 1, failed: 0, discarded: false } }));
  const repository = new TauriProposalsRepository();
  await repository.submit('project1', 12, 'proposal1', 1, 'submit01');
  expect(await repository.apply('project1', 12, 'proposal1', 2, ['op1'], 'nonce0001', 'apply001')).toEqual({ applied: 1, failed: 0, discarded: false });
  for (const [name, args] of ipc.mock.calls) {
    expect(name).toBe('execute_v2_command');
    expect(commandSchema.safeParse(args.envelope).success).toBe(true);
    expect(args.envelope.expected_revision).toBe(12);
    expect(args.principal).toBe('local-owner');
  }
});
it('空选驳回也是合法审批请求', async () => {
  ipc.mockImplementation((_name, { envelope }) => ({ ok: true, command_id: envelope.command_id, error: null, outcome: { applied: 0, failed: 0, discarded: true } }));
  expect((await new TauriProposalsRepository().apply('p1', 3, 'proposal1', 2, [], 'nonce0001', 'reject01')).discarded).toBe(true);
  expect(commandSchema.safeParse(ipc.mock.calls[0][1].envelope).success).toBe(true);
});
it('拒绝业务失败、身份错配和不完整的成功回执', async () => {
  const repository = new TauriProposalsRepository();
  ipc.mockResolvedValue({ ok: false, command_id: '', error: { code: 'VALIDATION_ERROR', message: '命令无效' } });
  await expect(repository.apply('p1', 3, 'proposal1', 2, ['op1'], 'nonce0001', 'apply001')).rejects.toMatchObject({ code: 'VALIDATION_ERROR', message: '命令无效' });
  ipc.mockResolvedValue({ ok: true, command_id: 'wrong', error: null });
  await expect(repository.submit('p1', 3, 'proposal1', 1, 'submit01')).rejects.toThrow('身份不匹配');
  ipc.mockResolvedValue({ ok: true, command_id: 'cmd-apply001', error: null });
  await expect(repository.apply('p1', 3, 'proposal1', 2, ['op1'], 'nonce0001', 'apply001')).rejects.toThrow();
});
