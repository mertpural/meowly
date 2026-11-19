// Raporlar (Mac: Tonti/Reports.swift → ReportsPage, ReportCard). Kart ekranda ve paylaşılan resimde aynıdır.
const REPORT_KEYS = ["Raporlar", "Ne üzerinde çalıştığını gün, hafta, ay ya da yıl olarak gör; standup notu ya da paylaşım için kopyala.",
  "Günlük", "Haftalık", "Aylık", "Yıllık", "Günlük rapor", "Haftalık rapor", "Aylık rapor", "Yıllık rapor",
  "MARKDOWN KOPYALA", "RESMİ KOPYALA", "RESMİ KAYDET", "Kopyalandı", "Kaydedildi", "ÇIKANLAR", "Bu dönemde kayıt yok.",
  "+%lld commit daha", "daha az göster", "+%lld proje daha", "İSTATİSTİKLER", "çalışma", "commit", "oturum", "proje",
  "EN YOĞUN GÜN", "Tonti not aldı · bilgisayarında, sohbetler okunmadan", "Çalışma", "Oturum", "Commit", "Proje", "%lld commit", "BİLGİSAYARDA", "Bilgisayarda"];

const KINDS = [["day", "Günlük", "Günlük rapor"], ["week", "Haftalık", "Haftalık rapor"], ["month", "Aylık", "Aylık rapor"], ["year", "Yıllık", "Yıllık rapor"]];
const R = { kind: "week", t: null, data: null, open: new Set(), note: "", gen: 0 };

const rloc = () => (R.data && R.data.en ? "en" : "tr");
function rfmt(t, opts) { return new Intl.DateTimeFormat(rloc(), opts).format(new Date(t * 1000)).replace(/\./g, ""); }

/** Dönem etiketi (Mac: ReportPeriod.label). */
function periodLabel(d) {
  switch (d.kind) {
    case "day": return rfmt(d.start, { day: "numeric", month: "long", year: "numeric" }) + ", " + rfmt(d.start, { weekday: "long" });
    case "week": { const last = d.end - 1; return `${rfmt(d.start, { day: "numeric", month: "short" })} – ${rfmt(last, { day: "numeric", month: "short" })} ${rfmt(last, { year: "numeric" })}`; }
    case "month": { const s = rfmt(d.start, { month: "long", year: "numeric" }); return s.charAt(0).toLocaleUpperCase(rloc()) + s.slice(1); }
    default: return rfmt(d.start, { year: "numeric" });
  }
}

function rup(s) { return s.toLocaleUpperCase(rloc()); }

/** Kart. `interactive`: ekranda "+N commit daha" açılır; resimde kısa hâli kalır. */
function reportCard(d, interactive) {
  const title = T.t(KINDS.find(k => k[0] === d.kind)[2]);
  const projs = interactive ? d.projects : d.projects.slice(0, 8);
  const shipped = projs.map(p => {
    const open = interactive && R.open.has(p.name);
    const shown = open ? p.commits : p.commits.slice(0, d.perProject);
    let more = "";
    if (p.commits.length > shown.length) {
      const txt = T.f("+%lld commit daha", p.commits.length - shown.length);
      more = interactive ? `<a class="more" data-open="${esc(p.name)}">${esc(txt)}</a>` : `<span class="more sub">${esc(txt)}</span>`;
    } else if (open && p.commits.length > d.perProject) {
      more = `<a class="more" data-close="${esc(p.name)}">${T.t("daha az göster")}</a>`;
    }
    return `<div class="rp"><div class="row"><b>${esc(p.name)}</b><span class="spacer"></span><span class="sub r">${esc(p.right)}</span></div>
      ${shown.map(c => `<div class="rc"><b>✓</b><span>${esc(c)}</span></div>`).join("")}${more}
      ${p.commits.length ? "" : `<span class="sub r">${esc(p.tokens)}</span>`}</div>`;
  }).join("");
  const num = (v, k) => `<div class="rn"><b>${esc(String(v))}</b><span>${esc(rup(k))}</span></div>`;
  const busiest = d.kind !== "day" && d.busiest ? `<div class="rbusy"><span>${esc(rup(T.t("EN YOĞUN GÜN")))}</span>
    <b>${esc(rfmt(d.busiest.day, { day: "numeric", month: "short" }) + " " + rfmt(d.busiest.day, { weekday: "short" }))}</b><span class="d">${esc(d.busiest.dur)}</span></div>` : "";
  let chart = "";
  if (d.buckets.length) {
    const mx = Math.max(1, ...d.buckets.map(b => b.v));
    const lbl = b => d.kind === "week" ? rfmt(b.t, { weekday: "narrow" }) : d.kind === "month" ? rfmt(b.t, { day: "numeric" }) : rfmt(b.t, { month: "narrow" });
    chart = `<div class="rchart" style="gap:${d.buckets.length > 20 ? 2 : 6}px">${d.buckets.map(b => `<div class="rb">
      <i style="height:${Math.max(3, 54 * b.v / mx)}px;background:${b.v > 0 ? "var(--pink)" : "var(--line)"}"></i>
      ${d.buckets.length <= 12 ? `<span>${esc(lbl(b))}</span>` : ""}</div>`).join("")}</div>`;
  }
  return `<div class="px lift rcard">
    <div class="row rh"><div class="col" style="gap:3px"><b class="rt">${esc(title)}</b><span class="sub rl">${esc(periodLabel(d))}</span></div>
      <span class="spacer"></span><b class="brand2">Meowly</b><img class="px-img rcat" src="img/cat/content.png"></div>
    <div class="rdiv"></div>
    <div class="rcols"><div class="rship"><span class="rk">${T.t("ÇIKANLAR")}</span>
      ${d.projects.length ? "" : `<span class="sub" style="font-size:11.5px">${T.t("Bu dönemde kayıt yok.")}</span>`}
      ${shipped}${!interactive && d.projects.length > 8 ? `<span class="sub more">${T.f("+%lld proje daha", d.projects.length - 8)}</span>` : ""}</div>
      <div class="rsep"></div>
      <div class="rnums"><span class="rk">${T.t("İSTATİSTİKLER")}</span>${num(d.active, T.t("çalışma"))}${num(d.commits, T.t("commit"))}${num(d.sessions, T.t("oturum"))}${num(d.projectCount, T.t("proje"))}${num(d.tokens, "token")}${busiest}</div></div>
    ${computerRow(d.computer)}${chart}
    <span class="sub rfoot">✎ ${T.t("Tonti not aldı · bilgisayarında, sohbetler okunmadan")}</span>
  </div>`;
}

