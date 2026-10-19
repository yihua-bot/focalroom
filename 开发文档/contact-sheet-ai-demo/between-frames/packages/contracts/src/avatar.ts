// 头像来源白名单：Google 官方头像；桌面显式允许时支持已压缩的位图 data URL；SVG 一律拒绝。
export function safeAvatarUrl(value: unknown, allowEmbedded = false): string | null {
  if (typeof value !== "string" || value.length > 400_000) return null;
  if (allowEmbedded && (value.startsWith("data:image/jpeg;base64,") || value.startsWith("data:image/png;base64,"))) return value;
  try {
    const url = new URL(value);
    return url.protocol === "https:" && /^lh[3-6]\.googleusercontent\.com$/.test(url.hostname)
      && !url.username && !url.password && !url.port && !url.hash ? url.href : null;
  } catch { return null; }
}
