// Günlük sayfası (Mac: Tonti/TontiUI.swift → DiaryPage, EntryRow, HeatGrid; Tonti/ComputerUI.swift → ComputerDayView, ComputerSide).
const DIARY_KEYS = ["Günlük", "Tonti yapay zekâ oturumlarını, commit'lerini ve bilgisayarda ne yaptığını buraya yazar. Sohbetlerin ve ekranın okunmaz.",
  "BUGÜN", "YAPAY ZEKÂ", "BİLGİSAYAR", "Bu gün için not yok.", "Claude ya da Codex ile çalıştığında Tonti buraya yazar.",
  "BAŞKA DEPOLARDA", "O GÜN", "çalışma", "oturum", "commit", "token", "SON 15 HAFTA", "BU HAFTA",
  "Bu gün için bilgisayar kaydı yok.", "Tonti, bilgisayar günlüğü açıkken uygulamaları not alır.", "BİLGİSAYARDA",
  "SAAT SAAT", "daha az göster", "+%lld daha", "DOSYA DEĞİŞİKLİKLERİ"];

const D = { day: null, data: null, gen: 0, open: new Set() };

function upperL(s) { return s.toLocaleUpperCase(D.data && D.data.en ? "en" : "tr"); }

function diaryDayText() {
  const d = new Date(D.data.day * 1000);
  const l = D.data.en ? "en" : "tr";
  return new Intl.DateTimeFormat(l, { day: "numeric", month: "long", year: "numeric" }).format(d) + ", " + new Intl.DateTimeFormat(l, { weekday: "long" }).format(d);
}

function entryRow(e) {
  const tags = e.providers.map(p => `<span class="tag ${p === "claude" ? "c" : "o"}">${esc(p.toUpperCase())}</span>`).join("");
  const body = e.commits.length
    ? `<div class="commits">${e.commits.map(c => `<div class="cm"><b>✓</b><span>${esc(c)}</span></div>`).join("")}</div>`
    : `<span class="range">${esc(e.range)}</span>`;
  return `<div class="entry"><div class="when"><b>${e.start}</b><span>${esc(e.dur)}</span></div>
    <div class="px lift ecard"><div class="row eh"><b>${esc(e.project)}</b><span class="spacer"></span>${tags}</div>${body}<span class="tok">${esc(e.tokens)}</span></div></div>`;
}

function otherCard() {
  const o = D.data.others;
  if (!o.length) return "";
  return `<div class="px lift ecard other"><span class="lbl">${T.t("BAŞKA DEPOLARDA")}</span>${o.map(r => `<div class="orepo">
    <div class="row"><b>${esc(r.name)}</b><span class="spacer"></span><span class="sub">${esc(r.count)}</span></div>
    ${r.commits.map(c => `<div class="oc"><span class="sub">${c.time}</span><span>${esc(c.subject)}</span></div>`).join("")}
    ${r.more ? `<span class="sub more">${esc(r.more)}</span>` : ""}</div>`).join("")}</div>`;
}

function heatGrid() {
  const h = D.data.heat;
  const mx = Math.max(1, ...h.map(x => x.t));
  const lead = h.length ? (h[0].wd + 6) % 7 : 0; // ilk sütun pazartesiden başlasın
  const cells = Array(lead).fill(null).concat(h);
  // Sütun sütun yerleşen ızgara (haftalar sütun, günler satır): dar alanda asla alt satıra kaymaz.
  const out = cells.map(x => {
    if (!x) return `<i class="hgx empty"></i>`;
    const lv = x.t === 0 ? 0 : Math.min(4, 1 + Math.floor(x.t / mx * 3.999));
    const sel = Math.abs(x.ts - D.data.day) < 1 ? " sel" : "";
    return `<i class="hgx${sel}" style="background:var(--heat${lv})" data-day="${x.ts}" title="${esc(x.tip)}"></i>`;
  }).join("");
  return `<div class="hg">${out}</div>`;
}

