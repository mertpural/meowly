// Ayarlar (Mac: Dashboard.swift → SettingRow ve sayfalar; ComputerUI.swift; GitHubProfile.swift; Updater.swift → UpdateCard).
const SET_KEYS = ["%@ sürümüne güncelle", "Denetleniyor…", "İndiriliyor…", "Yeniden başlatılıyor…", "Windows bildirimleri kapalı; Meowly'nin bildirimleri görünmez. Windows Ayarları > Sistem > Bildirimler'den açabilirsin.", "WINDOWS AYARLARI", "Ayarlar", "Görünüm", "Pencere, menü ve görev çubuğundaki gösterge birlikte değişir.", "Açık", "Koyu", "Cache dahil",
  "Token sayılarına önbellekten okunan (cache) token'lar da eklenir. Kapalıyken yalnızca yeni okunan ve yazılan token'lar sayılır; sayılar daha küçük ve gerçek işe daha yakın olur.",
  "Kedinin adı", "Masaüstündeki kedin ve uygulamadaki metinler bu adı kullanır.", "KAYDET", "Tonti'nin balonları",
  "Tonti başının üstündeki balonda ne yaptığını söyler. Sohbet içeriği okunmaz; yalnızca proje adı, süre ve token sayısı kullanılır.",
  "Çalışırken not alır (proje ve süre)", "Uzun oturumda mola hatırlatır", "Günün ilk açılışında dünün özeti", "Günün token eşiklerini kutlar", "Sevilince mırlar",
  "Dil", "Arayüz dilini seç. Değişiklik hemen uygulanır.", "Sistem", "Günlükte commit'leri göster",
  "KURULUM KONTROLÜ", "Görev çubuğunda ve popover'da görünecek sağlayıcıları seç.", "Limit", "Token", "Kesin", "Tahmini", "Veri yok",
  "ÖNİZLEME", "Etiketleri göster", "5 saatlik için \"5s\", haftalık için \"H\", günlük kota için \"G\". Aynı yüzdeler olsa bile hangisi olduğu anlaşılır.",
  "5 saatlik limit", "Haftalık limit", "Görev çubuğunda göster.", "Yüzde", "Çubuk", "Çubuk + yüzde", "Halka", "Nokta", "Pil", "Segment",
  "Hangi sağlayıcıların görüneceğini Sağlayıcılar sayfasından seçersin.", "Soldaki kısım menüyü açar, sağdaki kutu Tonti'yi kutuya koyar ya da masaüstüne çıkarır.",
  "Limit bildirimleri", "Kalan %15 ve %5'in altına inince, ayrıca limit yenilenince bildirim gönderir.", "Bitiş tahmini notu",
  "Kartların altında \"Bu hızla biter: 5 saatlik 19:40\" satırını gösterir.", "Sonraki otomatik mesaj notu",
  "Claude kartında pencere başlatıcının bir sonraki mesaj saatini gösterir. Başlatıcı kapalıysa zaten görünmez.", "Haftalık özet",
  "Pazartesi sabahı geçen haftanın toplam token'ını, en yoğun gününü ve en çok kullandığın aracı bildirir. Bildirimler kapalıysa gelmez.",
  "Limiti Claude Code komutuyla ölç", "Claude limitini Claude Code'dan al", "Girişte otomatik başlat", "Windows'a giriş yapınca uygulama kendiliğinden açılır.",
  "Claude oturumunu otomatik yenile", "Claude pencere başlatıcı",
  "Otomatik yenileme", "Verilerin arka planda ne sıklıkla okunacağı. Yerel kayıtlar (token geçmişi, Codex, Antigravity) her yenilemede okunur, 15 saniye bile sorun olmaz.",
  "15 saniye", "30 saniye", "1 dakika", "2 dakika", "5 dakika", "10 dakika", "15 dakika", "Yalnızca elle", "Şimdi senkronize et", "Son senkron: %@", "Henüz okunmadı.",
  "Senkronize et", "Kaynak başına sınırlar", "Gemini kotası: en sık 2 dakikada bir.", "Antigravity limiti: IDE açıksa her yenilemede yerel sunucudan okunur, kapalıysa okunamaz.",
  "Codex ve token geçmişi: yerel dosyalardan, her yenilemede.",
  "NOT ALIR", "ASLA ALMAZ", "hangi uygulamada ne kadar kaldığın", "pencere başlıkları (dosya, belge adı)", "tarayıcıda yalnızca site adı (github.com gibi)",
  "projelerinde kaç dosya değişikliği oldu", "takvimdeki toplantıların", "ekran görüntüsü ya da ekran kaydı", "bastığın tuşlar, yazdığın metinler",
  "sayfa içerikleri, tam adresler, sohbetler", "dosyaların içeriği", "parola uygulamalarının pencere adları",
  "Uygulama süreleri", "Pencere başlıkları", "Tarayıcı siteleri", "Dosya değişiklikleri", "Takvim", "İzin gerekmez",
  "Tüm bilgisayar kayıtları silinecek. Emin misin?", "SİL", "VAZGEÇ", "BİLGİSAYAR KAYITLARINI SİL", "Yapay zekâ oturumları ve commit'ler etkilenmez.", "Silindi",
  "GitHub profil kartı", "GitHub kullanıcı adı", "GitHub anahtarı", "Kayıtlı", "Kaldır", "Kaydet", "Ne sıklıkta güncellensin",
  "Veri değişmediyse ya da o gün kullanım yoksa hiçbir şey gönderilmez.", "Günde bir", "Haftada bir", "Elle", "Servislere göre renklendir",
  "Kapalıysa tek renk (yeşil tonları).", "Koyu kart", "Kapalıysa açık renkli kart.", "Şimdi gönder", "Sonra README'ne bu satırı ekle: ", "Markdown'ı kopyala", "Markdown kopyalandı",
  "Masaüstünde yaşayan kedin Tonti çalışmanı not alır, raporunu hazırlar; yapay zekâ limitlerin görev çubuğunda.", "Sürüm %@", "Güncellemeleri denetle",
  "Güncellemeleri kendiliğinden denetle", "Tanıtım turu", "Görev çubuğunu, kediyi ve ayarları kısaca gösteren beş adımlık tur.", "Turu göster", "GitHub",
  "Meowly'yi beğendiysen bir yıldız başkalarının da bulmasına yardım eder. Hata ya da fikir için de buraya yazabilirsin.", "⭐ Yıldız ver", "Hata bildir",
  "Veri doğruluğu etiketleri", "Sağlayıcının kendi verisi ya da kayıtlardan birebir.", "Tahmin ya da kayıt yapısından çözümlenmiş alanlar.",
  "Kaynak yok ya da erişilemedi; uydurma değer gösterilmez.", "Veri kaynakları", "Codex: ~/.codex oturum kayıtları (limit ve token)",
  "Antigravity: yerel konuşma veritabanı (token), yerel dil sunucusu (limit)", "Gemini: Google Code Assist kota API'si (Gemini CLI oturumuyla)",
  "Çalışıyor", "Bu sağlayıcı Windows sürümünde henüz okunmuyor."];

