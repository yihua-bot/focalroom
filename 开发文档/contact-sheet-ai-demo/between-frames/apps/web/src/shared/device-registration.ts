import { z } from 'zod';
import { request, type Session } from '../shared-api';

// 非机密安装标识，与账号分离；不使用浏览器指纹，也不在这里保存会话。
export async function registerWebDevice(session: Session, current: () => boolean = () => true) {
  if (!current()) return;
  const keyName = 'focalroom.web.device-key';
  let key = localStorage.getItem(keyName);
  if (!key || !z.uuid().safeParse(key).success) {
    key = crypto.randomUUID();
    localStorage.setItem(keyName, key);
  }
  const device = z.object({ id: z.uuid() }).parse(await request('/devices', session, 'POST', {
    device_key: key, name: 'Web 浏览器', platform: 'web',
  }));
  if (!current()) return;
  await request(`/devices/${device.id}/session`, session, 'POST', {});
}
