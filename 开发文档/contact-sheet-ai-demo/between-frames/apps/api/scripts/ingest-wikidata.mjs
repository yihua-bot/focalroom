#!/usr/bin/env node
// K2 自动采集：Wikidata 摄影师 → Wikipedia 摘要 → 生产 SQL
// 用法：node ingest-wikidata.mjs [--dry-run]
// 数据来源：Wikidata SPARQL (CC0) + Wikipedia REST (CC BY-SA)

const SPARQL = `
SELECT ?p ?label ?birth ?death ?nationality ?sitelinks WHERE {
  ?p wdt:P106 wd:Q33231;
     rdfs:label ?label;
     wikibase:sitelinks ?sitelinks;
     wdt:P569 ?birth.
  FILTER(LANG(?label) = "en")
  FILTER(?sitelinks > 8)
  OPTIONAL { ?p wdt:P570 ?death }
  OPTIONAL { ?p wdt:P27 ?nc. ?nc rdfs:label ?nationality. FILTER(LANG(?nationality) = "en") }
} ORDER BY DESC(?sitelinks) LIMIT 300`;

const esc = (s) => String(s ?? "").replace(/'/g, "''").replace(/\\/g, "\\\\");

async function fetchWikidata() {
  const url = "https://query.wikidata.org/sparql?format=json&query=" + encodeURIComponent(SPARQL);
  const res = await fetch(url, {
    headers: { "Accept": "application/json", "User-Agent": "FocalRoom-Knowledge-Ingest/1.0" },
    signal: AbortSignal.timeout(30000),
  });
  if (!res.ok) throw new Error(`Wikidata ${res.status}`);
  const data = await res.json();
  return data.results.bindings.map((b) => ({
    name: b.label.value,
    birthYear: b.birth?.value?.slice(0, 4) || null,
    deathYear: b.death?.value?.slice(0, 4) || null,
    nationality: b.nationality?.value || null,
    sitelinks: Number(b.sitelinks?.value || 0),
  }));
}

async function fetchWikipediaSummary(name) {
  try {
    const url = `https://en.wikipedia.org/api/rest_v1/page/summary/${encodeURIComponent(name.replace(/ /g, "_"))}`;
    const res = await fetch(url, {
      headers: { "User-Agent": "FocalRoom-Knowledge-Ingest/1.0" },
      signal: AbortSignal.timeout(8000),
    });
    if (!res.ok) return null;
    const data = await res.json();
    return data.extract?.slice(0, 500) || null;
  } catch { return null; }
}

async function main() {
  console.error("📥 查询 Wikidata（sitelinks > 8 的摄影师）…");
  const photographers = await fetchWikidata();
  console.error(`✅ 获取 ${photographers.length} 位摄影师`);

  // 并发获取 Wikipedia 摘要（每批 20）
  const results = [];
  for (let i = 0; i < photographers.length; i += 20) {
    const batch = photographers.slice(i, i + 20);
    const summaries = await Promise.all(batch.map((p) => fetchWikipediaSummary(p.name)));
    batch.forEach((p, j) => {
      if (summaries[j]) results.push({ ...p, biography: summaries[j] });
      else results.push(p);
    });
    if ((i / 20) % 5 === 0) console.error(`  ${i + batch.length}/${photographers.length}…`);
  }
  console.error(`✅ ${results.filter((r) => r.biography).length} 位有 Wikipedia 摘要`);

  // 生成 SQL
  const sql = ["-- K2 自动采集：Wikidata + Wikipedia 摄影师数据",
    `-- 生成时间：${new Date().toISOString()}`,
    `-- 来源：Wikidata SPARQL (CC0) + Wikipedia REST (CC BY-SA)`,
    `-- 共 ${results.length} 位摄影师`,
    "-- 置信标注：imported = 自动采集，待专家校审",
    "", "BEGIN;", ""];

  for (const p of results) {
    sql.push(
      `INSERT INTO fr_photographers(name_en, birth_year, death_year, nationality, biography, review_status, source)`,
      `VALUES('${esc(p.name)}', ${p.birthYear || "NULL"}, ${p.deathYear || "NULL"}, ${p.nationality ? `'${esc(p.nationality)}'` : "NULL"},`,
      `  ${p.biography ? `'${esc(p.biography)}'` : "NULL"}, 'imported', 'wikidata+wikipedia')`,
      `ON CONFLICT (name_en) DO NOTHING;`,
      "",
    );
  }
  sql.push("COMMIT;");
  const output = sql.join("\n");
  
  if (process.argv.includes("--dry-run")) {
    console.log(output.slice(0, 3000));
    console.error(`（dry-run 模式，完整 SQL ${output.length} 字节）`);
  } else {
    await Deno.writeFile ? console.log(output) : process.stdout.write(output);
  }
}

main().catch((e) => { console.error("❌", e.message); process.exit(1); });
