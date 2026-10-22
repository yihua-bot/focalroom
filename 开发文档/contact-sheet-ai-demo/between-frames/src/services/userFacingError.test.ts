import { expect, it } from 'vitest';
import { userFacingError } from './userFacingError';
it('底层异常不能把数据库、接口或测试文字带到产品界面', () => {
  for (const message of ['SQLite CHECK constraint failed: projects', 'sqlx::error at /private/test.rs', 'JSON parse at offset 14', 'MockProvider fixture failed', '原生验收失败']) {
    expect(userFacingError(new Error(message))).toBe('操作未完成，请重试。');
  }
  expect(userFacingError({ code: 'DATABASE', message: 'private query' })).toBe('无法保存更改，请重试。');
  expect(userFacingError(new Error('原图已移动，请重新定位。'))).toBe('原图已移动，请重新定位。');
});
