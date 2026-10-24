import { resolve } from 'node:path';

// 回归默认写测试产物；固定验收目录只在显式指定时使用，避免重跑覆盖历史证据。
export const evidenceDirectory = (suite: string) => resolve(process.env.FOCALROOM_TEST_EVIDENCE_DIRECTORY ?? 'test-results/editorial-evidence', suite);
