import { dispatch, type Principal, type ProjectAuthority } from '@betweenframes/contracts';

// M0 skeleton：尚无 HTTP/session/Postgres；调用时必须传入可信入口颁发的身份。
export function dispatchSharedCommand(input: unknown, principal: Principal, project: ProjectAuthority) {
  return dispatch(input, principal, project, 'CLOUD');
}
export const apiStatus = { implemented: false, milestone: 'M4', message: '云共享服务尚未开放' } as const;