const SX = { tab: "appearance", data: null, name: null, gh: "", confirm: false, note: "", ghStatus: "", menuOpen: false, msg: "" };

// Metinler (Windows'a uyarlanmış; Mac'te Anahtar Zinciri / macOS izinleri geçen yerler).
const TXT = {
  probe: "Anthropic'in adresi hız sınırına takılınca ya da değer eskiyince, Claude Code komutu tek satırlık küçük bir Haiku isteği atıp güncel limiti ölçer. Terminal gerekmez, Claude uygulamasını kullananlar için de çalışır. Yalnızca yeni Claude kullanımı olunca ve en sık 5 dakikada bir yapılır; çok az token harcar.",
  statusline: "Claude Code, her cevaptan sonra limit yüzdesini yerel bir dosyaya yazar; Meowly onu okur. Ağ gerekmez, Anthropic'in hız sınırına takılmaz. Açınca ~/.claude/settings.json dosyasına küçük bir durum satırı betiği eklenir (önce yedeği alınır, başka bir durum satırı varsa dokunulmaz). Yalnızca Claude Code çalışırken güncellenir.",
  refresh: "Claude Code oturumunun süresi dolarsa (yaklaşık 8 saatte bir) limit değeri eski kalır. Bu açıkken Meowly, Claude Code'a tek satırlık bir istek attırıp oturumunu yeniletir. Çok az token harcar, anahtarlarına dokunmaz.",
  starter: "5 saatlik pencere bittiğinde Haiku'ya tek satırlık bir istek gönderip yeni pencereyi erken başlatır. Ek limit vermez, yalnızca pencerenin başlama saatini belirler. Her istek çok az token harcar. İstek sohbet geçmişine kaydedilmez, bu yüzden Claude uygulamasında görünmez.",
  commits: "Oturum sırasında proje klasörüne attığın commit başlıkları Günlük'e ve raporlara yazılır. Yalnızca senin commit'lerin okunur, bilgisayardan çıkmaz.",
  claudeLimit: "Claude limit yüzdesi: Anthropic'in kullanım bilgisi sınırlı olduğu için seçtiğin süre ne olursa olsun en sık 1 dakikada bir sorgulanır; hız sınırına takılırsa 5 dakika beklenir. Arada son alınan değer gösterilir, yenilenen pencereler hemen %100 olur.",
  logIntro: "Tonti bilgisayarda ne yaptığını not alır; Günlük ve Raporlar bununla dolar. Her şey yalnızca bu bilgisayarda kalır, hiçbir yere gönderilmez.",
  logApps: "Öndeki uygulama değiştikçe not alınır; 5 dakika fare ve klavye kullanmazsan boşta sayılır. İzin gerekmez.",
  logTitles: "Hangi dosyada ya da belgede çalıştığını gösterir (\"CatPet.swift — ullage-cat\" gibi). Yalnızca öndeki pencerenin adı okunur; tuşlarına ya da ekranına bakmaz. Tarayıcılarda başlık saklanmaz.",
  logSites: "Tarayıcıda hangi sitede ne kadar kaldığını yalnızca alan adıyla tutar (youtube.com gibi). Sayfa başlığı, tam adres ve içerik saklanmaz; gizli pencereler atlanır.",
  logFiles: "Yapay zekâ araçlarıyla çalıştığın proje klasörlerinde saatte kaç değişiklik olduğunu sayar. Dosya adı ve içeriği saklanmaz.",
  logCal: "Günlükte ve raporlarda toplantılarını gösterir. Takvimi yalnızca okur, değiştirmez. Paylaşılan raporlara toplantı adları girmez, yalnızca toplam süre.",
  ghCard: "Token kullanımının ısı haritasını, kendi profil reposundaki bir resim olarak günceller. Resimde yalnızca günlük toplam token vardır (proje, model, sohbet yok). Her güncelleme profilinde bir commit olarak görünür.",
  ghUser: "Resim, kullanıcıadı/kullanıcıadı adlı profil reposuna yüklenir. Repo yoksa GitHub'da bu adla boş bir herkese açık repo aç.",
  ghToken: "GitHub > Settings > Developer settings > Fine-grained tokens: yalnızca bu profil reposunu seç, izin olarak Contents: Read and write ver. Anahtar yalnızca bu bilgisayarın Windows Kimlik Bilgisi Yöneticisi'nde durur.",
  update: "Günde bir kez sitedeki küçük bir dosyaya bakar; yalnızca sürüm numarası okunur, sana dair bir şey gönderilmez. Yeni sürüm varsa bildirim gelir, yüklemek için sen onaylarsın.",
  claudeSrc: "Claude: ~/.claude kayıtları (token), Claude Code durum satırı / komutu ya da Anthropic kullanım uç noktası (limit)",
};