// MARK: Bilgisayar görünümü (Mac: ComputerDayView, ComputerSide)

function computerDay(c) {
  if (!c.apps.length) return `<div class="empty"><img class="px-img" src="img/cat/sleep.png"><b>${T.t("Bu gün için bilgisayar kaydı yok.")}</b><span>${T.t("Tonti, bilgisayar günlüğü açıkken uygulamaları not alır.")}</span></div>`;
  const mx = Math.max(60, ...c.hours);
  const bars = c.hours.map((v, h) => `<div class="hb"><i style="height:${Math.max(3, 44 * v / mx)}px;background:${v > 0 ? "var(--pink)" : "var(--line)"}"></i><span>${h % 3 === 0 ? String(h).padStart(2, "0") : "&nbsp;"}</span></div>`).join("");
  const strip = `<div class="px lift ccard"><span class="lbl">${T.t("SAAT SAAT")}</span><div class="hours">${bars}</div></div>`;
  const rows = c.apps.map(a => {
    const open = D.open.has(a.key);
    const det = a.details.slice(0, open ? 8 : 2).map(x => `<div class="cdet"><span>${esc(x.t)}</span><span class="sub">${esc(x.dur)}</span></div>`).join("");
    const more = a.details.length > 2 ? `<a class="more" data-capp="${esc(a.key)}">${open ? T.t("daha az göster") : esc(T.f("+%lld daha", a.details.length - 2))}</a>` : "";
    return `<div class="px lift ccard">
      <div class="row capp">${a.icon ? `<img src="${a.icon}">` : `<i class="noicon"></i>`}<div class="col"><b>${esc(a.name)}</b><span class="sub">${esc(a.category)}</span></div>
        <span class="spacer"></span><b class="cdur">${esc(a.dur)}</b></div>
      <div class="cbar"><i style="width:${a.frac * 100}%"></i></div>${det}${more}</div>`;
  }).join("");
  return `<div class="cday">${strip}${rows}</div>`;
}

function computerSide(c) {
  let out = `<div class="px lift scard"><span class="lbl">${T.t("BİLGİSAYARDA")}</span><b class="big">${esc(c.active)}</b>
    ${c.categories.map(x => `<div class="crow"><span>${esc(x.name)}</span><span class="sub">${esc(x.dur)}</span></div>`).join("")}</div>`;
  // Mac: ComputerSide → TAKVİM (başlık ve saat; raporlara yalnızca toplam süre girer).
  if (c.meetings && c.meetings.length) out += `<div class="px lift scard"><span class="lbl">${T.t("TAKVİM")}</span>
    ${c.meetings.map(m => `<div class="mt"><b>${esc(m.title)}</b><span class="sub">${esc(m.range)}</span></div>`).join("")}</div>`;
  if (c.files.length) out += `<div class="px lift scard"><span class="lbl">${T.t("DOSYA DEĞİŞİKLİKLERİ")}</span>
    ${c.files.map(f => `<div class="crow"><b>${esc(f.project)}</b><span class="sub">${f.count}</span></div>`).join("")}</div>`;
  return out;
}

function dstat(v, k) { return `<div class="st"><b>${esc(String(v))}</b><span>${esc(upperL(T.t(k)))}</span></div>`; }

