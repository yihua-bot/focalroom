import { beforeEach, expect, it, vi } from 'vitest';
import { registerWebDevice } from './device-registration';
import { request } from '../shared-api';
vi.mock('../shared-api', () => ({ request: vi.fn() }));
const session = { id: 'fixture', csrf: 'csrf', email: 'fixture@fr.test', name: 'Fixture' };
beforeEach(() => { vi.mocked(request).mockReset(); localStorage.clear(); });
it('同一浏览器重复登记复用设备键，并绑定当前会话', async () => {
  const id = crypto.randomUUID();
  vi.mocked(request).mockResolvedValue({ id, bound: true } as never);
  await registerWebDevice(session);
  const first = vi.mocked(request).mock.calls[0][3];
  await registerWebDevice(session);
  expect(vi.mocked(request).mock.calls[2][3]).toEqual(first);
  expect(request).toHaveBeenCalledWith(`/devices/${id}/session`, session, 'POST', {});
});
it('账号切换后不绑定旧设备，登记失败也不盲继续', async () => {
  const active = vi.fn().mockReturnValueOnce(true).mockReturnValueOnce(false);
  vi.mocked(request).mockResolvedValue({ id: crypto.randomUUID() } as never);
  await registerWebDevice(session, active);
  expect(request).toHaveBeenCalledTimes(1);
  vi.mocked(request).mockRejectedValueOnce(new Error('unavailable'));
  await expect(registerWebDevice(session)).rejects.toThrow('unavailable');
  expect(request).toHaveBeenCalledTimes(2);
});
