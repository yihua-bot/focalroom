import { dispatch, type Principal, type ProjectAuthority } from '@betweenframes/contracts';

// M0 skeleton：尚无队列、模型请求或 task principal 颁发服务。
export function dispatchAgentProposal(input: unknown, principal: Extract<Principal, { type: 'agent' }>, project: ProjectAuthority) {
  return dispatch(input, principal, project, 'CLOUD');
}
export const workerStatus = { implemented: false, milestone: 'M2/M3', message: '云 Agent 尚未运行；既有本地 Runtime 保留' } as const;
