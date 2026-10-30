#!/usr/bin/env node
// K2 摄影师知识采集：Wikipedia 分类 + 摘要 API → SQL 种子
// 来源：Wikipedia (CC BY-SA)，通过分类获取、摘要补充、批量输出
// 用法：node ingest-photographers.mjs > photographers.sql

const CATEGORIES = [
  "Category:20th-century American photographers",
  "Category:American street photographers",
  "Category:French photographers",
  "Category:Japanese photographers",
  "Category:German photographers",
  "Category:British photographers",
  "Category:21st-century photographers",
  "Category:Chinese photographers",
  "Category:Landscape photographers",
  "Category:Documentary photographers",
  "Category:Fine art photographers",
  "Category:War photographers",
  "Category:Fashion photographers",
];

const esc = (s) => String(s ?? "").replace(/'/g, "''").replace(/\\/g, "\\\\");

async function fetchCategoryMembers(category) {
  const url = `https://en.wikipedia.org/w/api.php?action=query&list=categorymembers&cmtitle=${encodeURIComponent(category)}&cmlimit=500&cmtype=subcat|page&format=json`;
  const res = await fetch(url, { headers: { "User-Agent": "FocalRoom-K2/1.0" }, signal: AbortSignal.timeout(15000) });
  if (!res.ok) return [];
  const data = await res.json();
  return data.query?.categorymembers?.filter((m) => m.ns === 0).map((m) => m.title) || [];
}

async function fetchSummary(title, retries = 2) {
  for (let attempt = 0; attempt <= retries; attempt++) {
    try {
      const url = `https://en.wikipedia.org/api/rest_v1/page/summary/${encodeURIComponent(title.replace(/ /g, "_"))}`;
      const res = await fetch(url, { headers: { "User-Agent": "FocalRoom-K2/1.0" }, signal: AbortSignal.timeout(10000) });
      if (res.status === 429) { await new Promise(r => setTimeout(r, 1000 * (attempt + 1))); continue; }
      if (!res.ok) return null;
      const d = await res.json();
      if (d.type !== "standard") return null;
      return { extract: d.extract?.slice(0, 400) || "", description: d.description || "" };
    } catch { if (attempt < retries) { await new Promise(r => setTimeout(r, 500)); continue; } return null; }
  }
  return null;
}

async function main() {
  const seen = new Set();

  // 并发拉取所有分类
  console.error("📥 拉取 Wikipedia 摄影师分类…");
  const membersList = await Promise.all(CATEGORIES.map((c) => fetchCategoryMembers(c, 100)));
  for (const members of membersList) {
    for (const title of members) {
      if (!seen.has(title) && !title.startsWith("Category:") && !title.startsWith("List of") && !title.includes("(disambiguation)")) {
        seen.add(title);
      }
    }
  }
  console.error(`✅ 去重后 ${seen.size} 位摄影师`);

  // 并发获取摘要
  const titles = Array.from(seen);
  const results = [];
  const BATCH = 10;
  for (let i = 0; i < titles.length; i += BATCH) {
    const batch = titles.slice(i, i + BATCH);
    const summaries = await Promise.all(batch.map((t) => fetchSummary(t)));
    batch.forEach((t, j) => {
      if (summaries[j] && summaries[j].extract) {
        results.push({ name: t, ...summaries[j] });
      }
    });
    if ((i / BATCH) % 4 === 0) console.error(`  ${Math.min(i + BATCH, titles.length)}/${titles.length}…`);
  }
  console.error(`✅ ${results.length} 位确认为摄影师（有 Wikipedia 摘要）`);

  // 生成 SQL
  const lines = [
    "-- K2 Wikipedia 摄影师知识采集",
    `-- 生成：${new Date().toISOString()}`,
    `-- 来源：Wikipedia API (CC BY-SA)`,
    `-- 共 ${results.length} 位`,
    "-- 置信：imported（自动采集，待专家校审）",
    "", "BEGIN;", "",
  ];
  for (const p of results) {
    lines.push(
      `INSERT INTO fr_photographers(name_en, biography, review_status)`,
      `VALUES('${esc(p.name)}', '${esc(p.extract)}', 'imported')`,
      `ON CONFLICT (name_en) DO NOTHING;`,
      "",
    );
  }
  lines.push("COMMIT;");
  process.stdout.write(lines.join("\n"));
}

main().catch((e) => { console.error("❌", e.message); process.exit(1); });
