import fs from "node:fs";

const catalog = JSON.parse(fs.readFileSync("FocalRoom/Localizable.xcstrings", "utf8"));
const existing = Object.keys(catalog.strings);
const translations = JSON.parse(fs.readFileSync("/tmp/i18n-translations.json", "utf8"));
const livePlain = JSON.parse(fs.readFileSync("/tmp/i18n-live-plain.json", "utf8"));
const interpSites = JSON.parse(fs.readFileSync("/tmp/i18n-live-interp-sites.json", "utf8"));

// Strings in the catalog are runtime keys, so Swift source escapes are decoded first.
function decode(raw) {
  return raw
    .replace(/\\\(\s/g, "\u0000(")
    .replace(/\\n/g, "\n")
    .replace(/\\t/g, "\t")
    .replace(/\\"/g, '"')
    .replace(/\\\\/g, "\\")
    .replaceAll("\u0000(", "\\(");
}

// Verified against the declarations in FocalRoomCore/Models.swift and the App-side call sites.
const INT_EXPR = new Set([
  "draft.items.count", "draft.photos.count", "index + 1", "library.completed", "library.duplicates",
  "library.failures.count", "library.imported", "library.total", "migration.total", "migration.uploaded",
  "model.aiTargetCount", "model.snapshot?.snapshot.photos.count ?? 0", "move.fromIndex + 1", "move.toIndex + 1",
  "nodes.count", "picked.count", "position + 1", "project.migration?.uploadedPhotoIDs.count ?? 0",
  "project.photos.count", "project?.photos.count ?? 0", "proposal.operations.count", "purchasedProductIDs.count",
  "result.comment_count", "row.pendingCanvasMoves", "selected.count", "selectedPhotos.count", "target.count",
  "task.inspectedPhotos.count", "board.revision", "diff.from.revision", "diff.to.revision",
  "projection.baseRevision", "projection.currentRevision", "proposal.base_revision", "project.revision",
  "row.revision", "version.revision",
  // 草案画面依据口径一族（NativeDraftEvidenceSection）：这几格全是 Int，插值必须落成 %lld。
  "evidence.observed", "evidence.unobserved", "evidence.pageTier", "evidence.detailTier",
  "evidence.otherVersion", "evidence.otherTool", "evidence.crossContractPairs", "evidence.omitted",
  "draftRevision", "then",
]);

function spec(expr) {
  const e = expr.trim();
  if (INT_EXPR.has(e)) return "%lld";
  if (/\.count$|\+\s*1$|\.revision$|_revision$|Count$/.test(e)) return "%lld";
  return "%@";
}

function toKey(rawLit) {
  let out = "";
  let i = 0;
  const lit = decode(rawLit);
  while (i < lit.length) {
    if (lit[i] === "\\" && lit[i + 1] === "(") {
      let depth = 1;
      let j = i + 2;
      const start = j;
      while (j < lit.length && depth > 0) {
        if (lit[j] === "(") depth++;
        else if (lit[j] === ")") depth--;
        if (depth === 0) break;
        j++;
      }
      out += spec(lit.slice(start, j));
      i = j + 1;
      continue;
    }
    out += lit[i];
    i++;
  }
  return out;
}

const byDecoded = new Map();
for (const [zh, t] of translations) byDecoded.set(decode(zh), t);

const keys = new Map(); // key -> {zh, src}
for (const lit of livePlain) {
  const k = decode(lit);
  if (!keys.has(k)) keys.set(k, { lit, plain: true });
}
const interpKeys = [];
for (const s of interpSites) {
  const k = toKey(s.val);
  if (!keys.has(k)) keys.set(k, { lit: s.val, plain: false, site: `${s.file}:${s.line}` });
  interpKeys.push(k);
}

// literals reached through Text(LocalizedStringKey(rawValue)) / localizedText() rather than a literal
const coreDir = "Sources/FocalRoomCore";
const coreLits = new Set();
for (const f of fs.readdirSync(coreDir).filter((n) => n.endsWith(".swift"))) {
  const src = fs.readFileSync(`${coreDir}/${f}`, "utf8");
  const re = /"((?:[^"\\\n]|\\.)*[一-鿿](?:[^"\\\n]|\\.)*)"/g;
  let m;
  while ((m = re.exec(src))) {
    const v = decode(m[1]);
    if (!v.includes("\\(")) coreLits.add(v);
  }
}
const extras = ["选片任务", "聊天", "密码登录", "验证码登录", "待选", "保留", "排除", "知道了", ...coreLits];
for (const e of extras) if (!keys.has(e)) keys.set(e, { lit: e, plain: true, extra: true });

const report = { total: keys.size, inCatalog: 0, fromWork: 0, needAuthoring: [] };
for (const [k, v] of keys) {
  if (existing.includes(k)) report.inCatalog++;
  else if (byDecoded.has(k)) report.fromWork++;
  else report.needAuthoring.push({ key: k, lit: v.lit, site: v.site || "", plain: v.plain });
}
const gaps = [];
for (const k of existing) {
  const loc = catalog.strings[k].localizations || {};
  for (const lang of ["en", "ja", "ko"]) if (!loc[lang]?.stringUnit?.value) gaps.push({ key: k, lang });
}
console.log(`existing keys missing a language: ${gaps.length}`);
fs.writeFileSync("/tmp/i18n-gaps.json", JSON.stringify(gaps, null, 1));
fs.writeFileSync("/tmp/i18n-final-keys.json", JSON.stringify([...keys.keys()], null, 1));
fs.writeFileSync("/tmp/i18n-need.json", JSON.stringify(report.needAuthoring, null, 1));
console.log(`catalog keys now: ${existing.length}`);
console.log(`required by code: ${report.total}`);
console.log(`  already in catalog: ${report.inCatalog}`);
console.log(`  translation ready:  ${report.fromWork}`);
console.log(`  need authoring:     ${report.needAuthoring.length}`);
console.log(`  interpolated:       ${interpKeys.length}`);
