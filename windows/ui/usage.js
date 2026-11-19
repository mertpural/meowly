// Kullanım sayfası (Mac: HistoryView.swift → HistoryPage, StatCard, CostCard, ActivityStats, HeatmapCard; Extras.swift → ProjectsCard).
const USAGE_KEYS = ["Kullanım", "Yapay zekâ araçlarının token kullanımı, tahmini maliyeti ve geçmişi. Yalnızca sayılar okunur.",
  "BUGÜN", "DÜN", "SON 7 GÜN", "SON 30 GÜN", "düne göre", "önceki haftaya göre", "TAHMİNİ API MALİYETİ",
  "Aynı token'ları API fiyatıyla alsaydın ödeyeceğin tutar. Abonelik ücretin değil, tahminidir.", "Bugün", "7 gün", "30 gün",
  "EN ÇOK KULLANILAN PROJELER", "Yalnızca Claude Code, klasör adıyla", "90 gün", "Tümü", "Henüz proje verisi yok.",
  "TOPLAM TOKEN", "tüm zamanlar", "GÜNCEL SERİ", "%lld gün", "kesintisiz kullanım", "bugün kullanım yok", "EN UZUN SERİ",
  "%lld aktif gün", "EN YOĞUN GÜN", "GÜNLÜK ORT.", "aktif gün başına", "Etkinlik", "Son 53 hafta", "Sağlayıcıya göre renk",
  "Yoğunluk", "Pzt", "Çar", "Cum", "Bir günün üzerine gel, o günkü kullanımı gör.", "%@ · kullanım yok", "%@ · %@ token · %@",
  "Az", "Çok", "24 saat", "Sağlayıcı", "Model", "Cache dahil", "Hepsi", "Paylaşım kartı", "CSV", "Token kullanımı", " toplam",
  "Bu aralıkta kayıt yok.", "Dağılım"];

const U = { range: "week", group: "provider", filter: null, hover: null, data: null };
const PNAME = { claude: "Claude", codex: "Codex", antigravity: "Antigravity", gemini: "Gemini", opencode: "OpenCode", kimi: "Kimi", cline: "Cline" };

