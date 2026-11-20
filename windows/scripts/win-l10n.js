// Windows'a uyarlanmış metinlerin İngilizcesi (Mac'te "Mac", "Keychain", "macOS izni" geçen metinlerin Windows hâli).
// Anahtarlar settings.js'deki TXT nesnesinden alınır; çıktı: windows/src-tauri/l10n_win_en.json
// Kullanım: node windows/scripts/win-l10n.js
const fs = require("fs");
const path = require("path");
const W = path.join(__dirname, "..");
const st = fs.readFileSync(path.join(W, "ui/settings.js"), "utf8");
const TXT = {};
for (const m of st.match(/const TXT = \{([\s\S]*?)\n\};/)[1].matchAll(/(\w+):\s*"((?:[^"\\]|\\.)*)"/g)) TXT[m[1]] = JSON.parse('"' + m[2] + '"');
const fromTxt = {
  starter: "When the 5-hour window ends, sends Haiku a one-line request to start the next window early. It gives no extra limit, it only decides when the window starts. Each request uses very few tokens. The request isn't saved to chat history, so it doesn't show up in the Claude app.",
  commits: "Commit titles you make in the project folder during a session go into the diary and reports. Only your own commits are read and nothing leaves your computer.",
  logIntro: "Tonti notes what you do on the computer to fill your diary and reports. Everything stays on this computer and is never sent anywhere.",
  logTitles: "Shows which file or document you worked on (like \"Report.docx - Word\"). Only the front window's name is read; it never looks at your keys or screen. Browser titles are not stored.",
  logSites: "Keeps how long you stay on each site by domain only (like youtube.com). Page titles, full URLs and contents are not stored; private windows are skipped.",
  logFiles: "Counts changes per hour in the project folders you use with AI tools. File names and contents are not stored.",

  ghToken: "GitHub > Settings > Developer settings > Fine-grained tokens: select only this profile repo and grant Contents: Read and write. The token stays only in this computer's Windows Credential Manager.",
  claudeLimit: "Claude limit percentage: when Claude Code is installed, it is measured with its command at most every 5 minutes while you use Claude on this computer. Even if you don't, it is read from Anthropic's usage address every 15 minutes while you are at the computer (so it stays current when you use Claude on your phone or in the browser; this read doesn't use your limit). \"Sync\" reads it right away. Without Claude Code, the address is asked at most once a minute. The last value is shown in between; windows that reset show 100% right away.",
  claudeSrc: "Claude: ~/.claude logs (tokens), Claude Code status line / command or Anthropic usage endpoint (limit)",
};
const out = {};
for (const [k, en] of Object.entries(fromTxt)) {
  if (!TXT[k]) throw new Error("TXT." + k + " yok");
  out[TXT[k]] = en;
}
Object.assign(out, {
  "Bu bilgisayarda yapay zekâ aracı bulunamadı": "No AI tools found on this computer",
  "Claude Code, Codex, Gemini CLI, Antigravity, OpenCode, Kimi ya da Cline kullanmaya başlayınca Tonti burayı kendiliğinden doldurur. Hangisinin kurulu olmadığını Ayarlar → Sağlayıcılar'da görebilirsin.": "Once you start using Claude Code, Codex, Gemini CLI, Antigravity, OpenCode, Kimi or Cline, Tonti fills this in by itself. See which ones are missing in Settings → Providers.",
  "SAĞLAYICILAR": "PROVIDERS",
  "Son güncelleme (%@) kurulamadı. Siteden indirip kurabilirsin; ayarların ve kayıtların silinmez.": "The last update (%@) could not be installed. You can download it from the site and install it; your settings and records are kept.",
  "Aç": "Open",
  "Çık": "Quit",
  "Bu sağlayıcı Windows sürümünde henüz okunmuyor.": "This provider is not read on Windows yet.",
  "Windows'a giriş yapınca uygulama kendiliğinden açılır.": "Opens automatically when you sign in to Windows.",
  "Görev çubuğunda göster.": "Show in the taskbar.",
  "Soldaki kısım menüyü açar, sağdaki kutu Tonti'yi kutuya koyar ya da masaüstüne çıkarır.": "The left part opens the menu; the box on the right puts Tonti in the box or back on the desktop.",
  "Görev çubuğu": "Taskbar",
  "Claude Code oturumu bulunamadı": "Claude Code session not found",
  "Windows bildirimleri kapalı; Meowly'nin bildirimleri görünmez. Windows Ayarları > Sistem > Bildirimler'den açabilirsin.": "Windows notifications are off, so Meowly's notifications won't show. You can turn them on in Windows Settings > System > Notifications.",
  "Pencere, menü ve görev çubuğundaki gösterge birlikte değişir.": "The window, the menu and the taskbar indicator change together.",
  "Görev çubuğunda ve popover'da görünecek sağlayıcıları seç.": "Choose which providers appear in the taskbar and popover.",
  "Masaüstünde yaşayan kedin Tonti çalışmanı not alır, raporunu hazırlar; yapay zekâ limitlerin görev çubuğunda.": "Tonti, the cat on your desktop, takes notes of your work and prepares your reports; your AI limits sit in the taskbar.",
  "Görev çubuğunu, kediyi ve ayarları kısaca gösteren beş adımlık tur.": "A five-step tour of the taskbar, the cat and the settings.",
  // Mac'te "on your Mac": Windows'ta bilgisayar (Türkçesi zaten "bilgisayarında").
  "Tonti not aldı · bilgisayarında, sohbetler okunmadan": "Noted by Tonti · on your computer, chats never read",
  // Tanıtım turu (Mac: Tour.swift) — menü çubuğu ve "Mac" geçen iki adım.
  "Görev çubuğunda 5 saatlik ve haftalık kalan limitini görürsün. Soldaki kısma basınca ayrıntılar açılır; sağdaki kutuya basınca ben kutuya girerim.": "The taskbar shows your 5-hour and weekly limits. Click the left part for details; click the box on the right and I'll hop inside.",
  "Hepsi bu bilgisayarda kalır": "It all stays on this computer",
  "Oturum süresi dolmuş; Gemini CLI'ı bir kez çalıştırınca yenilenir": "Session expired; it renews once you run the Gemini CLI",
  "WINDOWS AYARLARI": "WINDOWS SETTINGS",
});
fs.writeFileSync(path.join(W, "src-tauri/l10n_win_en.json"), JSON.stringify(out, null, 1));
console.log("Windows metni:", Object.keys(out).length);