const TABS = [["appearance", "Tonti"], ["providers", "Sağlayıcılar"], ["menubar", "Görev çubuğu"], ["automation", "Bildirim ve otomasyon"],
  ["computer", "Bilgisayar günlüğü"], ["sync", "Senkronizasyon"], ["github", "GitHub kartı"], ["about", "Hakkında"]];

// MARK: Denetimler (piksel)

function sw(key, on) { return `<button class="pxsw px ${on ? "on" : ""}" data-sw="${key}" aria-pressed="${on}"><i></i></button>`; }
function segBtns(key, items, v) { return `<span class="seg">${items.map(([l, x]) => `<button class="btn px ${String(x) === String(v) ? "on" : ""}" data-seg="${key}" data-v="${esc(String(x))}">${esc(l)}</button>`).join("")}</span>`; }
function checkBox(key, on, title) { return `<label class="check ${on ? "on" : ""}" data-chk="${key}"><i>${on ? "✓" : ""}</i>${esc(title)}</label>`; }
function srow(title, sub, ctl) {
  return `<div class="px srow"><div class="sl"><b>${esc(title)}</b>${sub ? `<span>${esc(sub)}</span>` : ""}</div><div class="sc">${ctl}</div></div>`;
}
function confTag(c) {
  const [txt, col] = c === "exact" ? ["Kesin", "#34c759"] : c === "estimate" ? ["Tahmini", "#ff9500"] : ["Veri yok", "#8e8e93"];
  return `<span class="ctag" style="color:${col};background:color-mix(in srgb, ${col} 16%, transparent)">${T.t(txt)}</span>`;
}

// MARK: Sekmeler

