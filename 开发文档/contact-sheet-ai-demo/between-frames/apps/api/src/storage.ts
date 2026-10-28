import { mkdir, readFile, open, unlink } from "node:fs/promises";
import { constants } from "node:fs";
import { join } from "node:path";
import {
  S3Client,
  PutObjectCommand,
  GetObjectCommand,
  DeleteObjectCommand,
} from "@aws-sdk/client-s3";
import sharp from "sharp";
import { assert, ApiError } from "./errors.ts";
import { digest } from "./database.ts";
export interface PreviewStore {
  put(key: string, bytes: Buffer): Promise<void>;
  get(key: string): Promise<Buffer>;
  remove?(key: string): Promise<void>;
}
const validKey = (key: string) => /^[a-f0-9]{64}\.jpg$/.test(key);
export class LocalPreviewStore implements PreviewStore {
  readonly directory: string;
  constructor(directory: string) {
    this.directory = directory;
  }
  async remove(key: string) {
    assert(validKey(key), 400, 'VALIDATION_ERROR', '非法对象键');
    try { await unlink(join(this.directory, key)); }
    catch (error) { if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw error; }
  }
  async put(key: string, bytes: Buffer) {
    assert(validKey(key), 400, "VALIDATION_ERROR", "非法对象键");
    await mkdir(this.directory, { recursive: true, mode: 0o700 });
    // 内容寻址、独占创建；已有同 hash 文件也必须核对，不能覆盖链接。
    try {
      const file = await open(
        join(this.directory, key),
        constants.O_WRONLY |
          constants.O_CREAT |
          constants.O_EXCL |
          constants.O_NOFOLLOW,
        0o600,
      );
      try {
        await file.writeFile(bytes);
      } finally {
        await file.close();
      }
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "EEXIST") throw error;
      const existing = await this.get(key);
      assert(existing.equals(bytes), 409, "OBJECT_CONFLICT", "对象内容不一致");
    }
  }
  async get(key: string) {
    assert(validKey(key), 404, "NOT_FOUND", "预览不存在");
    const file = await open(
      join(this.directory, key),
      constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK,
    );
    try {
      const stat = await file.stat();
      assert(
        stat.isFile() && stat.size <= 8388608,
        404,
        "NOT_FOUND",
        "预览不可用",
      );
      return await file.readFile();
    } finally {
      await file.close();
    }
  }
}
export class R2PreviewStore implements PreviewStore {
  readonly client: S3Client;
  readonly bucket: string;
  constructor(client: S3Client, bucket: string) {
    this.client = client;
    this.bucket = bucket;
  }
  async remove(key: string) {
    assert(validKey(key), 400, 'VALIDATION_ERROR', '非法对象键');
    await this.client.send(new DeleteObjectCommand({ Bucket: this.bucket, Key: key }));
  }
  async put(key: string, bytes: Buffer) {
    assert(validKey(key), 400, "VALIDATION_ERROR", "非法对象键");
    await this.client.send(
      new PutObjectCommand({
        Bucket: this.bucket,
        Key: key,
        Body: bytes,
        ContentType: "image/jpeg",
        CacheControl: "private, no-store",
      }),
    );
  }
  async get(key: string) {
    assert(validKey(key), 404, "NOT_FOUND", "预览不存在");
    const object = await this.client.send(
      new GetObjectCommand({ Bucket: this.bucket, Key: key }),
    );
    assert(
      object.ContentLength && object.ContentLength <= 8388608 && object.Body,
      404,
      "NOT_FOUND",
      "预览不可用",
    );
    return Buffer.from(await object.Body.transformToByteArray());
  }
}
export async function sanitizePreview(input: Buffer) {
  try {
    return await decodePreview(input);
  } catch (error) {
    if (error instanceof ApiError) throw error;
    const invalid = new ApiError(
      400,
      "MEDIA_POLICY",
      "预览损坏或编码不受支持，请重新导出 JPEG 预览",
    );
    Object.defineProperty(invalid, "cause", { value: error });
    throw invalid;
  }
}
async function decodePreview(input: Buffer) {
  assert(
    input.length > 0 && input.length <= 8388608,
    400,
    "MEDIA_POLICY",
    "只接受 8 MiB 以内的派生预览",
  );
  const pipeline = sharp(input, {
    limitInputPixels: 2048 * 2048,
    failOn: "error",
  });
  const meta = await pipeline.metadata();
  assert(
    meta.format === "jpeg" &&
      meta.width &&
      meta.height &&
      Math.max(meta.width, meta.height) <= 2048 &&
      !meta.pages,
    400,
    "MEDIA_POLICY",
    "仅接受长边不超过 2048px 的单张 JPEG 预览",
  );
  // rotate 消化方向标记；默认不保留 EXIF/XMP/GPS。绝不上传原始输入字节。
  const { data, info } = await pipeline
    .rotate()
    .jpeg({ quality: 88 })
    .toBuffer({ resolveWithObject: true });
  const sha = digest(data);
  return {
    data,
    sha,
    width: info.width,
    height: info.height,
    key: `${sha}.jpg`,
  };
}
// readFile 只用于配置检查，避免 SDK 隐式读取其他项目的凭证文件。
export async function readR2Config(path: string) {
  const config = JSON.parse(await readFile(path, "utf8")) as Record<
    string,
    string
  >;
  assert(
    /^https:\/\/[a-f0-9]+\.r2\.cloudflarestorage\.com$/.test(
      config.endpoint ?? "",
    ) &&
      !!config.bucket &&
      !!config.accessKeyId &&
      !!config.secretAccessKey,
    400,
    "CONFIG_INVALID",
    "R2 配置不完整",
  );
  return new R2PreviewStore(
    new S3Client({
      region: "auto",
      endpoint: config.endpoint,
      credentials: {
        accessKeyId: config.accessKeyId,
        secretAccessKey: config.secretAccessKey,
      },
      followRegionRedirects: false,
    }),
    config.bucket,
  );
}
