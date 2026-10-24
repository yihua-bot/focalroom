import { expect, it } from 'vitest';
import { canvasReceiptSchema, canvasRequestSchema } from './canvas';
import examples from '../../../contracts/canvas-command-examples.json';

it('20 类画布命令与 Rust 共用同一组严格协议样例', () => {
  expect(examples).toHaveLength(20);
  for (const example of examples) {
    expect(canvasRequestSchema.parse(example)).toEqual(example);
    expect(canvasRequestSchema.safeParse({ ...example, actor: 'human' }).success).toBe(false);
    const duplicate = { ...example, operations: [example.operations[0], example.operations[0]] };
    expect(canvasRequestSchema.safeParse(duplicate).success).toBe(false);
  }
});

it('内置演示项目的稳定编号可用于全部画布操作和回执', () => {
  for (const example of examples) {
    const request = { ...example, projectId: 'demo-first-walk' };
    expect(canvasRequestSchema.parse(request)).toEqual(request);
    expect(canvasReceiptSchema.parse({ commandId: request.commandId, projectId: request.projectId,
      boardId: request.boardId, baseRevision: 0, revision: 1, changed: true, results: [] }).projectId).toBe(request.projectId);
  }
});

it('编号兼容仍拒绝空项目、超长项目及无效的命令和画布编号', () => {
  for (const invalid of [{ projectId: '' }, { projectId: 'p'.repeat(161) },
    { commandId: 'not-a-command' }, { boardId: 'not-a-board' }]) {
    expect(canvasRequestSchema.safeParse({ ...examples[0], ...invalid }).success).toBe(false);
  }
});
