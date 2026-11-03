import fs from "node:fs";
import path from "node:path";

const DIR = "FocalRoom";
const ZH = /[一-鿿]/;

// Positions whose string literal becomes a LocalizedStringKey lookup at runtime.
const LOCALIZED = [
  /String\(localized:\s*$/,
  /localizedText\($/,
  /\bText\($/,
  /\bButton\($/,
  /\bLabel\($/,
  /\bToggle\($/,
  /\bPicker\($/,
  /\bSection\($/,
  /\bContentUnavailableView\($/,
  /\bNavigationLink\($/,
  /navigationTitle\($/,
  /navigationSubtitle\($/,
  /\bAlert\($/,
  /\.alert\($/,
  /confirmationPrompt\($/,
  /settingsPrompt\($/,
  /\bLink\($/,
  /\bTextField\($/,
  /\bSecureField\($/,
  /\bTextEditor\($/,
  /\bProgressView\($/,
  /\bLabeledContent\($/,
  /\bStepper\($/,
  /\bMenu\($/,
  /\bGroupBox\($/,
  /\.confirmationDialog\($/,
  /prompt:\s*$/,
  /return\s+$/,
  /LocalizedStringKey\($/,
  /\balert\($/,
  /case\s+\w+\s*=\s*$/,
];

function literals(src) {
  const out = [];
  let i = 0;
  while (i < src.length) {
    if (src[i] === '"' && src[i - 1] !== "\\") {
      let j = i + 1;
      let val = "";
      let closed = false;
      while (j < src.length) {
        if (src[j] === "\\") {
          if (src[j + 1] === "(") {
            // interpolation: consume the balanced group, quotes inside belong to the expression
            let depth = 1;
            let k = j + 1;
            val += "\\(";
            k++;
            while (k < src.length && depth > 0) {
              if (src[k] === "(") depth++;
              else if (src[k] === ")") depth--;
              if (depth === 0) break;
              val += src[k];
              k++;
            }
            val += ")";
            j = k + 1;
            continue;
          }
          val += src[j] + (src[j + 1] ?? "");
          j += 2;
          continue;
        }
        if (src[j] === '"') {
          closed = true;
          break;
        }
        if (src[j] === "\n") break;
        val += src[j];
        j++;
      }
      if (closed) {
        out.push({ start: i, end: j, val });
        i = j + 1;
        continue;
      }
    }
    i++;
  }
  return out;
}

const found = new Map(); // literal content -> [{file, where}]
const skipped = [];
const interpSites = [];
for (const f of fs.readdirSync(DIR).filter((n) => n.endsWith(".swift"))) {
  const src = fs.readFileSync(path.join(DIR, f), "utf8");
  for (const lit of literals(src)) {
    if (!ZH.test(lit.val)) continue;
    const before = src.slice(Math.max(0, lit.start - 80), lit.start).replace(/\s+/g, " ");
    const lineStart = src.lastIndexOf("\n", lit.start) + 1;
    const lineSoFar = src.slice(lineStart, lit.start);
    if (lineSoFar.includes("//")) continue; // literal inside a comment
    const hit = LOCALIZED.some((re) => re.test(before));
    if (!hit) {
      skipped.push(`${f}: ${before.slice(-45)} | ${lit.val.slice(0, 60)}`);
      continue;
    }
    const line = src.slice(0, lit.start).split("\n").length;
    const hasInterp = /\\\(/.test(lit.val);
    if (hasInterp) interpSites.push({ file: f, line, val: lit.val, before: before.slice(-38) });
    if (!found.has(lit.val)) found.set(lit.val, { count: 0, files: new Set(), interp: hasInterp });
    const e = found.get(lit.val);
    e.count++;
    e.files.add(f);
    if (hasInterp) e.interp = true;
  }
}

const interp = [...found.keys()].filter((k) => k.includes("\\("));
const plain = [...found.keys()].filter((k) => !k.includes("\\("));
console.log(`localized literals: ${found.size}  (plain ${plain.length}, interpolated ${interp.length})`);
fs.writeFileSync("/tmp/i18n-live-plain.json", JSON.stringify(plain, null, 1));
fs.writeFileSync("/tmp/i18n-live-interp.json", JSON.stringify(interp, null, 1));
fs.writeFileSync("/tmp/i18n-live-interp-sites.json", JSON.stringify(interpSites, null, 1));
console.log(`\ninterpolated sites:`);
for (const s of interpSites) console.log(`  ${s.file}:${s.line}  [${s.before}]\n      ${s.val}`);
console.log(`\nnon-localized Chinese literals (left as-is, count ${skipped.length}):`);
for (const s of skipped) console.log("  " + s);
