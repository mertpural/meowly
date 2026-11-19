// Açılır pencere (Mac: Views.swift → PopoverView, TabBar, OverviewCard, ProviderDetail).
const KEYS = ["Güncellendi %@", "Tümü", "Günlük", "Raporlar", "5 SAATLİK", "HAFTALIK", "Bugün %@", " · cache dahil",
  "Hiçbir sağlayıcı seçili değil. Ayarlardan aç.", "Veri alınamadı. Ayrıntı için Sağlayıcılar sayfasına bak.",
  "Claude girişi süresi dolmuş: Terminal'de claude yazıp /login yap.",
  "Limit en son %@ tarihinde ölçüldü; birkaç dakika içinde yeniden ölçülür.", "kaldı", "TOKEN", "Son 7 gün", "bugün",
  "Sağlıklı", "Dikkat", "Kritik", "Bitti", "Çok hızlı", "Hızlı", "Rahat", "Dengeli", "Veri alınamadı."];

let S = null;

function overviewCard(p) {
  const l = p.limits;
  const metric = (title, r, raw) => !r ? "" : `
    <div class="metric">
      <div class="mh"><span class="title">${raw ? esc(title) : T.t(title)}</span><span class="pct">${r.pct}</span></div>
      ${paceBar(r)}
      <div class="reset">${esc(r.reset)}</div>
    </div>`;
  const rows = l && (l.five || l.week);
  const other = (l && l.other) || [];
  return `<div class="px card ov">
    <div class="row top">${logo(p.id, 22)}<span class="pname">${p.name}</span><span class="spacer"></span>
      ${p.hasTokens ? `<span class="today">${esc(T.f("Bugün %@", p.today) + (S.includeCache ? T.t(" · cache dahil") : ""))}</span>` : ""}</div>
    ${rows ? `<div class="metrics">${metric("5 SAATLİK", l.five)}${metric("HAFTALIK", l.week)}</div>` : ""}
    ${other.map(r => `<div class="metrics">${metric(r.label, r, true)}</div>`).join("")}
    ${!rows && !other.length ? `<div class="note">${T.t("Veri alınamadı. Ayrıntı için Sağlayıcılar sayfasına bak.")}</div>` : ""}
    ${p.authExpired ? `<div class="warn">${T.t("Claude girişi süresi dolmuş: Terminal'de claude yazıp /login yap.")}</div>`
      : p.stale ? `<div class="warn">${esc(T.f("Limit en son %@ tarihinde ölçüldü; birkaç dakika içinde yeniden ölçülür.", p.stale))}</div>` : ""}
    ${p.footnotes.map(f => `<div class="note">${esc(f)}</div>`).join("")}
  </div>`;
}