function renderDiary(main) {
  const d = D.data;
  const pc = d.mode === "pc";
  let left, side;
  if (pc) {
    left = computerDay(d.pc);
    side = computerSide(d.pc);
  } else {
    left = !d.entries.length && !d.others.length
      ? `<div class="empty"><img class="px-img" src="img/cat/sleep.png"><b>${T.t("Bu gün için not yok.")}</b><span>${T.t("Claude ya da Codex ile çalıştığında Tonti buraya yazar.")}</span></div>`
      : `<div class="timeline">${d.entries.map(entryRow).join("")}${otherCard()}</div>`;
    side = `<div class="px lift scard"><span class="lbl">${T.t("O GÜN")}</span><div class="sgrid">
        ${dstat(d.stats.work, "çalışma")}${dstat(d.stats.sessions, "oturum")}${dstat(d.stats.commits, "commit")}${dstat(d.stats.tokens, "token")}</div></div>
      <div class="px lift scard"><span class="lbl">${T.t("SON 15 HAFTA")}</span>${heatGrid()}</div>
      ${d.week.length ? `<div class="px lift scard"><span class="lbl">${T.t("BU HAFTA")}</span>${d.week.map(w => `<div class="wk">
        <div class="row"><b>${esc(w.name)}</b><span class="spacer"></span><span class="sub">${esc(w.dur)}</span></div>
        <i style="width:${w.frac * 100}%"></i></div>`).join("")}</div>` : ""}`;
  }
  main.innerHTML = `<div class="diary">
    <div class="dhead"><div class="ptitle"><b>${T.t("Günlük")}</b><span>${T.t("Tonti yapay zekâ oturumlarını, commit'lerini ve bilgisayarda ne yaptığını buraya yazar. Sohbetlerin ve ekranın okunmaz.")}</span></div>
      <button class="btn px" data-shift="-1">‹</button><b class="dday">${esc(diaryDayText())}</b><button class="btn px" data-shift="1">›</button>
      <button class="btn px ${d.isToday ? "on" : ""}" data-today="1">${T.t("BUGÜN")}</button></div>
    ${noToolsBanner(typeof S !== "undefined" ? S : null)}
    ${d.nudge ? `<div class="px lift nudge"><img class="px-img" src="img/cat/idle_2.png"><div class="col" style="gap:3px;flex:1"><b>${T.t("Tonti daha fazlasını not alabilir")}</b><span>${esc(d.nudge)}</span></div>
      <button class="btn px on" data-nudge="go">${T.t("AYARLARA GİT")}</button><button class="btn px" data-nudge="hide">✕</button></div>` : ""}
    <div class="modes"><button class="btn px ${!pc ? "on" : ""}" data-mode="ai">${T.t("YAPAY ZEKÂ")}</button><button class="btn px ${pc ? "on" : ""}" data-mode="pc">${T.t("BİLGİSAYAR")}</button></div>
    <div class="dbody"><div class="dleft">${left}</div><div class="dside">${side}</div></div>
  </div>`;
}

async function loadDiary(main) {
  const g = ++D.gen;
  const data = await invoke("diary_page", { day: D.day ?? Date.now() / 1000 });
  if (g !== D.gen) return;
  D.data = data;
  D.day = data.day;
  renderDiary(main);
}

function diaryClick(e, main) {
  const t = e.target.closest("[data-shift],[data-today],[data-mode],[data-day],[data-capp],[data-nudge]");
  if (!t) return false;
  const ds = t.dataset;
  if (ds.nudge === "go") { page = "computer"; render(); return true; }
  if (ds.nudge === "hide") { invoke("set_setting", { key: "log_nudge_hidden", value: true }); D.data.nudge = null; renderDiary(main); return true; }
  if (ds.capp) {
    if (D.open.has(ds.capp)) D.open.delete(ds.capp); else D.open.add(ds.capp);
    const sc = main.querySelector(".dleft").scrollTop;
    renderDiary(main);
    main.querySelector(".dleft").scrollTop = sc;
  } else if (ds.shift) {
    const next = D.day + Number(ds.shift) * 86400;
    if (next > Date.now() / 1000) return true;
    D.day = next; loadDiary(main);
  } else if (ds.today) { D.day = null; loadDiary(main); }
  else if (ds.mode) { D.data.mode = ds.mode; invoke("set_diary_mode", { mode: ds.mode }); renderDiary(main); }
  else if (ds.day) { D.day = Number(ds.day); loadDiary(main); }
  return true;
}
