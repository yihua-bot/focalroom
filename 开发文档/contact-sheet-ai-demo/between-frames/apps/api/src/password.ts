import { randomBytes, scrypt } from "node:crypto";

// 注册、登录与密码恢复共用同一格式和参数，避免恢复后无法登录。
export const derivePassword = (password: string, salt: string) =>
  new Promise<Buffer>((resolve, reject) =>
    scrypt(
      password,
      salt,
      32,
      { N: 32768, r: 8, p: 1, maxmem: 67108864 },
      (error, key) => (error ? reject(error) : resolve(key)),
    ),
  );

export async function hashPassword(password: string) {
  const salt = randomBytes(16).toString("hex");
  return `scrypt-v1$${salt}$${(await derivePassword(password, salt)).toString("hex")}`;
}

export const validPassword = (password: string) =>
  password.length >= 8 && Buffer.byteLength(password) <= 1024;
