// 仅展示受支持的 Google 头像来源；无效地址使用本地占位。
export function safeAvatarUrl(value: unknown): string | null {
  if (typeof value !== "string" || value.length > 2048) return null;
  try {
    const url = new URL(value);
    return url.protocol === "https:" && /^lh[3-6]\.googleusercontent\.com$/.test(url.hostname)
      && !url.username && !url.password && !url.port && !url.hash ? url.href : null;
  } catch { return null; }
}