/** Bilgisayarda: toplam süre ve en çok kullanılan 5 uygulama (başlık ve site yok). */
function computerRow(c) {
  if (!c || c.active <= 0) return "";
  const meet = c.meeting > 0 ? `<span class="sub" style="font-size:10.5px;margin-left:6px">· ${esc(T.t("toplantı"))} ${esc(c.meetingText)}</span>` : "";
  return `<div class="rpc"><div class="row"><span class="rk">${T.t("BİLGİSAYARDA")}</span><span class="spacer"></span><b>${esc(c.activeText)}</b>${meet}</div>
    <div class="rpapps">${c.apps.map(a => `<div class="rpa"><div class="row">${a.icon ? `<img src="${a.icon}">` : ""}<b>${esc(a.name)}</b></div>
      <i style="width:${Math.max(4, 100 * a.frac)}px"></i><span class="sub">${esc(a.dur)}</span></div>`).join("")}</div></div>`;
}

function markdown(d) {
  const title = T.t(KINDS.find(k => k[0] === d.kind)[2]);
  let out = `## ${title} · ${periodLabel(d)}\n\n`;
  for (const p of d.projects) {
    out += `**${p.name}** · ${p.right}\n`;
    for (const c of p.commits) out += `- ${c}\n`;
    out += "\n";
  }
  if (d.computer && d.computer.active > 0) {
    out += `**${T.t("Bilgisayarda")} · ${d.computer.activeText}**\n`;
    for (const a of d.computer.apps) out += `- ${a.name}: ${a.dur}\n`;
    if (d.computer.meeting > 0) out += `- ${T.t("Toplantı")}: ${d.computer.meetingText}\n`;
    out += "\n";
  }
  out +=`${T.t("Çalışma")}: ${d.active} · ${T.t("Oturum")}: ${d.sessions} · ${T.t("Commit")}: ${d.commits} · ${T.t("Proje")}: ${d.projectCount} · Token: ${d.tokens}\n`;
  return out;
}

function renderReports(main) {
  const d = R.data;
  main.innerHTML = `<div class="reports">
    <div class="ptitle rpad"><b>${T.t("Raporlar")}</b><span>${T.t("Ne üzerinde çalıştığını gün, hafta, ay ya da yıl olarak gör; standup notu ya da paylaşım için kopyala.")}</span></div>
    ${noToolsBanner(typeof S !== "undefined" ? S : null)}
    <div class="row rctl">${KINDS.map(([k, t]) => `<button class="btn px ${d.kind === k ? "on" : ""}" data-kind="${k}">${esc(rup(T.t(t)))}</button>`).join("")}
      <span class="spacer"></span><button class="btn px" data-rshift="-1">‹</button><b class="rlabel">${esc(periodLabel(d))}</b><button class="btn px" data-rshift="1">›</button></div>
    <div class="rbody">${reportCard(d, true)}
      <div class="row rbtns"><button class="btn px" data-act="md">${T.t("MARKDOWN KOPYALA")}</button><button class="btn px on" data-act="copy">${T.t("RESMİ KOPYALA")}</button>
        <button class="btn px" data-act="save">${T.t("RESMİ KAYDET")}</button><span class="sub rnote">${esc(R.note)}</span></div></div>
  </div>`;
}

