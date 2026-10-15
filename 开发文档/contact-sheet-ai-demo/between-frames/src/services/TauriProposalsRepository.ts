import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';
import { parseCommand, type CommandEnvelope } from '@betweenframes/contracts';

const operation = z.object({
  operationId: z.string(), photoId: z.string(), op: z.enum(['add', 'move', 'remove']),
  toIndex: z.number().int().nullable(), reason: z.string(),
});
const proposal = z.object({
  id: z.string(), taskId: z.string().nullable(), author: z.string(), title: z.string(),
  state: z.enum(['draft', 'submitted', 'applied', 'discarded']),
  baseRevision: z.number().int(), revision: z.number().int(), approvalNonce: z.string(),
  operations: z.array(operation), createdAt: z.string(), updatedAt: z.string(),
});
const listing = z.object({ proposals: z.array(proposal) });

export interface ProposalItem { id: string; title: string; state: string; author: string; revision: number; approvalNonce: string; operations: { operationId: string; photoId: string; op: string; toIndex: number | null; reason: string }[] }

export class TauriProposalsRepository {
  async query(projectId: string): Promise<ProposalItem[]> {
    return listing.parse(await invoke('get_proposals', { projectId })).proposals;
  }
  async submit(projectId: string, projectRevision: number, proposalId: string, expectedRevision: number, requestId: string) {
    return this.execute({ version: '2.0', command_id: `cmd-${requestId}`, workspace_id: 'local-workspace', project_id: projectId, expected_revision: projectRevision, idempotency_key: `idem-${requestId}`, kind: 'proposal.submit', payload: { proposal_id: proposalId, expected_proposal_revision: expectedRevision } });
  }
  async apply(projectId: string, projectRevision: number, proposalId: string, expectedRevision: number, accepted: string[], nonce: string, requestId: string) {
    const result = await this.execute({ version: '2.0', command_id: `cmd-${requestId}`, workspace_id: 'local-workspace', project_id: projectId, expected_revision: projectRevision, idempotency_key: `idem-${requestId}`, kind: 'proposal.apply', payload: { proposal_id: proposalId, expected_proposal_revision: expectedRevision, accepted_operation_ids: accepted, approval_nonce: nonce } });
    return z.object({ applied: z.number().int().nonnegative(), failed: z.literal(0), discarded: z.boolean() }).parse(result.outcome);
  }
  private async execute(input: CommandEnvelope) {
    const envelope = parseCommand(input);
    const result = z.object({ ok: z.boolean(), command_id: z.string(), error: z.object({ code: z.string(), message: z.string() }).nullable(), outcome: z.unknown().optional() }).parse(await invoke('execute_v2_command', { envelope, principal: 'local-owner' }));
    if (!result.ok) throw Object.assign(new Error(result.error?.message ?? '提案操作失败'), { code: result.error?.code });
    if (result.command_id !== envelope.command_id) throw new Error('提案回执身份不匹配');
    return result;
  }
}
