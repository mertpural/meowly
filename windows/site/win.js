// Meowly sitesi: Windows indirmesi. Sitenin kendi dosyalarına dokunmadan eklenir (index.html'de tek <script> satırı).
// İndirme bölümüne Windows düğmesini ve kurulum adımlarını koyar; Windows'tan gelen ziyaretçide Windows düğmesi öne geçer.
// Sürüm ve boyut /latest-windows.json'dan okunur. Dış kütüphane yok, hiçbir şey gönderilmez.
(() => {
  const TR = {
    btn: "↓ Windows için indir",
    btnMac: "↓ Mac için indir",
    meta: "Windows 10/11 · 64 bit · yaklaşık %s MB · sürüm %v",
    metaNoV: "Windows 10/11 · 64 bit",
    warn: "Tarayıcı “zararlı olabilir” derse: ⋯ → Sakla → Yine de sakla. Windows uyarırsa: Ek bilgi → Yine de çalıştır.",
    title: "Windows'ta", macTitle: "Mac'te",
    s1t: "Kur", s1p: "<b>Meowly-Setup.exe</b>'yi çalıştır. Yönetici izni istemez, yalnızca senin hesabına kurulur.",
    s2t: "Uyarıyı geç", s2p: "Tarayıcı indirirken uyarırsa: <b>⋯ → Sakla → Yine de sakla</b>. Windows açarken uyarırsa: <b>Ek bilgi → Yine de çalıştır</b>. Meowly henüz ücretli kod imzasıyla imzalanmadı; bu normal.",
    s3t: "Görev çubuğu", s3p: "Limitler saatin hemen yanında durur; kedin masaüstünde yaşar. Tıklayınca günlüğün açılır.",
    s4t: "Kaldırmak", s4p: "<b>Ayarlar → Uygulamalar</b>'dan kaldırılır. Kayıtların <code>%APPDATA%\\Meowly</code> içinde kalır; istersen onu da sil.",
    hash: "İndirilen dosya değişmemiş mi? PowerShell'de çalıştır, sonuç şununla aynı olmalı:",
  };
  const EN = {
    btn: "↓ Download for Windows",
    btnMac: "↓ Download for Mac",
    meta: "Windows 10/11 · 64-bit · about %s MB · version %v",
    metaNoV: "Windows 10/11 · 64-bit",
    warn: "If the browser says it may be harmful: ⋯ → Keep → Keep anyway. If Windows warns: More info → Run anyway.",
    title: "On Windows", macTitle: "On Mac",
    s1t: "Install", s1p: "Run <b>Meowly-Setup.exe</b>. No admin rights needed; it installs for your account only.",
    s2t: "Getting past the warning", s2p: "If the browser warns while downloading: <b>⋯ → Keep → Keep anyway</b>. If Windows warns when opening: <b>More info → Run anyway</b>. Meowly isn't signed with a paid code-signing certificate yet; that's expected.",
    s3t: "Taskbar", s3p: "Your limits sit right next to the clock; your cat lives on the desktop. Click it to open your diary.",
    s4t: "Removing it", s4p: "Uninstall from <b>Settings → Apps</b>. Your records stay in <code>%APPDATA%\\Meowly</code>; delete that too if you like.",
    hash: "Is the download unchanged? Run this in PowerShell; the result should match:",
  };
  const isWin = /Windows/i.test((navigator.userAgentData && navigator.userAgentData.platform) || navigator.userAgent) || /[?&]os=win\b/.test(location.search);
  const sec = document.getElementById("download");
  if (!sec) return;
  const fin = sec.querySelector(".final");
  const steps = sec.querySelector(".steps");
  const macBtn = document.getElementById("dlbtn");
  let info = null;

  const winBtn = document.createElement("a");
  winBtn.className = "btn big" + (isWin ? " fill" : "");
  winBtn.id = "dlbtn-win";
  winBtn.href = "/download/Meowly-Setup.exe";
  winBtn.addEventListener("click", () => setTimeout(() => box.scrollIntoView({ behavior: "smooth", block: "start" }), 300));
  const meta = document.createElement("p");
  meta.className = "winmeta";
  const warn = document.createElement("p");
  warn.className = "winmeta winwarn";
  const btns = document.createElement("div");
  btns.className = "winbtns";
  if (fin && macBtn) {
    macBtn.replaceWith(btns);
    if (isWin) { btns.append(winBtn, macBtn); macBtn.classList.remove("fill"); } else btns.append(macBtn, winBtn);
    btns.after(meta); meta.after(warn);
  }

  // Mac adımlarının üstüne de başlık: iki sistemin adımları karışmasın.
  const macTitle = document.createElement("h3");
  macTitle.className = "wt mt";
  if (steps) steps.before(macTitle);

  const box = document.createElement("div");
  box.className = "winsteps";
  box.id = "windows";
  if (isWin && steps) macTitle.before(box); else (steps || fin || sec).after(box);   // Windows ziyaretçisinde Windows adımları önce gelir

  const css = document.createElement("style");
  css.textContent = `
    #download .winbtns { display: flex; gap: 12px; flex-wrap: wrap; justify-content: center; }
    #download .winmeta { margin-top: 10px; font-size: 13px; opacity: .75; }
    #download .winsteps { margin-top: 28px; }
    #download .winsteps h3.wt, #download h3.wt.mt { margin: 0 0 12px; }
    #download .winsteps .hash { margin-top: 14px; font-size: 13px; }
    #download .winsteps .hash code { display: block; margin-top: 6px; padding: 8px 10px; overflow-x: auto; white-space: nowrap; }
  `;
  document.head.appendChild(css);

  const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  function render() {
    const L = document.documentElement.lang === "tr" ? TR : EN;
    macTitle.textContent = L.macTitle;
    winBtn.textContent = L.btn;
    warn.textContent = L.warn;
    if (macBtn && isWin) macBtn.textContent = L.btnMac;
    meta.textContent = info ? L.meta.replace("%s", (info.size / 1048576).toFixed(1)).replace("%v", info.version) : L.metaNoV;
    const st = (n, t, p) => `<div class="px"><div class="n">${n}</div><h3>${t}</h3><p>${p}</p></div>`;
    box.innerHTML = `<h3 class="wt">${L.title}</h3><div class="steps">${st("01", L.s1t, L.s1p)}${st("02", L.s2t, L.s2p)}${st("03", L.s3t, L.s3p)}${st("04", L.s4t, L.s4p)}</div>`
      + (info ? `<p class="hash">${L.hash}<code>(Get-FileHash .\\Meowly-Setup.exe).Hash</code><code>${esc(info.sha256.toUpperCase())}</code></p>` : "");
  }
  // Site dili değişince (<html lang>) metinler de değişir.
  new MutationObserver(render).observe(document.documentElement, { attributes: true, attributeFilter: ["lang"] });
  render();
  fetch("/latest-windows.json", { cache: "no-store" }).then(r => r.ok ? r.json() : null).then(j => { if (j && j.version) { info = j; render(); } }).catch(() => {});
})();
