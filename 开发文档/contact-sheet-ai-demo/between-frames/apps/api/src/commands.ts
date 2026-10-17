import { dispatch, type Principal, type ProjectAuthority } from '@betweenframes/contracts';

// 兼容旧契约的纯权限预检，不执行写入。实际 HTTP/session/Postgres 入口见 server.ts / AuthorityService.command。
export function dispatchSharedCommand(input: unknown, principal: Principal, project: ProjectAuthority) {
  return dispatch(input, principal, project, 'CLOUD');
}
export const apiStatus = { implemented: true, milestone: 'M4/M5', message: 'NestJS/PostgreSQL 独立服务已实现；生产部署须独立验收' } as const;
