// Mac'in Sources/UllageCat/L10n.swift sözlüğünü (exact + phrases) windows/src-tauri/l10n_en.json'a çıkarır.
// Swift dize sabitleri sırayla okunur (aynı satırdaki birden çok çift dahil). Windows'a özgü mevcut anahtarlar korunur.
// Kullanım: node windows/scripts/extract-l10n.js
const fs = require("fs");
const path = require("path");
const root = path.join(__dirname, "..", "..");
const src = fs.readFileSync(path.join(root, "Sources/UllageCat/L10n.swift"), "utf8");
const out = path.join(root, "windows/src-tauri/l10n_en.json");

function block(name) {
  const i = src.indexOf(`static let ${name}:`);
  if (i < 0) throw new Error(name + " bulunamadı");
  const start = src.indexOf("[", src.indexOf("=", i));
  // Eşleşen kapanış köşeli parantezi (dizelerin içindekiler sayılmaz).
  let depth = 0, inStr = false;
  for (let j = start; j < src.length; j++) {
    const c = src[j];
    if (inStr) { if (c === "\\") { j++; continue; } if (c === '"') inStr = false; continue; }
    if (c === '"') { inStr = true; continue; }
    if (c === "/" && src[j + 1] === "/") { j = src.indexOf("\n", j); continue; }
    if (c === "[") depth++;
    if (c === "]" && --depth === 0) return src.slice(start + 1, j);
  }
  throw new Error(name + " kapanmıyor");
}

function strings(body) {
  const res = [];
  for (let j = 0; j < body.length; j++) {
    const c = body[j];
    if (c === "/" && body[j + 1] === "/") { j = body.indexOf("\n", j); if (j < 0) break; continue; }
    if (c !== '"') continue;
    let s = "";
    for (j++; j < body.length && body[j] !== '"'; j++) {
      if (body[j] === "\\") {
        const n = body[++j];
        if (n === "n") s += "\n"; else if (n === "t") s += "\t"; else if (n === "u") {
          const e = body.indexOf("}", j); s += String.fromCodePoint(parseInt(body.slice(j + 2, e), 16)); j = e;
        } else s += n;
      } else s += body[j];
    }
    res.push(s);
  }
  return res;
}

const ex = strings(block("exact"));
if (ex.length % 2) throw new Error("exact: tek sayıda dize (" + ex.length + ")");
const ph = strings(block("phrases"));
const old = fs.existsSync(out) ? JSON.parse(fs.readFileSync(out, "utf8")) : {};
const dict = {};
for (let i = 0; i < ex.length; i += 2) dict[ex[i]] = ex[i + 1];
let kept = 0;
for (const [k, v] of Object.entries(old)) if (k !== "__phrases" && !(k in dict)) { dict[k] = v; kept++; }
const phrases = [];
for (let i = 0; i < ph.length; i += 2) phrases.push([ph[i], ph[i + 1]]);
dict.__phrases = JSON.stringify(phrases);
fs.writeFileSync(out, JSON.stringify(dict));
console.log(`Mac çifti: ${ex.length / 2}, öbek: ${phrases.length}, Windows'a özgü korunan: ${kept}, toplam: ${Object.keys(dict).length - 1}`);
