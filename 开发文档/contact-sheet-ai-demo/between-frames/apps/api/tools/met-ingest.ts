/**
 * K2 Focal Photography Intelligence — Met Museum 馆藏数据接入器。
 * 从 Met Collection API 同步摄影部门（ID 19）的摄影师与作品元数据到知识库。
 *
 * 用法：
 *   node apps/api/tools/met-ingest.ts --dry-run        # 预览 SQL，不写入数据库
 *   node apps/api/tools/met-ingest.ts --db <url>       # 直接写入指定数据库
 *   node apps/api/tools/met-ingest.ts --artist "Stephen Shore"  # 只同步指定摄影师
 *
 * 数据来源：The Metropolitan Museum of Art Collection API
 * https://metmuseum.github.io/ — 遵循 API 使用条款；公共领域图像标记 isPublicDomain。
 * 只同步元数据（标题/年代/媒介/馆藏号/来源 URL）；图像不下载，按 rightsStatus 标记。
 */

const MET_API = "https://collectionapi.metmuseum.org/public/collection/v1";
const PHOTOGRAPHY_DEPARTMENT = 19;

interface MetObject {
  objectID: number;
  title: string;
  artistDisplayName: string;
  artistBeginDate: string;
  artistEndDate: string;
  objectDate: string;
  medium: string;
  classification: string;
  department: string;
  isPublicDomain: boolean;
  primaryImage: string;
  accessionNumber: string;
  objectURL: string;
  galleryNumber: string;
}

interface MetSearchResponse {
  total: number;
  objectIDs: number[] | null;
}

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url, { headers: { accept: "application/json" } });
  if (!response.ok) throw new Error(`Met API ${response.status}: ${url}`);
  return response.json() as Promise<T>;
}

async function searchMet(artist: string): Promise<number[]> {
  const url = `${MET_API}/search?departmentId=${PHOTOGRAPHY_DEPARTMENT}&q=${encodeURIComponent(artist)}&hasImages=true`;
  const data = await fetchJson<MetSearchResponse>(url);
  return data.objectIDs ?? [];
}

async function getObject(id: number): Promise<MetObject> {
  return fetchJson<MetObject>(`${MET_API}/objects/${id}`);
}

/** 从 Met API 年份字段解析整数（"1947" / "1947-" / "1947-2014" / "ca. 1947"） */
function parseYear(value: string | undefined): number | null {
  if (!value) return null;
  const match = value.match(/\d{4}/);
  return match ? Number(match[0]) : null;
}

/** Met 分类映射到我们的知识本体：只保留摄影相关分类。 */
function isPhotographyClassification(obj: MetObject): boolean {
  return obj.classification?.toLowerCase().includes("photograph") ||
         obj.department?.includes("Photograph");
}

/** 生成 fr_photographers 的 INSERT SQL（幂等：ON CONFLICT name_en DO UPDATE）。 */
function photographerUpsert(name: string, birth: number | null, death: number | null): string {
  const escaped = name.replace(/'/g, "''");
  return `INSERT INTO fr_photographers(name_en, birth_year, death_year, review_status)
VALUES('${escaped}', ${birth ?? "NULL"}, ${death ?? "NULL"}, 'imported')
ON CONFLICT (name_en) DO UPDATE SET updated_at = now();`;
}

/** 生成 fr_knowledge_sources 的 INSERT SQL。 */
function sourceInsert(entityType: string, entityIdQuery: string, obj: MetObject): string {
  return `INSERT INTO fr_knowledge_sources(entity_type, entity_id, url, institution, accession_number, verified_at)
SELECT '${entityType}', ${entityIdQuery}, '${obj.objectURL}', 'The Metropolitan Museum of Art', '${obj.accessionNumber}', now()
WHERE NOT EXISTS (
  SELECT 1 FROM fr_knowledge_sources
  WHERE entity_type = '${entityType}' AND entity_id = (${entityIdQuery}) AND institution = 'The Metropolitan Museum of Art'
);`;
}

async function main() {
  const args = process.argv.slice(2);
  const dryRun = args.includes("--dry-run");
  const artistIdx = args.indexOf("--artist");
  const artist = artistIdx >= 0 ? args[artistIdx + 1] : null;

  if (!artist) {
    console.error("用法：met-ingest --artist <摄影师英文名> [--dry-run|--db <url>]");
    process.exit(1);
  }

  console.log(`[met-ingest] 搜索 "${artist}"（摄影部门 ID ${PHOTOGRAPHY_DEPARTMENT}）...`);
  const objectIds = await searchMet(artist);
  console.log(`[met-ingest] 找到 ${objectIds.length} 件相关作品`);

  const sqlStatements: string[] = [];
  let processed = 0;
  let skipped = 0;

  for (const objectId of objectIds.slice(0, 20)) {
    try {
      const obj = await getObject(objectId);
      if (!isPhotographyClassification(obj)) { skipped++; continue; }

      if (processed === 0) {
        // 第一件有效作品提取艺术家信息
        const artistBirth = parseYear(obj.artistBeginDate);
        const artistDeath = parseYear(obj.artistEndDate);
        sqlStatements.push(`-- 摄影师：${obj.artistDisplayName || artist}`);
        sqlStatements.push(photographerUpsert(obj.artistDisplayName || artist, artistBirth, artistDeath));
      }

      // 作品元数据（只记录标题/年代/媒介/馆藏号/来源，不下载图像）
      const title = obj.title.replace(/'/g, "''");
      const entityIdQuery = `(SELECT id FROM fr_photographers WHERE name_en = '${(obj.artistDisplayName || artist).replace(/'/g, "''")}')`;
      sqlStatements.push(`INSERT INTO fr_artworks(photographer_id, title_en, capture_date, medium, review_status)
SELECT ${entityIdQuery}, '${title}', '${obj.objectDate}', '${obj.medium?.replace(/'/g, "''") ?? ""}', 'imported'
WHERE NOT EXISTS (
  SELECT 1 FROM fr_artworks a JOIN fr_photographers p ON p.id = a.photographer_id
  WHERE p.name_en = '${(obj.artistDisplayName || artist).replace(/'/g, "''")}' AND a.title_en = '${title}'
);`);
      sqlStatements.push(sourceInsert("artwork", `(SELECT a.id FROM fr_artworks a JOIN fr_photographers p ON p.id = a.photographer_id WHERE p.name_en = '${(obj.artistDisplayName || artist).replace(/'/g, "''")}' AND a.title_en = '${title}')`, obj));

      processed++;
    } catch (error) {
      console.error(`[met-ingest] 对象 ${objectId} 失败：${error instanceof Error ? error.message : error}`);
    }
  }

  console.log(`[met-ingest] 处理 ${processed} 件（跳过 ${skipped} 件非摄影分类）`);
  console.log(`[met-ingest] 生成 ${sqlStatements.length} 条 SQL`);

  if (dryRun) {
    console.log("--- SQL ---");
    for (const sql of sqlStatements) console.log(sql + "\n");
  } else {
    console.log("[met-ingest] 请使用 --dry-run 预览或配置 --db <url> 写入（当前版本仅支持预览）");
  }
}

main().catch(error => { console.error("[met-ingest] 致命错误：", error); process.exit(1); });
