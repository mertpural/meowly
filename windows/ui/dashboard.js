// Ana pencere iskeleti (Mac: TontiRoot, LimitMini, SettingsPage). Sayfalar sırayla dolduruluyor.
const PAGES = [["diary", "Günlük", "✎"], ["reports", "Raporlar", "▤"], ["usage", "Kullanım", "◔"], ["settings", "Ayarlar", "⚙"]];
const SETTINGS_TABS = [["appearance", "Tonti"], ["providers", "Sağlayıcılar"], ["menubar", "Görev çubuğu"],
  ["automation", "Bildirim ve otomasyon"], ["computer", "Bilgisayar günlüğü"], ["sync", "Senkronizasyon"], ["github", "GitHub kartı"], ["about", "Hakkında"]];
const KEYS = ["senin kedin, senin günlüğün", "5 saatlik", "Haftalık", "Tonti bu sayfayı hazırlıyor."]
  .concat(PAGES.map(p => p[1]), SETTINGS_TABS.map(t => t[1]), USAGE_KEYS, DIARY_KEYS, REPORT_KEYS, SET_KEYS);

let S = null;
// Geliştirme: "#usage@y=800" sayfayı aşağı kaydırılmış açar (ekran görüntüsü için).
let [page, devScroll] = (location.hash.slice(1) || "diary").split("@y=");

/** Eski sayfa adları (kedi, menü, bildirimler) buraya yönlenir. */
function pageOf(p) {
  if (p === "diary" || p === "history") return "diary";
  if (p === "usage") return p;
  if (p.startsWith("reports")) return "reports";
  return "settings";
}

function upper(s) { return s.toLocaleUpperCase(S && S.en ? "en" : "tr"); }

function limitMini() {
  const rows = S.providers.filter(p => (p.id === "claude" || p.id === "codex") && p.limits && (p.limits.five || p.limits.week));
  if (!rows.length) return "";
  const bar = (title, r) => `<div class="r"><div class="h"><span>${T.t(title)}</span><b>${r.pct}</b></div>
    <div class="bar"><i style="width:${r.left * 100}%;background:${r.left < 0.15 ? "var(--pink)" : "var(--ink)"}"></i></div></div>`;
  return `<div class="px mini" data-page="usage">${rows.map(p => `<div class="col" style="gap:4px">
    <span class="n">${p.id.toUpperCase()}</span>
    ${p.limits.five ? bar("5 saatlik", p.limits.five) : ""}${p.limits.week ? bar("Haftalık", p.limits.week) : ""}</div>`).join("")}</div>`;
}

function soon(title) {
  return `<div class="soon"><img class="px-img" src="img/cat/sleep.png"><b>${T.t(title)}</b><span>${T.t("Tonti bu sayfayı hazırlıyor.")}</span></div>`;
}

function render() {
  setTheme(S.dark);
  const cur = pageOf(page);
  document.getElementById("side").innerHTML = `
    <div class="brand"><img class="px-img" src="img/cat/content.png"><div><b>MEOWLY</b><small>${T.t("senin kedin, senin günlüğün")}</small></div></div>
    <nav class="nav">${PAGES.map(([id, t, g]) => `<a class="px ${id === cur ? "on" : ""}" data-page="${id}"><span class="g">${g}</span>${upper(T.t(t))}</a>`).join("")}</nav>
    <span class="spacer"></span>
    ${limitMini()}`;
  const main = document.getElementById("main");
  main.classList.toggle("noscroll", cur === "diary" || cur === "settings");
  if (cur === "diary") {
    if (D.data) renderDiary(main); else main.innerHTML = "";
    loadDiary(main);
  } else if (cur === "reports") {
    reportsNav(page);
    if (R.data) renderReports(main); else main.innerHTML = "";
    loadReports(main);
  } else if (cur === "settings") {
    const tab = SETTINGS_TABS.some(t => t[0] === page) ? page : "appearance";
    if (SX.data) renderSettings(main, tab); else main.innerHTML = "";
    loadSettings(main, tab);
  } else if (cur === "usage") {
    if (U.data) renderUsage(main); else main.innerHTML = "";
    loadUsage(main).then(() => { if (devScroll) { main.scrollTop = Number(devScroll); devScroll = null; } });
  } else {
    main.innerHTML = soon(PAGES.find(p => p[0] === cur)[1]);
  }
}

document.addEventListener("mousemove", e => { if (pageOf(page) === "usage" && U.data) usageHover(e, document.getElementById("main")); });

document.addEventListener("click", e => {
  if (pageOf(page) === "usage" && usageClick(e, document.getElementById("main"))) return;
  if (pageOf(page) === "diary" && diaryClick(e, document.getElementById("main"))) return;
  if (pageOf(page) === "reports" && reportsClick(e, document.getElementById("main"))) return;
  if (pageOf(page) === "settings" && settingsClick(e, document.getElementById("main"))) return;
  const a = e.target.closest("[data-page]");
  if (!a) return;
  page = a.dataset.page === "settings" ? "appearance" : a.dataset.page;
  render();
});

async function load() {
  S = await invoke("get_state");
  await T.sync(S);
  // Bir alana yazarken (kedinin adı, GitHub) yeniden çizip odağı kaçırma.
  if (document.activeElement && document.activeElement.tagName === "INPUT") return;
  render();
}

(async () => {
  await T.load();
  await load();
  listen("meowly-state", load);
  listen("meowly-page", e => { [page, devScroll] = e.payload.split("@y="); render(); });
  // Tanıtım turu: gösterilecek öğenin pencere içindeki yeri (Mac: tourAnchor). Yalnızca konum, içerik yok.
  listen("meowly-tour-anchor", e => {
    const sel = { "sidebar.nav": ".nav", "settings.tabs": ".settings .tabs", "computer.privacy": ".privacy" }[e.payload];
    const el = sel && document.querySelector(sel);
    if (!el) return;
    const r = el.getBoundingClientRect();
    invoke("tour_anchor", { x: r.left, y: r.top, w: r.width, h: r.height });
  });
})();

document.addEventListener("input", e => { if (pageOf(page) === "settings") settingsInput(e, document.getElementById("main")); });
document.addEventListener("keydown", e => { if (pageOf(page) === "settings") settingsKey(e, document.getElementById("main")); });
document.addEventListener("change", e => { if (pageOf(page) === "settings") settingsChange(e, document.getElementById("main")); });