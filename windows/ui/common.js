// Ortak yardımcılar: Rust komutları, metinler, simgeler, limit çubuğu.
const TAURI = window.__TAURI__;
const invoke = (cmd, args) => TAURI.core.invoke(cmd, args || {});
const listen = (ev, fn) => TAURI.event.listen(ev, fn);

/** Metinler: Türkçe anahtar → seçili dil, kedinin adı işlenmiş (Mac: L.t + CatName.apply).
 *  Bütün sözlük açılışta bir kez alınır; dil ya da kedinin adı değişince yenilenir. Sözlükte olmayan metne
 *  Mac'teki gibi öbek değiştirme uygulanır. */
const T = {
  map: {}, ph: [], name: "Tonti", sig: "",
  async load() {
    const r = await invoke("tr_all");
    this.map = r.map || {}; this.ph = r.phrases || []; this.name = r.name || "Tonti"; this.sig = r.en + "|" + this.name;
  },
  /** Durum güncellemesinde: dil ya da ad değiştiyse sözlüğü yeniden al. */
  async sync(st) { if (st && st.catName !== undefined && (st.en + "|" + st.catName) !== this.sig) await this.load(); },
  t(k) {
    const v = this.map[k];
    if (v != null) return v;
    let s = k;
    for (const [a, b] of this.ph) s = s.split(a).join(b);
    return catNameApply(s, this.name);
  },
  f(k, ...a) { let s = this.t(k); for (const x of a) s = s.replace(/%lld|%@/, x); return s.replace(/%%/g, "%"); },
};

/** Türkçe ekler kedinin adının son ünlüsüne göre (Mac: CatName.apply; Rust: l10n::cat_name_apply). */
function catNameApply(s, n) {
  if (!s.includes("Tonti") || n === "Tonti") return s;
  const V = "aeıioöuüAEIİOÖUÜ";
  const low = c => (c === "I" ? "ı" : c === "İ" ? "i" : c.toLowerCase());
  const last = [...n].reverse().map(low).find(c => V.includes(c)) || "e";
  const nar = "aı".includes(last) ? "ı" : "ou".includes(last) ? "u" : "öü".includes(last) ? "ü" : "i";
  const endV = V.includes(n.slice(-1));
  return s.replace(/Tonti'nin/g, n + "'" + (endV ? "n" : "") + nar + "n").replace(/Tonti'yi/g, n + "'" + (endV ? "y" : "") + nar)
    .replace(/Tonti's/g, n + "'s").replace(/Tonti/g, n);
}

function esc(s) { return String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c])); }

function setTheme(dark) { document.documentElement.dataset.theme = dark ? "dark" : "light"; }

/** SF Symbols karşılıkları (gearshape.fill, arrow.clockwise, power, bolt.fill, calendar). */
const ICON = {
  gear: '<svg class="f" viewBox="0 0 24 24"><path d="M13.9 2l.5 2.6c.6.2 1.2.5 1.7.9l2.5-.9 1.9 3.3-2 1.7c.1.6.1 1.3 0 1.9l2 1.7-1.9 3.3-2.5-.9c-.5.4-1.1.7-1.7.9l-.5 2.6h-3.8l-.5-2.6c-.6-.2-1.2-.5-1.7-.9l-2.5.9-1.9-3.3 2-1.7c-.1-.6-.1-1.3 0-1.9l-2-1.7 1.9-3.3 2.5.9c.5-.4 1.1-.7 1.7-.9l.5-2.6zM12 8.6a3.4 3.4 0 100 6.8 3.4 3.4 0 000-6.8z" fill-rule="evenodd"/></svg>',
  refresh: '<svg class="s" viewBox="0 0 24 24"><path d="M19 12a7 7 0 11-2.05-4.95"/><path d="M17.5 3.5v4h-4"/></svg>',
  power: '<svg class="s" viewBox="0 0 24 24"><path d="M12 3v8"/><path d="M7.3 6.2a7.5 7.5 0 109.4 0"/></svg>',
  bolt: '<svg viewBox="0 0 24 24" width="10" height="10"><path d="M13 2L4 14h6l-1 8 9-12h-6z" fill="currentColor"/></svg>',
  cal: '<svg viewBox="0 0 24 24" width="10" height="10"><path d="M4 5h16v15H4zM4 9h16M8 3v4M16 3v4" fill="none" stroke="currentColor" stroke-width="2"/></svg>',
};

