import { describe, expect, it } from 'vitest';
import fixtures from '../fixtures/commands.json';
import permissions from '../fixtures/permissions.json';
import { commandSchema, parseCommand } from './schema';
import { dispatch, type Principal, type ProjectAuthority } from './policy';

const project: ProjectAuthority = { workspace_id: 'w1', project_id: 'p1', mode: 'LOCAL', local_state: 'LOCAL', revision: 3 };
const owner: Principal = { type: 'user', id: 'u1', workspace_id: 'w1', project_id: 'p1', role: 'OWNER', can_approve: true, can_edit_formal: true, can_propose: true };

describe('v2 命令与权限基座', () => {
  for (const fixture of permissions) {
    it(`TS/Rust 共同权限样例：${fixture.name}`, () => {
      expect(dispatch(fixture.command, fixture.principal as Principal, fixture.project as ProjectAuthority, fixture.target as 'LOCAL' | 'CLOUD', fixture.now))
        .toMatchObject({ ok: false, error: { code: fixture.code } });
    });
  }
  for (const fixture of fixtures) {
    it(fixture.name, () => {
      if (fixture.valid) expect(() => parseCommand(fixture.command)).not.toThrow();
      else expect(() => parseCommand(fixture.command)).toThrow();
    });
  }
  it('所有命令的空 payload 均拒绝', () => {
    for (const fixture of fixtures.filter(item => item.valid)) {
      expect(commandSchema.safeParse({ ...fixture.command, payload: {} }).success).toBe(false);
    }
  });
  it('合法请求明确返回未实现，绝不伪造写入成功', () => {
    const reply = dispatch(fixtures[0].command, owner, project, 'LOCAL', 100);
    expect(reply).toEqual({ ok: false, command_id: 'cmd_test_001', error: { code: 'NOT_IMPLEMENTED', message: 'v2.0 写入入口尚未开放' } });
  });
  it.each([
    ['工作区不匹配', { ...owner, workspace_id: 'w2' }, project, 'LOCAL', 'PERMISSION_DENIED'],
    ['项目不匹配', { ...owner, project_id: 'p2' }, project, 'LOCAL', 'PERMISSION_DENIED'],
    ['Viewer 不可写', { ...owner, role: 'VIEWER' }, project, 'LOCAL', 'PERMISSION_DENIED'],
    ['旧基线', owner, { ...project, revision: 4 }, 'LOCAL', 'REVISION_CONFLICT'],
    ['共享项目不回退本地', owner, { ...project, mode: 'SHARED', local_state: 'SHARED_CACHE' }, 'LOCAL', 'AUTHORITY_MISMATCH'],
    ['迁移中不可本地写', owner, { ...project, local_state: 'PROMOTING' }, 'LOCAL', 'AUTHORITY_MISMATCH'],
    ['本地项目不发云写', owner, project, 'CLOUD', 'AUTHORITY_MISMATCH'],
    ['过期 Agent', { type: 'agent', id: 'task1', workspace_id: 'w1', project_id: 'p1', expires_at_ms: 99 }, project, 'LOCAL', 'AUTH_REVOKED'],
  ] as const)('%s', (_label, principal, authority, target, code) => {
    expect(dispatch(fixtures[0].command, principal, authority, target, 100)).toMatchObject({ ok: false, error: { code } });
  });
  it('Reviewer 即使附带 approve 标记也不能批准正式版', () => {
    const command = fixtures.find(item => item.name === '有效审批')!.command;
    expect(dispatch(command, { ...owner, role: 'REVIEWER' }, project, 'LOCAL', 100)).toMatchObject({ error: { code: 'PERMISSION_DENIED' } });
  });
  it('Agent 只能提出草案，不能写正式选择', () => {
    const principal: Principal = { type: 'agent', id: 'task1', workspace_id: 'w1', project_id: 'p1', expires_at_ms: 200 };
    expect(dispatch(fixtures[0].command, principal, project, 'LOCAL', 100)).toMatchObject({ error: { code: 'PERMISSION_DENIED' } });
    const proposal = fixtures.find(item => item.name === '有效提案')!.command;
    expect(dispatch(proposal, principal, project, 'LOCAL', 100)).toMatchObject({ error: { code: 'NOT_IMPLEMENTED' } });
  });
});