function tabAppearance(s) {
  const d = SX.data;
  const name = SX.name ?? d.catName;
  let out = srow(T.t("Görünüm"), T.t("Pencere, menü ve görev çubuğundaki gösterge birlikte değişir."), segBtns("dark", [[T.t("Açık"), false], [T.t("Koyu"), true]], s.dark));
  out += srow(T.t("Cache dahil"), T.t("Token sayılarına önbellekten okunan (cache) token'lar da eklenir. Kapalıyken yalnızca yeni okunan ve yazılan token'lar sayılır; sayılar daha küçük ve gerçek işe daha yakın olur."), sw("include_cache", s.include_cache));
  out += srow(T.t("Kedinin adı"), T.t("Masaüstündeki kedin ve uygulamadaki metinler bu adı kullanır."),
    `<span class="namebox"><span class="px pxinw"><input class="pxin" id="catname" maxlength="20" value="${esc(name)}" placeholder="Tonti"></span><button class="btn px ${name !== d.catName ? "on" : ""}" data-savename="1">${T.t("KAYDET")}</button></span>`);
  out += srow(T.t("Tonti'nin balonları"), T.t("Tonti başının üstündeki balonda ne yaptığını söyler. Sohbet içeriği okunmaz; yalnızca proje adı, süre ve token sayısı kullanılır."), sw("cat_bubbles", s.cat_bubbles));
  if (s.cat_bubbles) {
    out += `<div class="px scard2 checks">${checkBox("bubble_work", s.bubble_work, T.t("Çalışırken not alır (proje ve süre)"))}${checkBox("bubble_break", s.bubble_break, T.t("Uzun oturumda mola hatırlatır"))}
      ${checkBox("bubble_daily", s.bubble_daily, T.t("Günün ilk açılışında dünün özeti"))}${checkBox("bubble_record", s.bubble_record, T.t("Günün token eşiklerini kutlar"))}${checkBox("bubble_chat", s.bubble_chat, T.t("Sevilince mırlar"))}</div>`;
  }
  out += srow(T.t("Dil"), T.t("Arayüz dilini seç. Değişiklik hemen uygulanır."), segBtns("language", [[T.t("Sistem"), "system"], ["Türkçe", "tr"], ["English", "en"]], s.language));
  out += srow(T.t("Günlükte commit'leri göster"), T.t(TXT.commits), sw("journal_commits", s.journal_commits));
  return out;
}

function tabProviders() {
  const d = SX.data;
  let out = `<div class="px scard2 setup"><span class="lbl">${T.t("KURULUM KONTROLÜ")}</span>${d.setup.map(it => `<div class="si">
    <i class="${it.ok ? "ok" : "no"}">${it.ok ? "✓" : "!"}</i><div><b>${esc(it.title)}</b>${it.ok ? "" : `<span>${esc(it.hint)}</span>`}</div></div>`).join("")}</div>`;
  out += `<span class="snote">${T.t("Görev çubuğunda ve popover'da görünecek sağlayıcıları seç.")}</span>`;
  for (const p of d.providers || []) {
    out += `<div class="px srow prov">${logo(p.id, 34)}<div class="sl"><div class="row" style="gap:8px"><b>${esc(p.name)}</b><span class="smini">${T.t("Limit")}</span>${confTag(p.limit)}<span class="smini">${T.t("Token")}</span>${confTag(p.token)}</div>
      ${p.notes.map(n => `<span>${esc(n)}</span>`).join("")}</div><div class="sc">${sw("provider:" + p.id, p.on)}</div></div>`;
  }
  return out;
}

const STYLES = [["percent", "Yüzde"], ["bar", "Çubuk"], ["barPercent", "Çubuk + yüzde"], ["ring", "Halka"], ["dot", "Nokta"], ["battery", "Pil"], ["segments", "Segment"]];
function styleGrid(key, cur) {
  const d = SX.data;
  return `<div class="sgrid3">${STYLES.map(([id, l]) => `<div class="stc ${cur === id ? "on" : ""}" data-seg="${key}" data-v="${id}">
    <div class="stp" style="background:${d.dark ? "#4d4d4d" : "#cccccc"}"><img src="${d.previews[id]}" style="height:22px"></div><span>${T.t(l)}</span></div>`).join("")}</div>`;
}