/** Limit çubuğu: kalan oran, durum rengi, tempo işareti (beklenen kalan). */
function paceBar(row) {
  const w = Math.max(0, Math.min(1, row.left));
  const mark = row.expected == null ? "" : `<i class="mark" style="left:calc(${row.expected * 100}% - 1px)"></i>`;
  return `<div class="pace"><div class="track"><div class="fill" style="width:max(8px, ${w * 100}%);background:${row.color}"></div></div>${mark}</div>`;
}

/** SwiftUI sistem renkleri (Chip, Pace). */
const SYS = { green: "#34c759", orange: "#ff9500", red: "#ff3b30", blue: "#007aff" };

function statusWord(left) {
  if (left > 0.4) return ["Sağlıklı", SYS.green];
  if (left > 0.15) return ["Dikkat", SYS.orange];
  return ["Kritik", SYS.red];
}

/** Kullanım geçen süreyle orantılıysa "Dengeli"; öndeyse "Hızlı", gerideyse "Rahat". */
function pace(row) {
  if (row.expected == null) return null;
  const elapsed = 1 - row.expected, used = 1 - row.left, d = used - elapsed;
  if (row.left <= 0.001) return ["Bitti", SYS.red];
  if (d > 0.25) return ["Çok hızlı", SYS.red];
  if (d > 0.1) return ["Hızlı", SYS.orange];
  if (d < -0.15) return ["Rahat", SYS.blue];
  return ["Dengeli", SYS.green];
}

function chip(text, color) {
  return `<span class="chip" style="color:${color};background:color-mix(in srgb, ${color} 16%, transparent)">${esc(text)}</span>`;
}

/** Sağlayıcı logosu. Codex tek renkli bir maskedir: Mac'teki gibi marka grisiyle boyanır (BrandBadge). */
/** Ek araçların logosu yok: Mac'teki gibi harfli renkli rozet (Provider.short, Provider.color). */
const BADGE = { opencode: ["O", "#4dcc8c"], kimi: ["K", "#855ceb"], cline: ["L", "#f2bf40"] };
function logo(p, size) {
  if (BADGE[p]) return `<i class="logo" style="display:inline-flex;align-items:center;justify-content:center;width:${size}px;height:${size}px;border-radius:${Math.round(size * 0.28)}px;background:${BADGE[p][1]};color:#fff;font:700 ${Math.round(size * 0.58)}px var(--mono);font-style:normal">${BADGE[p][0]}</i>`;
  if (p === "codex") return `<i class="logo" style="display:inline-block;width:${size}px;height:${size}px;background:#9ea1a8;-webkit-mask:url(img/logos/codex.png) center/contain no-repeat;mask:url(img/logos/codex.png) center/contain no-repeat"></i>`;
  return `<img class="logo" src="img/logos/${p}.png" style="width:${size}px;height:${size}px">`;
}
/** Bu bilgisayarda hiçbir yapay zekâ aracının izi yoksa sayfaların başında çıkan uyarı (boş sayfa yerine).
 *  Durum `get_state` → `detected` listesinden gelir; liste doluysa hiçbir şey çizilmez. */
const NOTOOLS_KEYS = ["Bu bilgisayarda yapay zekâ aracı bulunamadı",
  "Claude Code, Codex, Gemini CLI, Antigravity, OpenCode, Kimi ya da Cline kullanmaya başlayınca Tonti burayı kendiliğinden doldurur. Hangisinin kurulu olmadığını Ayarlar → Sağlayıcılar'da görebilirsin.",
  "SAĞLAYICILAR"];
function noToolsBanner(st) {
  if (!st || !Array.isArray(st.detected) || st.detected.length) return "";
  return `<div class="px notools"><img class="px-img" src="img/cat/sleep.png" alt=""><div class="col"><b>${T.t(NOTOOLS_KEYS[0])}</b><span>${T.t(NOTOOLS_KEYS[1])}</span></div>
    <button class="btn px" data-page="providers">${T.t(NOTOOLS_KEYS[2])}</button></div>`;
}
