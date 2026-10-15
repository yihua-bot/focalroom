import { parseCommand, type CommandEnvelope, type CommandReply, type ErrorCode } from './schema.ts';

export type ProjectAuthority = {
  workspace_id: string; project_id: string; mode: 'LOCAL' | 'SHARED';
  local_state: 'LOCAL' | 'PROMOTING' | 'SHARED_CACHE'; revision: number;
};
// 仅由可信桌面入口/session/task 颁发，不从命令 JSON 反序列化。
export type Principal = { id: string; workspace_id: string; project_id: string } & (
  | { type: 'user'; role: 'OWNER' | 'ADMIN' | 'EDITOR' | 'REVIEWER' | 'VIEWER'; can_approve: boolean; can_edit_formal: boolean; can_propose: boolean }
  | { type: 'agent'; expires_at_ms: number }
);
export type AuthorityTarget = 'LOCAL' | 'CLOUD';

function denial(command: CommandEnvelope, principal: Principal, project: ProjectAuthority, target: AuthorityTarget, now: number): ErrorCode | null {
  if (command.workspace_id !== project.workspace_id || command.project_id !== project.project_id
    || principal.workspace_id !== project.workspace_id || principal.project_id !== project.project_id) return 'PERMISSION_DENIED';
  if (principal.type === 'agent') {
    if (principal.expires_at_ms <= now) return 'AUTH_REVOKED';
    if (command.kind !== 'proposal.create') return 'PERMISSION_DENIED';
  } else {
    const owner = principal.role === 'OWNER' || principal.role === 'ADMIN';
    if (principal.role === 'VIEWER') return 'PERMISSION_DENIED';
    if (command.kind === 'proposal.apply') {
      if (!owner && !(principal.role === 'EDITOR' && principal.can_approve)) return 'PERMISSION_DENIED';
    } else if (command.kind.startsWith('photo.') || command.kind.startsWith('sequence.')) {
      if (!owner && !(principal.role === 'EDITOR' && principal.can_edit_formal)) return 'PERMISSION_DENIED';
    } else if (command.kind.startsWith('proposal.') && !owner && !principal.can_propose) return 'PERMISSION_DENIED';
  }
  if ((target === 'LOCAL' && (project.mode !== 'LOCAL' || project.local_state !== 'LOCAL'))
    || (target === 'CLOUD' && project.mode !== 'SHARED')) return 'AUTHORITY_MISMATCH';
  if (command.expected_revision !== project.revision) return 'REVISION_CONFLICT';
  return null;
}

export function dispatch(input: unknown, principal: Principal, project: ProjectAuthority, target: AuthorityTarget, now = Date.now()): CommandReply {
  let command: CommandEnvelope;
  try { command = parseCommand(input); }
  catch { return { ok: false, command_id: '', error: { code: 'VALIDATION_ERROR', message: '命令不符合 v2.0 契约' } }; }
  const code = denial(command, principal, project, target, now) ?? 'NOT_IMPLEMENTED';
  return { ok: false, command_id: command.command_id, error: { code, message: code === 'NOT_IMPLEMENTED' ? 'v2.0 写入入口尚未开放' : '命令未获执行，请检查权限、项目模式或版本' } };
}