function tabMenubar(s) {
  const d = SX.data;
  let out = `<div class="px scard2"><span class="lbl">${T.t("ÖNİZLEME")}</span><div class="row" style="gap:14px">
    ${d.strip ? `<span class="prevbg" style="background:${d.dark ? "#4d4d4d" : "#cccccc"}"><img src="${d.strip}" style="height:18px"></span>` : ""}
    <span class="sub" style="font-size:11px">${T.t("Soldaki kısım menüyü açar, sağdaki kutu Tonti'yi kutuya koyar ya da masaüstüne çıkarır.")}</span></div></div>`;
  out += srow(T.t("Etiketleri göster"), T.t("5 saatlik için \"5s\", haftalık için \"H\", günlük kota için \"G\". Aynı yüzdeler olsa bile hangisi olduğu anlaşılır."), sw("show_tags", s.show_tags));
  out += srow(T.t("5 saatlik limit"), T.t("Görev çubuğunda göster."), sw("show_five", s.show_five));
  if (s.show_five) out += styleGrid("style_five", s.style_five);
  out += srow(T.t("Haftalık limit"), T.t("Görev çubuğunda göster."), sw("show_week", s.show_week));
  if (s.show_week) out += styleGrid("style_week", s.style_week);
  out += `<span class="snote">${T.t("Hangi sağlayıcıların görüneceğini Sağlayıcılar sayfasından seçersin.")}</span>`;
  return out;
}

function tabAutomation(s) {
  const d = SX.data;
  let out = srow(T.t("Limit bildirimleri"), T.t("Kalan %15 ve %5'in altına inince, ayrıca limit yenilenince bildirim gönderir."), sw("notifications_on", s.notifications_on));
  if (d.winToastsOff && s.notifications_on) out += `<div class="px lift row scard2 wnote"><span class="snote" style="flex:1">${T.t("Windows bildirimleri kapalı; Meowly'nin bildirimleri görünmez. Windows Ayarları > Sistem > Bildirimler'den açabilirsin.")}</span><button class="btn px" data-link="notifications">${T.t("WINDOWS AYARLARI")}</button></div>`;
  out += srow(T.t("Bitiş tahmini notu"), T.t("Kartların altında \"Bu hızla biter: 5 saatlik 19:40\" satırını gösterir."), sw("show_forecast", s.show_forecast));
  out += srow(T.t("Sonraki otomatik mesaj notu"), T.t("Claude kartında pencere başlatıcının bir sonraki mesaj saatini gösterir. Başlatıcı kapalıysa zaten görünmez."), sw("show_next_start", s.show_next_start));
  out += srow(T.t("Haftalık özet"), T.t("Pazartesi sabahı geçen haftanın toplam token'ını, en yoğun gününü ve en çok kullandığın aracı bildirir. Bildirimler kapalıysa gelmez."), sw("weekly_summary", s.weekly_summary));
  out += srow(T.t("Limiti Claude Code komutuyla ölç"), T.t(TXT.probe), sw("cli_probe", s.cli_probe));
  out += srow(T.t("Claude limitini Claude Code'dan al"), T.t(TXT.statusline) + (SX.msg ? " — " + SX.msg : ""), sw("statusline_on", s.statusline_on));
  out += srow(T.t("Girişte otomatik başlat"), T.t("Windows'a giriş yapınca uygulama kendiliğinden açılır."), sw("login_item", d.loginItem));
  out += srow(T.t("Claude oturumunu otomatik yenile"), T.t(TXT.refresh), sw("refresh_login", s.refresh_login));
  out += srow(T.t("Claude pencere başlatıcı"), T.t(TXT.starter) + (d.starterNote || ""), sw("starter_on", s.starter_on));
  return out;
}

function tabComputer(s) {
  const d = SX.data;
  const b = x => `<div class="bl"><b>·</b><span>${T.t(x)}</span></div>`;
  let out = `<div class="px scard2 privacy"><b class="pi">${esc(T.t(TXT.logIntro))}</b><div class="row" style="gap:24px;align-items:flex-start">
    <div class="col" style="gap:4px"><span class="pk">${T.t("NOT ALIR")}</span>${["hangi uygulamada ne kadar kaldığın", "pencere başlıkları (dosya, belge adı)", "tarayıcıda yalnızca site adı (github.com gibi)", "projelerinde kaç dosya değişikliği oldu", "takvimdeki toplantıların"].map(b).join("")}</div>
    <div class="col" style="gap:4px"><span class="pk">${T.t("ASLA ALMAZ")}</span>${["ekran görüntüsü ya da ekran kaydı", "bastığın tuşlar, yazdığın metinler", "sayfa içerikleri, tam adresler, sohbetler", "dosyaların içeriği", "parola uygulamalarının pencere adları"].map(b).join("")}</div></div></div>`;
  // Takvim: Windows'un takvim deposu. Erişim kapalıysa Mac'teki "İZİN VER" gibi Windows gizlilik ayarını açar.
  const layer = (key, title, sub) => key === "log_calendar"
    ? srow(T.t(title), T.t(sub), `<span class="col" style="align-items:flex-end;gap:6px">${sw("calendar_on", s.calendar_on)}${!s.calendar_on ? "" : d.calendarAccess
        ? `<span class="smini">${T.t("Çalışıyor")}</span>` : `<button class="btn px on" data-link="calprivacy">${T.t("İZİN VER")}</button>`}</span>`)
    : srow(T.t(title), T.t(sub), `<span class="col" style="align-items:flex-end;gap:6px">${sw(key, s[key])}${s[key] ? `<span class="smini">${T.t("İzin gerekmez")}</span>` : ""}</span>`);
  out += layer("log_apps", "Uygulama süreleri", TXT.logApps) + layer("log_titles", "Pencere başlıkları", TXT.logTitles) + layer("log_sites", "Tarayıcı siteleri", TXT.logSites)
    + layer("log_files", "Dosya değişiklikleri", TXT.logFiles) + layer("log_calendar", "Takvim", TXT.logCal);
  out += SX.confirm
    ? `<div class="row" style="gap:10px"><b style="font-size:12px">${T.t("Tüm bilgisayar kayıtları silinecek. Emin misin?")}</b><button class="btn px on" data-del="yes">${T.t("SİL")}</button><button class="btn px" data-del="no">${T.t("VAZGEÇ")}</button></div>`
    : `<div class="row" style="gap:10px"><button class="btn px" data-del="ask">${T.t("BİLGİSAYAR KAYITLARINI SİL")}</button><span class="sub" style="font-size:11px">${SX.note ? T.t(SX.note) : T.t("Yapay zekâ oturumları ve commit'ler etkilenmez.")}</span></div>`;
  return out;
}

const INTERVALS = [["15 saniye", 15], ["30 saniye", 30], ["1 dakika", 60], ["2 dakika", 120], ["5 dakika", 300], ["10 dakika", 600], ["15 dakika", 900], ["Yalnızca elle", 0]];
function tabSync(s) {
  const d = SX.data;
  const cur = INTERVALS.find(i => i[1] === s.sync_interval) || INTERVALS[1];
  const menu = `<span class="pxmenu"><button class="btn px" data-menu="1">${T.t(cur[0])}  ▾</button>${SX.menuOpen ? `<span class="px mlist">${INTERVALS.map(([l, v]) => `<a data-seg="sync_interval" data-v="${v}">${T.t(l)}</a>`).join("")}</span>` : ""}</span>`;
  let out = srow(T.t("Otomatik yenileme"), T.t("Verilerin arka planda ne sıklıkla okunacağı. Yerel kayıtlar (token geçmişi, Codex, Antigravity) her yenilemede okunur, 15 saniye bile sorun olmaz."), menu);
  out += srow(T.t("Şimdi senkronize et"), d.updatedAt ? T.f("Son senkron: %@", d.updatedAt) : T.t("Henüz okunmadı."), `<button class="btn px" data-sync="1">${T.t("Senkronize et")}</button>`);
  out += `<div class="px scard2 limits"><b>${T.t("Kaynak başına sınırlar")}</b><span>${T.t(TXT.claudeLimit)}</span><span>${T.t("Gemini kotası: en sık 2 dakikada bir.")}</span>
    <span>${T.t("Antigravity limiti: IDE açıksa her yenilemede yerel sunucudan okunur, kapalıysa okunamaz.")}</span><span>${T.t("Codex ve token geçmişi: yerel dosyalardan, her yenilemede.")}</span></div>`;
  return out;
}

function tabGithub(s) {
  const d = SX.data;
  let out = srow(T.t("GitHub profil kartı"), T.t(TXT.ghCard), sw("gh_on", s.gh_on));
  if (!s.gh_on) return out;
  out += srow(T.t("GitHub kullanıcı adı"), T.t(TXT.ghUser), `<span class="px pxinw"><input class="pxin" id="ghuser" value="${esc(s.gh_user)}" placeholder="kullanıcıadı" style="width:180px"></span>`);
  out += srow(T.t("GitHub anahtarı"), T.t(TXT.ghToken), d.ghToken
    ? `<span class="row" style="gap:8px"><span class="sub" style="font-size:12px">${T.t("Kayıtlı")}</span><button class="btn px" data-gh="remove">${T.t("Kaldır")}</button></span>`
    : `<span class="row" style="gap:8px"><span class="px pxinw"><input class="pxin" id="ghtok" type="password" placeholder="github_pat_…" style="width:170px"></span><button class="btn px" data-gh="save">${T.t("Kaydet")}</button></span>`);
  out += srow(T.t("Ne sıklıkta güncellensin"), T.t("Veri değişmediyse ya da o gün kullanım yoksa hiçbir şey gönderilmez."), segBtns("gh_interval", [[T.t("Günde bir"), "daily"], [T.t("Haftada bir"), "weekly"], [T.t("Elle"), "manual"]], s.gh_interval));
  out += srow(T.t("Servislere göre renklendir"), T.t("Kapalıysa tek renk (yeşil tonları)."), sw("gh_split", s.gh_split));
  out += srow(T.t("Koyu kart"), T.t("Kapalıysa açık renkli kart."), sw("gh_dark", s.gh_dark));
  const ghSt = SX.ghStatus || d.ghStatus;
  out += srow(T.t("Şimdi gönder"), ghSt || (T.t("Sonra README'ne bu satırı ekle: ") + d.ghMarkdown),
    `<span class="row" style="gap:8px"><button class="btn px" data-gh="md" ${s.gh_user ? "" : "disabled"}>${T.t("Markdown'ı kopyala")}</button><button class="btn px" data-gh="send" ${SX.ghBusy || !d.ghToken || !s.gh_user ? "disabled" : ""}>${T.t("Şimdi gönder")}</button></span>`);
  return out;
}

function tabAbout(s) {
  const d = SX.data;
  let out = `<div class="px scard2 about"><span class="abox"><img class="px-img" src="img/cat/content.png"></span><div class="col" style="gap:4px"><b style="font-size:22px">Meowly</b>
    <span class="sub" style="font-size:12px">${T.t("Masaüstünde yaşayan kedin Tonti çalışmanı not alır, raporunu hazırlar; yapay zekâ limitlerin görev çubuğunda.")}</span></div></div>`;
  const up = d.update || {}, a = up.available;
  const ubtn = a ? `<button class="btn px on" data-upd="install" ${SX.updBusy ? "disabled" : ""}>${esc(T.f("%@ sürümüne güncelle", a.version))}</button>`
    : `<button class="btn px" data-upd="check" ${SX.updBusy ? "disabled" : ""}>${T.t("Güncellemeleri denetle")}</button>`;
  out += `<div class="px scard2 upd"><div class="row"><b style="font-size:13px">${T.f("Sürüm %@", d.version)}</b><span class="spacer"></span>${ubtn}</div>
    ${SX.updStatus ? `<span class="snote" style="font-size:11px">${esc(SX.updStatus)}</span>` : ""}
    ${a && a.notes ? `<span class="snote" style="font-size:11px">${esc(a.notes)}</span>` : ""}
    <div class="row" style="gap:16px"><div class="sl"><b style="font-size:12px">${T.t("Güncellemeleri kendiliğinden denetle")}</b><span>${T.t(TXT.update)}</span></div><span class="spacer"></span>${sw("update_check", s.update_check)}</div></div>`;
  out += srow(T.t("Tanıtım turu"), T.t("Görev çubuğunu, kediyi ve ayarları kısaca gösteren beş adımlık tur."), `<button class="btn px" data-tour="1">${T.t("Turu göster")}</button>`);
  out += srow(T.t("GitHub"), T.t("Meowly'yi beğendiysen bir yıldız başkalarının da bulmasına yardım eder. Hata ya da fikir için de buraya yazabilirsin."),
    `<span class="row" style="gap:8px"><button class="btn px" data-link="repo">${T.t("⭐ Yıldız ver")}</button><button class="btn px" data-link="issues">${T.t("Hata bildir")}</button></span>`);
  out += `<div class="px scard2 labels"><b>${T.t("Veri doğruluğu etiketleri")}</b>
    <div class="row" style="gap:8px">${confTag("exact")}<span>${T.t("Sağlayıcının kendi verisi ya da kayıtlardan birebir.")}</span></div>
    <div class="row" style="gap:8px">${confTag("estimate")}<span>${T.t("Tahmin ya da kayıt yapısından çözümlenmiş alanlar.")}</span></div>
    <div class="row" style="gap:8px">${confTag("none")}<span>${T.t("Kaynak yok ya da erişilemedi; uydurma değer gösterilmez.")}</span></div></div>`;
  out += `<div class="px scard2 labels"><b>${T.t("Veri kaynakları")}</b><span>${T.t(TXT.claudeSrc)}</span><span>${T.t("Codex: ~/.codex oturum kayıtları (limit ve token)")}</span>
    <span>${T.t("Antigravity: yerel konuşma veritabanı (token), yerel dil sunucusu (limit)")}</span><span>${T.t("Gemini: Google Code Assist kota API'si (Gemini CLI oturumuyla)")}</span></div>`;
  return out;
}

function renderSettings(main, tab) {
  SX.tab = tab;
  const s = SX.data.s;
  const body = { appearance: tabAppearance, providers: tabProviders, menubar: tabMenubar, automation: tabAutomation, computer: tabComputer, sync: tabSync, github: tabGithub, about: tabAbout }[tab](s);
  const keepScroll = main.querySelector(".sbody")?.scrollTop || 0;
  main.innerHTML = `<div class="settings"><div class="page-title">${T.t("Ayarlar")}</div>
    <div class="tabs">${TABS.map(([id, t]) => `<button class="btn px ${id === tab ? "on" : ""}" data-page="${id}">${T.t(t)}</button>`).join("")}</div>
    <div class="sbody">${body}</div></div>`;
  main.querySelector(".sbody").scrollTop = keepScroll;
}

async function loadSettings(main, tab) {
  SX.data = await invoke("settings_page");
  renderSettings(main, tab);
}

async function setS(main, key, value) {
  const r = await invoke("set_setting", { key, value });
  SX.msg = r || "";
  // Ana süreç değeri okuyup durumu yazınca sayfa güncel veriyle yeniden çizilir.
  await loadSettings(main, SX.tab);
}

function settingsClick(e, main) {
  const t = e.target.closest("[data-sw],[data-seg],[data-chk],[data-savename],[data-del],[data-menu],[data-sync],[data-gh],[data-link],[data-upd],[data-tour]");
  if (!t) { if (SX.menuOpen) { SX.menuOpen = false; renderSettings(main, SX.tab); } return false; }
  const ds = t.dataset, s = SX.data.s;
  if (ds.sw) {
    if (ds.sw.startsWith("provider:")) {
      const id = ds.sw.slice(9), p = SX.data.providers.find(x => x.id === id);
      setS(main, "provider", { id, on: !p.on });
    } else if (ds.sw === "login_item") setS(main, "login_item", !SX.data.loginItem);
    else setS(main, ds.sw, !s[ds.sw]);
  } else if (ds.seg) {
    let v = ds.v; if (v === "true" || v === "false") v = v === "true"; else if (ds.seg === "sync_interval") v = Number(v);
    SX.menuOpen = false; setS(main, ds.seg, v);
  } else if (ds.chk) setS(main, ds.chk, !s[ds.chk]);
  else if (ds.savename) { const n = main.querySelector("#catname").value; SX.name = null; invoke("save_cat_name", { name: n }).then(() => loadSettings(main, SX.tab)); }
  else if (ds.del === "ask") { SX.confirm = true; SX.note = ""; renderSettings(main, SX.tab); }
  else if (ds.del === "no") { SX.confirm = false; renderSettings(main, SX.tab); }
  else if (ds.del === "yes") { invoke("delete_computer_log"); SX.confirm = false; SX.note = "Silindi"; renderSettings(main, SX.tab); }
  else if (ds.menu) { SX.menuOpen = !SX.menuOpen; renderSettings(main, SX.tab); }
  else if (ds.sync) invoke("sync_now");
  else if (ds.gh === "save") { const v = main.querySelector("#ghtok").value; if (v) invoke("gh_token", { action: "save", token: v }).then(() => loadSettings(main, SX.tab)); }
  else if (ds.gh === "remove") invoke("gh_token", { action: "remove" }).then(() => loadSettings(main, SX.tab));
  else if (ds.gh === "send") {
    if (SX.ghBusy) return true;
    SX.ghBusy = true; renderSettings(main, SX.tab);
    invoke("gh_send").then(m => { SX.ghStatus = m; SX.ghBusy = false; renderSettings(main, SX.tab); });
  }
  else if (ds.gh === "md") { navigator.clipboard.writeText(SX.data.ghMarkdown); SX.ghStatus = T.t("Markdown kopyalandı"); renderSettings(main, SX.tab); }
  else if (ds.link) invoke("open_link", { which: ds.link });
  else if (ds.upd) updateAct(main, ds.upd);
  else if (ds.tour) invoke("tour_start");
  return true;
}

/** Güncelleme: denetle ya da indir-doğrula-kur (Mac: UpdateCard). */
async function updateAct(main, what) {
  if (SX.updBusy) return;
  SX.updBusy = true;
  SX.updStatus = T.t(what === "check" ? "Denetleniyor…" : "İndiriliyor…");
  renderSettings(main, SX.tab);
  if (what === "check") {
    const r = await invoke("update_check");
    SX.data.update = r; SX.updStatus = r.status;
  } else {
    const r = await invoke("update_install");
    SX.updStatus = r || T.t("Yeniden başlatılıyor…");
  }
  SX.updBusy = false;
  renderSettings(main, SX.tab);
}

function settingsInput(e, main) {
  if (e.target.id === "catname") { SX.name = e.target.value; const b = main.querySelector("[data-savename]"); if (b) b.classList.toggle("on", SX.name !== SX.data.catName); }
}
function settingsKey(e, main) {
  if (e.key !== "Enter") return;
  if (e.target.id === "catname") { invoke("save_cat_name", { name: e.target.value }).then(() => { SX.name = null; loadSettings(main, SX.tab); }); }
  if (e.target.id === "ghuser") setS(main, "gh_user", e.target.value);
}
function settingsChange(e, main) { if (e.target.id === "ghuser") setS(main, "gh_user", e.target.value); }
