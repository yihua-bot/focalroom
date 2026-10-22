import { expect, it } from 'vitest';
import { canvasRequestSchema } from './canvas';
import examples from '../../../contracts/canvas-command-examples.json';

it('15 类画布命令与 Rust 共用同一组严格协议样例', () => {
  expect(examples).toHaveLength(15);
  for (const example of examples) {
    expect(canvasRequestSchema.parse(example)).toEqual(example);
    expect(canvasRequestSchema.safeParse({ ...example, actor: 'human' }).success).toBe(false);
    const duplicate = { ...example, operations: [example.operations[0], example.operations[0]] };
    expect(canvasRequestSchema.safeParse(duplicate).success).toBe(false);
  }
});
