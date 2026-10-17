export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  constructor(status: number, code: string, message: string) {
    super(message);
    this.status = status;
    this.code = code;
  }
}
export function assert(
  condition: unknown,
  status: number,
  code: string,
  message: string,
): asserts condition {
  if (!condition) throw new ApiError(status, code, message);
}
export const denied = () =>
  new ApiError(403, "PERMISSION_DENIED", "未获得此操作的项目授权");