function fmtTok(n) {
  if (n >= 1e9) return (n / 1e9).toFixed(1) + "B";
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  if (n >= 1e3) return Math.floor(n / 1e3) + "K";
  return String(n);
}
const loc = () => (U.data && U.data.en ? "en" : "tr");
function keyOf(d) { return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`; }
function dateOf(k) { const [y, m, d] = k.split("-").map(Number); return new Date(y, m - 1, d); }

function dayTotal(k) {
  const d = U.data.heat[k];
  if (!d) return 0;
  if (U.filter) return d[U.filter] || 0;
  return Object.values(d).reduce((a, b) => a + b, 0);
}

function streaks() {
  const s = { current: 0, longest: 0, active: 0, bestKey: null, best: 0 };
  const keys = Object.keys(U.data.heat).filter(k => dayTotal(k) > 0).sort();
  s.active = keys.length;
  for (const k of keys) if (dayTotal(k) > s.best) { s.best = dayTotal(k); s.bestKey = k; }
  let run = 0, prev = null;
  for (const k of keys) {
    const d = dateOf(k);
    run = prev && Math.round((d - prev) / 86400000) === 1 ? run + 1 : 1;
    s.longest = Math.max(s.longest, run);
    prev = d;
  }
  const day = new Date(); day.setHours(0, 0, 0, 0);
  if (dayTotal(keyOf(day)) === 0) day.setDate(day.getDate() - 1);
  while (dayTotal(keyOf(day)) > 0) { s.current++; day.setDate(day.getDate() - 1); }
  return s;
}

function statCard(title, value, compareTo, label) {
  let line = "&nbsp;";
  if (compareTo != null && compareTo > 0) {
    const d = (value - compareTo) / compareTo, up = d >= 0;
    // Önceki dönem çok küçükse yüzde anlamsız büyür: iki katını aşınca "4×" gibi kat yazılır.
    const txt = d >= 1 ? `${Math.round(d + 1)}× ${label}` : `${Math.abs(Math.round(d * 100))}% ${label}`;
    line = `<span style="color:${up ? "var(--amber)" : "var(--sub)"}"><b class="arrow">${up ? "↗" : "↘"}</b> ${esc(txt)}</span>`;
  }
  return `<div class="px u-stat"><span class="t">${T.t(title)}</span><b>${fmtTok(value)}</b><span class="c">${line}</span></div>`;
}

const SYM = {
  sum: "∑", flame: '<svg viewBox="0 0 24 24"><path d="M12 2c1 4 6 6 6 12a6 6 0 01-12 0c0-3 2-5 3-6 0 2 1 3 2 3 0-4-1-6 1-9z" fill="currentColor"/></svg>',
  trophy: '<svg viewBox="0 0 24 24"><path d="M6 3h12v3h3v2a5 5 0 01-5 5 5 5 0 01-3 2v3h3v3H8v-3h3v-3a5 5 0 01-3-2 5 5 0 01-5-5V6h3zM5 8a3 3 0 001 2V8zm14 0h-1v2a3 3 0 001-2z" fill="currentColor"/></svg>',
  bolt: '<svg viewBox="0 0 24 24"><path d="M13 2L4 14h6l-1 8 9-12h-6z" fill="currentColor"/></svg>',
  chart: '<svg viewBox="0 0 24 24"><path d="M3 3v18h18M6 15l4-4 3 3 6-6M15 8h4v4" fill="none" stroke="currentColor" stroke-width="2"/></svg>',
};

function stat(icon, title, value, sub) {
  return `<div class="px u-stat2"><span class="h"><i>${SYM[icon]}</i>${T.t(title)}</span><b>${esc(value)}</b><span class="s">${esc(sub)}</span></div>`;
}

function seg(items, sel, attr) {
  return `<span class="seg">${items.map(([lbl, v]) => `<button class="btn px ${String(v) === String(sel) ? "on" : ""}" data-${attr}="${v}">${esc(lbl)}</button>`).join("")}</span>`;
}

function heatmap() {
  const weeks = 53, d = U.data;
  const today = new Date(); today.setHours(0, 0, 0, 0);
  const monday = new Date(today); monday.setDate(today.getDate() - ((today.getDay() + 6) % 7));
  const start = new Date(monday); start.setDate(monday.getDate() - (weeks - 1) * 7);
  const grid = [];
  for (let w = 0; w < weeks; w++) {
    const col = [];
    for (let r = 0; r < 7; r++) { const x = new Date(start); x.setDate(start.getDate() + w * 7 + r); col.push(x > today ? null : x); }
    grid.push(col);
  }
  const vals = grid.flat().filter(Boolean).map(x => dayTotal(keyOf(x))).filter(v => v > 0).sort((a, b) => a - b);
  const th = vals.length ? [0.25, 0.5, 0.75].map(q => vals[Math.min(vals.length - 1, Math.floor(vals.length * q))]) : [];
  const level = v => { if (v <= 0) return 0; for (let i = 0; i < th.length; i++) if (v <= th[i]) return i + 1; return th.length + 1; };
  const monthFmt = new Intl.DateTimeFormat(loc(), { month: "short" });
  const months = {}; let last = -1;
  grid.forEach((col, i) => { const f = col.find(Boolean); if (!f) return; if (f.getMonth() !== last) { months[i] = monthFmt.format(f).replace(".", ""); last = f.getMonth(); } });
  const accent = "var(--accent)";
  const cells = grid.map((col, w) => `<div class="hcol">${col.map(x => {
    if (!x) return `<i class="hc empty"></i>`;
    const k = keyOf(x), v = dayTotal(k), lv = level(v);
    const hov = U.hover === k ? " hov" : "";
    if (d.heatByProvider && v > 0) {
      const day = d.heat[k];
      const parts = ["claude", "codex", "gemini", "antigravity", "opencode", "kimi", "cline"].filter(p => day[p] > 0 && (!U.filter || U.filter === p))
        .map(p => `<i style="background:${d.providerColors[p] || "var(--sub)"};height:${day[p] / v * 100}%"></i>`).join("");
      return `<i class="hc split${hov}" data-k="${k}" style="opacity:${[0, 0.5, 0.68, 0.84, 1][Math.min(lv, 4)]}">${parts}</i>`;
    }
    const bg = d.heatByProvider || lv === 0 ? "color-mix(in srgb, var(--sub) 14%, transparent)"
      : `color-mix(in srgb, ${accent} ${[0, 28, 50, 75, 100][Math.min(lv, 4)]}%, transparent)`;
    return `<i class="hc${hov}" data-k="${k}" style="background:${bg}"></i>`;
  }).join("")}</div>`).join("");
  const dayLbl = ["Pzt", "", "Çar", "", "Cum", "", ""].map(x => `<span>${x ? T.t(x) : ""}</span>`).join("");
  const monthLbl = Object.entries(months).map(([i, m]) => `<span style="left:calc(var(--lw) + ${i} * (var(--cell) + var(--gap)))">${esc(m)}</span>`).join("");
  let readout = T.t("Bir günün üzerine gel, o günkü kullanımı gör.");
  if (U.hover) {
    const x = dateOf(U.hover);
    const ds = new Intl.DateTimeFormat(loc(), { day: "numeric", month: "long", year: "numeric" }).format(x) + ", " +
      new Intl.DateTimeFormat(loc(), { weekday: "long" }).format(x);
    const tot = dayTotal(U.hover);
    if (!tot) readout = T.f("%@ · kullanım yok", ds);
    else {
      const parts = Object.entries(d.heat[U.hover] || {}).filter(([p]) => !U.filter || p === U.filter).sort((a, b) => b[1] - a[1])
        .map(([p, v]) => `${PNAME[p] || p} ${fmtTok(v)}`).join(" · ");
      readout = T.f("%@ · %@ token · %@", ds, fmtTok(tot), parts);
    }
  }
  const legend = d.heatByProvider
    ? ["claude", "codex", "antigravity"].filter(p => !U.filter || U.filter === p).map(p => `<span class="lg"><i style="background:${d.providerColors[p]}"></i>${PNAME[p]}</span>`).join("")
    : `<span class="lg">${T.t("Az")}</span>${[0, 1, 2, 3, 4].map(lv => `<i class="lv" style="background:${lv === 0 ? "color-mix(in srgb, var(--sub) 14%, transparent)" : `color-mix(in srgb, ${accent} ${[0, 28, 50, 75, 100][lv]}%, transparent)`}"></i>`).join("")}<span class="lg">${T.t("Çok")}</span>`;
  return `<div class="px u-card heat">
    <div class="row hh"><b>${T.t("Etkinlik")}</b><span class="sub">${T.t("Son 53 hafta")}</span><span class="spacer"></span>
      <button class="btn px pill ${d.heatByProvider ? "on" : ""}" data-heat="1">${T.t("Sağlayıcıya göre renk")}</button>
      <button class="btn px pill ${!d.heatByProvider ? "on" : ""}" data-heat="0">${T.t("Yoğunluk")}</button></div>
    <div class="hgrid"><div class="months">${monthLbl}</div><div class="hbody"><div class="days">${dayLbl}</div><div class="hcols">${cells}</div></div></div>
    <div class="row hf"><span class="readout">${esc(readout)}</span><span class="spacer"></span><span class="legend">${legend}</span></div>
  </div>`;
}

const PALETTE = ["var(--pink)", "var(--amber)", "#789ed6", "#78b88f", "#a387d6", "var(--sub)", "#dbc266", "#b84f69"];

function chartData() {
  const d = U.data;
  const pts = d.points.filter(p => !U.filter || p.p === U.filter);
  const label = p => U.group === "provider" ? (PNAME[p.p] || p.p) : p.m;
  const totals = {};
  for (const p of pts) totals[label(p)] = (totals[label(p)] || 0) + p.t;
  const order = Object.entries(totals).sort((a, b) => b[1] - a[1]);
  const color = name => d.colors[name] || PALETTE[Math.max(0, order.findIndex(o => o[0] === name)) % PALETTE.length];
  return { pts, label, order, color };
}

function niceTicks(max) {
  if (max <= 0) return [0];
  const raw = max / 4, mag = Math.pow(10, Math.floor(Math.log10(raw)));
  const step = [1, 2, 2.5, 5, 10].map(m => m * mag).find(s => s >= raw) || 10 * mag;
  const out = []; for (let v = 0; v <= max + 1e-9; v += step) out.push(v);
  return out;
}

function chart() {
  const d = U.data, { pts, label, order, color } = chartData();
  const total = order.reduce((a, o) => a + o[1], 0);
  let body;
  if (!pts.length) body = `<div class="nodata">${T.t("Bu aralıkta kayıt yok.")}</div>`;
  else {
    // Kovalar: aralığın tamamı (veri az olsa bile tek çubuk tüm genişliği doldurmasın).
    const buckets = [];
    const s = new Date(d.since * 1000);
    if (d.hourly) { s.setMinutes(0, 0, 0); for (let t = new Date(s); t <= new Date(d.now * 1000); t.setHours(t.getHours() + 1)) buckets.push(new Date(t)); }
    else { s.setHours(0, 0, 0, 0); const e = new Date(); e.setHours(0, 0, 0, 0); for (let t = new Date(s); t <= e; t.setDate(t.getDate() + 1)) buckets.push(new Date(t)); }
    const bkey = t => d.hourly ? `${keyOf(t)} ${String(t.getHours()).padStart(2, "0")}:00` : keyOf(t);
    const stacks = buckets.map(t => { const k = bkey(t); const m = {}; for (const p of pts) if (p.b === k) m[label(p)] = (m[label(p)] || 0) + p.t; return m; });
    const max = Math.max(1, ...stacks.map(m => Object.values(m).reduce((a, b) => a + b, 0)));
    const ticks = niceTicks(max), top = ticks[ticks.length - 1] || max;
    // Gerçek piksel genişliğinde çiz (esnetme yazıları bozar); üstte etiket payı.
    const W = Math.max(300, (document.getElementById("main").clientWidth || 1000) - 48 - 32), H = 260, padR = 52, padB = 22, padT = 8;
    const cw = (W - padR) / buckets.length, ph = H - padB - padT;
    let svg = ticks.map(v => { const y = padT + ph * (1 - v / top); return `<line x1="0" x2="${W - padR}" y1="${y}" y2="${y}" class="grid"/><text x="${W - padR + 6}" y="${y + 4}" class="yl">${fmtTok(v)}</text>`; }).join("");
    stacks.forEach((m, i) => {
      let y = H - padB;
      for (const [name] of [...order].reverse()) {
        const v = m[name]; if (!v) continue;
        const h = ph * v / top; y -= h;
        svg += `<rect x="${i * cw + cw * 0.12}" y="${y}" width="${cw * 0.76}" height="${h}" fill="${color(name)}"/>`;
      }
    });
    const want = 7, stepI = Math.max(1, Math.round(buckets.length / want));
    const fmt = d.hourly ? (t => String(t.getHours())) : (t => new Intl.DateTimeFormat(loc(), { day: "numeric", month: "short" }).format(t).replace(".", ""));
    buckets.forEach((t, i) => { if (i % stepI === 0) svg += `<text x="${i * cw + cw / 2}" y="${H - 6}" class="xl" text-anchor="middle">${esc(fmt(t))}</text>`; });
    body = `<svg viewBox="0 0 ${W} ${H}" width="${W}" height="${H}" class="chart">${svg}</svg>`;
  }
  const dist = order.filter(o => o[1] > 0).map(([n, v]) => `<div class="drow"><i class="sq" style="background:${color(n)}"></i><span class="dn">${esc(n)}</span>
    <span class="dbar"><i style="width:${v / Math.max(1, total) * 100}%;background:${color(n)}"></i></span><b>${fmtTok(v)}</b><span class="dp">${Math.round(v / Math.max(1, total) * 100)}%</span></div>`).join("");
  return `<div class="px u-card big"><div class="row ch"><b>${T.t("Token kullanımı")}</b><span class="spacer"></span><span class="sub">${fmtTok(total)}${T.t(" toplam")}</span></div>${body}</div>
    <div class="px u-card big"><b class="ct">${T.t("Dağılım")}</b>${dist}</div>`;
}

function renderUsage(main) {
  const d = U.data;
  const s = streaks();
  const avg = s.active ? Math.floor(Object.keys(d.heat).reduce((a, k) => a + dayTotal(k), 0) / s.active) : 0;
  const pretty = k => k ? new Intl.DateTimeFormat(loc(), { day: "numeric", month: "short", year: "numeric" }).format(dateOf(k)).replace(".", "") : "–";
  const mx = Math.max(1, ...d.projects.map(p => p.tokens));
  main.innerHTML = `<div class="usage">
    <div class="ptitle"><b>${T.t("Kullanım")}</b><span>${T.t("Yapay zekâ araçlarının token kullanımı, tahmini maliyeti ve geçmişi. Yalnızca sayılar okunur.")}</span></div>
    ${noToolsBanner(typeof S !== "undefined" ? S : null)}
    <div class="u-row">${statCard("BUGÜN", d.today, d.yesterday, T.t("düne göre"))}${statCard("DÜN", d.yesterday)}${statCard("SON 7 GÜN", d.last7, d.prev7, T.t("önceki haftaya göre"))}${statCard("SON 30 GÜN", d.last30)}</div>
    <div class="px u-card cost"><div class="cl"><span class="t">${T.t("TAHMİNİ API MALİYETİ")}</span><span class="sub">${T.t("Aynı token'ları API fiyatıyla alsaydın ödeyeceğin tutar. Abonelik ücretin değil, tahminidir.")}</span></div>
      ${[["Bugün", d.cost[0]], ["7 gün", d.cost[1]], ["30 gün", d.cost[2]]].map(([l, v]) => `<div class="cv"><b>~${v}</b><span>${T.t(l)}</span></div>`).join("")}</div>
    <div class="px u-card proj"><div class="row ph"><span class="t">${T.t("EN ÇOK KULLANILAN PROJELER")}</span><span class="spacer"></span><span class="sub">${T.t("Yalnızca Claude Code, klasör adıyla")}</span>
      ${seg([[T.t("30 gün"), 30], [T.t("90 gün"), 90], [T.t("Tümü"), 0]], d.projectDays, "pdays")}</div>
      ${d.projects.length ? "" : `<span class="sub">${T.t("Henüz proje verisi yok.")}</span>`}
      ${d.projects.map(p => `<div class="prow"><span class="pn">${esc(p.name)}</span><span class="pb"><i style="width:max(4px, ${p.tokens / mx * 100}%);background:${d.providerColors.claude}"></i></span><span class="pt">${p.fmt}</span><span class="pc">~${p.cost}</span></div>`).join("")}</div>
    <div class="u-row">${stat("sum", "TOPLAM TOKEN", fmtTok(d.allTime), T.t("tüm zamanlar"))}${stat("flame", "GÜNCEL SERİ", T.f("%lld gün", s.current), s.current > 0 ? T.t("kesintisiz kullanım") : T.t("bugün kullanım yok"))}
      ${stat("trophy", "EN UZUN SERİ", T.f("%lld gün", s.longest), T.f("%lld aktif gün", s.active))}${stat("bolt", "EN YOĞUN GÜN", fmtTok(s.best), pretty(s.bestKey))}${stat("chart", "GÜNLÜK ORT.", fmtTok(avg), T.t("aktif gün başına"))}</div>
    ${heatmap()}
    <div class="row ctl">${seg([[T.t("24 saat"), "day"], [T.t("7 gün"), "week"], [T.t("30 gün"), "month"]], U.range, "range")}
      ${seg([[T.t("Sağlayıcı"), "provider"], [T.t("Model"), "model"]], U.group, "group")}<span class="spacer"></span>
      <label class="check ${d.includeCache ? "on" : ""}" data-cache="1"><i>${d.includeCache ? "✓" : ""}</i>${T.t("Cache dahil")}</label></div>
    <div class="row flt"><button class="btn px pill ${!U.filter ? "on" : ""}" data-filter="">${T.t("Hepsi")}</button>
      ${["claude", "codex", "antigravity"].map(p => `<button class="btn px pill ${U.filter === p ? "on" : ""}" data-filter="${p}">${logo(p, 14)}${PNAME[p]}</button>`).join("")}
      <span class="spacer"></span><button class="btn px pill" data-csv="1">▦ ${T.t("CSV")}</button></div>
    ${chart()}
  </div>`;
}

async function loadUsage(main) {
  // İlk yüklemede -1: projeler kartının süresi kayıtlı ayardan alınır.
  U.data = await invoke("usage_page", { range: U.range, projectDays: U.data ? U.data.projectDays : -1 });
  renderUsage(main);
}

function usageClick(e, main) {
  const t = e.target.closest("[data-range],[data-group],[data-filter],[data-heat],[data-pdays],[data-cache],[data-csv]");
  if (!t) return false;
  const ds = t.dataset;
  if (ds.range) { U.range = ds.range; loadUsage(main); }
  else if (ds.group) { U.group = ds.group; renderUsage(main); }
  else if (ds.filter !== undefined) { U.filter = ds.filter || null; renderUsage(main); }
  else if (ds.heat !== undefined) { U.data.heatByProvider = ds.heat === "1"; invoke("set_usage_pref", { key: "heatByProvider", value: U.data.heatByProvider }); renderUsage(main); }
  else if (ds.pdays !== undefined) { U.data.projectDays = Number(ds.pdays); invoke("set_usage_pref", { key: "projectDays", value: U.data.projectDays }); loadUsage(main); }
  else if (ds.cache) { invoke("set_usage_pref", { key: "includeCache", value: !U.data.includeCache }).then(() => loadUsage(main)); }
  else if (ds.csv) invoke("save_csv");
  return true;
}

function usageHover(e, main) {
  const c = e.target.closest(".hc[data-k]");
  const k = c ? c.dataset.k : null;
  if (k === U.hover) return;
  U.hover = k;
  // Yalnızca ısı haritası ve okuma satırı yeniden çizilir.
  const old = main.querySelector(".heat");
  if (old) { const tmp = document.createElement("div"); tmp.innerHTML = heatmap(); old.replaceWith(tmp.firstElementChild); }
}
