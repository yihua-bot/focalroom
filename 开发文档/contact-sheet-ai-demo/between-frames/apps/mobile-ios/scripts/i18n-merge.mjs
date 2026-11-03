import fs from "node:fs";

const PATH = "FocalRoom/Localizable.xcstrings";
const catalog = JSON.parse(fs.readFileSync(PATH, "utf8"));
const required = JSON.parse(fs.readFileSync("/tmp/i18n-final-keys.json", "utf8"));
const rows = JSON.parse(fs.readFileSync("/tmp/i18n-translations.json", "utf8"));
const author = JSON.parse(fs.readFileSync("/tmp/i18n-author.json", "utf8"));

function decode(raw) {
  return raw
    .replace(/\\\(\s/g, "\u0000(")
    .replace(/\\n/g, "\n")
    .replace(/\\t/g, "\t")
    .replace(/\\"/g, '"')
    .replace(/\\\\/g, "\\")
    .replaceAll("\u0000(", "\\(");
}

const byKey = new Map();
for (const [zh, t] of rows) byKey.set(decode(zh), t);
const authored = new Map();
for (const [k, v] of Object.entries(author.interp)) authored.set(k, { en: v[0], ja: v[1], ko: v[2] });
for (const [k, v] of Object.entries(author.gaps)) {
  const cur = authored.get(k) || {};
  authored.set(k, { ...cur, ja: v[0], ko: v[1] });
}

const specCount = (s) => (s.match(/%(?:lld|@|lf|d|\$[\w$]+)/g) || []).length;
const stats = { added: 0, filled: 0, untranslated: [], mismatched: [] };

for (const key of required) {
  const entry = catalog.strings[key] || (catalog.strings[key] = { extractionState: "manual" });
  const loc = (entry.localizations ||= {});
  const fromRows = byKey.get(key) || {};
  const fromAuthor = authored.get(key) || {};
  for (const lang of ["en", "ja", "ko"]) {
    if (loc[lang]?.stringUnit?.value) continue;
    const value = fromAuthor[lang] || fromRows[lang];
    if (!value) {
      if (lang === "en") stats.untranslated.push(key);
      continue;
    }
    loc[lang] = { stringUnit: { state: "translated", value } };
    stats.filled++;
  }
  catalog.strings[key].extractionState = "manual";
}

for (const key of required) {
  const loc = catalog.strings[key]?.localizations || {};
  for (const lang of ["en", "ja", "ko"]) {
    const v = loc[lang]?.stringUnit?.value;
    if (v && specCount(v) !== specCount(key)) stats.mismatched.push(`${lang} | ${key} -> ${v}`);
  }
}

catalog.sourceLanguage = "zh-Hans";
catalog.version = "1.0";
fs.writeFileSync(PATH, JSON.stringify(catalog, null, 2) + "\n");

console.log(`keys in catalog now: ${Object.keys(catalog.strings).length} (required: ${required.length})`);
console.log(`translations written: ${stats.filled}`);
console.log(`still untranslated: ${stats.untranslated.length}`);
stats.untranslated.slice(0, 30).forEach((k) => console.log("  MISSING " + k));
console.log(`specifier mismatches: ${stats.mismatched.length}`);
stats.mismatched.slice(0, 30).forEach((m) => console.log("  " + m));
