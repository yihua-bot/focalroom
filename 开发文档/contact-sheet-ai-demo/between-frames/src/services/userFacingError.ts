/** 保留可操作的业务提示，底层实现信息只留在错误对象中。 */
export function userFacingError(error: unknown, fallback = '操作未完成，请重试。'): string {
  const value = error as { code?: string; message?: unknown; name?: string } | null;
  const byCode: Record<string, string> = {
    DATABASE: '无法保存更改，请重试。', INTERNAL: '服务暂时不可用，请稍后重试。',
    CLOUD_PROTOCOL: '暂时无法连接，请稍后重试。', CLOUD_CONFIG: '服务暂时不可用，请稍后重试。',
    CONFIG_INVALID: '服务暂时不可用，请稍后重试。', AUTH_REQUIRED: '请先登录。', UNAUTHORIZED: '请先登录。',
    KEYCHAIN_ERROR: '无法读取登录信息，请重新登录。', NOT_IMPLEMENTED: '此功能暂不可用。',
  };
  if (value?.code && byCode[value.code]) return byCode[value.code];
  const message = typeof value?.message === 'string' ? value.message : '';
  if (!message || value?.name === 'ZodError' || /\b(sqlite|sqlx|postgresql|schema|json|stack|panic|fixture|mock|provider|api key|base url|HTTP\s*\d{3})\b|CHECK constraint|FOREIGN KEY|原生验收|测试素材|验证离线导入|幂等|契约|反序列化|事务回滚/i.test(message)) return fallback;
  return message;
}
