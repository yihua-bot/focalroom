import { invoke } from '@tauri-apps/api/core';
import { z } from 'zod';

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
  async submit(projectId: string, proposalId: string, expectedRevision: number, requestId: string) {
    return invoke('execute_v2_command', { envelope: { version: '2.0', commandId: `cmd-${requestId}`, workspaceId: 'local-workspace', projectId, expectedRevision: 1, idempotencyKey: `idem-${requestId}`, kind: 'proposal.submit', payload: { proposal_id: proposalId, expected_proposal_revision: expectedRevision } }, principal: 'local-owner' });
  }
  async apply(projectId: string, proposalId: string, expectedRevision: number, accepted: string[], nonce: string, requestId: string) {
    return invoke('execute_v2_command', { envelope: { version: '2.0', commandId: `cmd-${requestId}`, workspaceId: 'local-workspace', projectId, expectedRevision: 1, idempotencyKey: `idem-${requestId}`, kind: 'proposal.apply', payload: { proposal_id: proposalId, expected_proposal_revision: expectedRevision, accepted_operation_ids: accepted, approval_nonce: nonce } }, principal: 'local-owner' });
  }
}
