import type { DraftOperation } from "../shared-api";

// 仅推导草案成员，供按钮状态和追加位置使用；正式排序仍由服务端比较与命令校验。
export function draftSequenceMembers(
  items: readonly { photo_id: string }[],
  operations: readonly Pick<DraftOperation, "op" | "photo_id">[],
): Set<string> {
  const members = new Set(items.map(item => item.photo_id));
  for (const operation of operations) {
    if (operation.op === "add") members.add(operation.photo_id);
    else if (operation.op === "remove") members.delete(operation.photo_id);
  }
  return members;
}