function detail(p) {
  const l = p.limits;
  const mcard = (title, icon, r) => {
    if (!r) return "";
    const st = statusWord(r.left), pc = pace(r);
    return `<div class="px card mcard">
      <div class="mh">${icon}<span>${T.t(title)}</span>${chip(T.t(st[0]), st[1])}</div>
      <div class="big"><b>${r.pct}</b><span>${T.t("kaldı")}</span></div>
      ${paceBar(r)}
      <div class="mfoot"><span>${esc(r.remaining ?? "")}</span><span class="spacer"></span>${pc ? chip(T.t(pc[0]), pc[1]) : ""}</div>
    </div>`;
  };
  const rows = l && (l.five || l.week);
  const anyRows = rows || (l && ((l.other || []).length || (l.pools || []).length));
  const pools = (l && l.pools) || [];
  const mx = Math.max(1, ...p.daily);
  const bars = p.daily.map((v, i) => `<i style="height:${Math.max(3, 34 * v / mx)}px;background:${p.color};opacity:${i === p.daily.length - 1 ? 1 : 0.45}"></i>`).join("");
  return `
    <div class="row det-head">${logo(p.id, 26)}<span class="pname">${p.name}</span></div>
    ${p.authExpired ? `<div class="warn">${T.t("Claude girişi süresi dolmuş: Terminal'de claude yazıp /login yap.")}</div>`
      : p.stale ? `<div class="warn">${esc(T.f("Limit en son %@ tarihinde ölçüldü; birkaç dakika içinde yeniden ölçülür.", p.stale))}</div>` : ""}
    ${!anyRows ? `<div class="px card empty">${esc(p.notes[0] ?? T.t("Veri alınamadı."))}</div>` : ""}
    ${rows ? `<div class="mcards">${mcard("5 SAATLİK", ICON.bolt, l.five)}${mcard("HAFTALIK", ICON.cal, l.week)}</div>` : ""}
    ${p.footnotes.map(f => `<div class="note">${esc(f)}</div>`).join("")}
    ${pools.length ? `<div class="px card pools"><div class="mh"><span>${esc(l.poolsKind)}</span></div>
      ${pools.map(r => `<div class="lb"><div class="row"><span class="t">${esc(r.title)}</span><span class="spacer"></span><b>${r.pct}</b></div>${paceBar(r)}<span class="r">${esc(r.reset)}</span></div>`).join("")}</div>` : ""}
    ${p.hasTokens ? `<div class="px card tok">
      <div class="mh"><span>${T.t("TOKEN")}</span><span>${T.t("Son 7 gün")}</span></div>
      <div class="row" style="gap:14px;align-items:flex-end">
        <div class="val"><b>${p.today}</b><span>${T.t("bugün")}</span></div>
        <div class="bars">${bars}</div>
      </div>
    </div>` : ""}`;
}

function render() {
  setTheme(S.dark);
  const sel = S.providers.find(p => p.id === S.tab);
  const tabs = [`<button class="btn px ${!sel ? "on" : ""}" data-tab="all">${T.t("Tümü")}</button>`]
    .concat(S.providers.map(p => `<button class="btn px ${sel && sel.id === p.id ? "on" : ""}" data-tab="${p.id}">${logo(p.id, 16)}</button>`));
  let content;
  if (!S.providers.length) content = `<div class="empty">${T.t("Hiçbir sağlayıcı seçili değil. Ayarlardan aç.")}</div>`;
  else if (!sel) content = S.providers.map(overviewCard).join("");
  else content = detail(sel);
  document.getElementById("root").innerHTML = `
    <div class="row head">
      <img class="px-img" src="img/cat/content.png"><span class="name">Meowly</span><span class="spacer"></span>
      ${S.updatedAt ? `<span class="upd">${esc(T.f("Güncellendi %@", S.updatedAt))}</span>` : ""}
    </div>
    <div class="row tabs">${tabs.join("")}</div>
    <div id="content">${content}</div>
    <div class="row bottom">
      <button class="btn px" data-open="diary">✎ ${T.t("Günlük")}</button>
      <button class="btn px" data-open="reports">▤ ${T.t("Raporlar")}</button>
      <button class="btn px" data-open="appearance">${ICON.gear}</button>
      <span class="spacer"></span>
      <button class="btn px" id="refresh">${ICON.refresh}</button>
      <button class="btn px" id="quit">${ICON.power}</button>
    </div>`;
  // Gizli pencerede ekran karesi çizilmez (requestAnimationFrame çalışmaz): ölçümü doğrudan yap.
  const report = () => invoke("popover_ready", { height: document.getElementById("root").offsetHeight });
  report();
  document.fonts.ready.then(report);
}

document.addEventListener("click", e => {
  const b = e.target.closest("button");
  if (!b) return;
  if (b.dataset.tab) { S.tab = b.dataset.tab; invoke("set_tab", { tab: S.tab }); render(); }
  else if (b.dataset.open) invoke("open_dashboard", { page: b.dataset.open });
  else if (b.id === "refresh") invoke("refresh_now");
  else if (b.id === "quit") invoke("quit");
});

async function load() { S = await invoke("get_state"); await T.sync(S); render(); }

(async () => {
  await T.load();
  await load();
  listen("meowly-state", load);
  listen("meowly-reshow", load);
})();