async function loadReports(main, shiftBy = 0) {
  const g = ++R.gen;
  const data = await invoke("report_page", { kind: R.kind, t: R.t ?? Date.now() / 1000, shiftBy });
  if (g !== R.gen) return;
  R.data = data;
  R.t = data.start + 3600;
  R.open.clear();
  renderReports(main);
}

/** Kediden "dünün raporu" ile açılış. */
function reportsNav(page) {
  if (page === "reports-yesterday") { R.kind = "day"; R.t = Date.now() / 1000 - 86400; }
}

function flash(main, s) {
  R.note = s; renderReports(main);
  setTimeout(() => { if (R.note === s) { R.note = ""; renderReports(main); } }, 2000);
}

// MARK: Kartı resme çevirme (2x PNG). Yazı tipi ve kedi resmi içine gömülür; dışarıya hiçbir şey gönderilmez.

async function dataUrl(path, type) {
  const buf = await (await fetch(path)).arrayBuffer();
  let bin = ""; const b = new Uint8Array(buf);
  for (let i = 0; i < b.length; i += 0x8000) bin += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000));
  return `data:${type};base64,${btoa(bin)}`;
}

let CAPTURE_CSS = null;
async function captureCss() {
  if (CAPTURE_CSS) return CAPTURE_CSS;
  let css = (await (await fetch("tonti.css")).text()) + "\n" + (await (await fetch("reports.css")).text());
  for (const w of ["Regular", "Medium", "SemiBold", "Bold"]) {
    css = css.replace(`url(fonts/JetBrainsMono-${w}.ttf)`, `url(${await dataUrl(`fonts/JetBrainsMono-${w}.ttf`, "font/ttf")})`);
  }
  css = css.replace(/:root\[data-theme="dark"\]/g, '.cap[data-theme="dark"]').replace(/:root/g, ".cap");
  CAPTURE_CSS = css;
  return css;
}

async function cardPng(d) {
  const css = await captureCss();
  const host = document.createElement("div");
  host.className = "cap";
  host.dataset.theme = d.dark ? "dark" : "light";
  host.style.cssText = "position:fixed;left:-20000px;top:0;padding:16px;background:var(--paper);color:var(--ink);font-family:var(--mono);display:inline-block";
  host.innerHTML = `<style>${css}</style>${reportCard(d, false)}`;
  const cat = await dataUrl("img/cat/content.png", "image/png");
  host.querySelectorAll("img").forEach(i => i.setAttribute("src", cat));
  document.body.appendChild(host);
  await document.fonts.ready;
  // Gölge (3 pt) ve yazı tipi yuvarlaması için pay: kartın alt kenarı kesilmesin.
  const w = Math.ceil(host.getBoundingClientRect().width) + 4, h = Math.ceil(host.getBoundingClientRect().height) + 20;
  host.style.width = w + "px"; host.style.height = h + "px"; host.style.boxSizing = "border-box";
  host.style.position = "static"; host.style.left = "0";
  const xhtml = new XMLSerializer().serializeToString(host);
  host.remove();
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><foreignObject width="100%" height="100%">${xhtml}</foreignObject></svg>`;
  const img = new Image();
  img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
  await img.decode();
  const c = document.createElement("canvas");
  c.width = w * 2; c.height = h * 2;
  const ctx = c.getContext("2d");
  ctx.imageSmoothingEnabled = false;
  ctx.scale(2, 2);
  ctx.drawImage(img, 0, 0);
  return await new Promise(r => c.toBlob(r, "image/png"));
}

/** Tıklama bu sayfaya aitse true döner (iş arkada yapılır); değilse false. */
function reportsClick(e, main) {
  const t = e.target.closest("[data-kind],[data-rshift],[data-act],[data-open],[data-close]");
  if (!t) return false;
  reportsAct(t, main);
  return true;
}

async function reportsAct(t, main) {
  const ds = t.dataset, d = R.data;
  if (ds.kind) {
    // Tür değişince bakılan an korunur: dönem bugünü içeriyorsa bugün, değilse dönemin başı.
    const now = Date.now() / 1000;
    R.t = d.start <= now && now < d.end ? now : Math.min(d.start + 3600, now);
    R.kind = ds.kind; loadReports(main);
  } else if (ds.rshift) {
    if (ds.rshift === "1" && !d.canNext) return;
    loadReports(main, Number(ds.rshift));
  } else if (ds.open) { R.open.add(ds.open); renderReports(main); }
  else if (ds.close) { R.open.delete(ds.close); renderReports(main); }
  else if (ds.act === "md") { await navigator.clipboard.writeText(markdown(d)); flash(main, T.t("Kopyalandı")); }
  else if (ds.act === "copy") {
    const blob = await cardPng(d);
    await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
    flash(main, T.t("Kopyalandı"));
  } else if (ds.act === "save") {
    const blob = await cardPng(d);
    const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
    if (await invoke("save_png", { name: `Meowly ${periodLabel(d)}.png`, data: bytes })) flash(main, T.t("Kaydedildi"));
  }
}
