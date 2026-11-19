import Foundation

/// Arayüz dili. Metinler Türkçe yazılır; İngilizce seçiliyse (ya da sistem dili Türkçe değilse) buradaki tablodan çevrilir.
/// Tabloda olmayan metin olduğu gibi gösterilir. Dinamik metinler için `phrases` kelime/öbek değişimi yapar.
enum L {
    /// "system", "tr" ya da "en". AILIMIT_LANG ortam değişkeni önizleme/test içindir.
    static var lang: String {
        ProcessInfo.processInfo.environment["AILIMIT_LANG"] ?? UserDefaults.standard.string(forKey: "language") ?? "system"
    }

    static var en: Bool {
        switch lang {
        case "en": return true
        case "tr": return false
        default: return !(Locale.preferredLanguages.first ?? "tr").hasPrefix("tr")
        }
    }

    static var locale: Locale { Locale(identifier: en ? "en_US" : "tr_TR") }

    static func t(_ s: String) -> String {
        guard en else { return CatName.apply(s) }
        if let x = exact[s] { return CatName.apply(x) }
        var r = s
        for (a, b) in phrases { r = r.replacingOccurrences(of: a, with: b) }
        return CatName.apply(r)
    }

    /// Biçim metni (%@, %lld) çevrilip değerler yerleştirilir.
    static func f(_ key: String, _ args: CVarArg...) -> String {
        String(format: t(key), locale: locale, arguments: args)
    }

    private static let phrases: [(String, String)] = [
        ("Limit alınamadı", "Could not read limit"),
        ("Sıfırlanma", "Resets in"),
        ("saatlik", "hour"),
        ("Haftalık", "Weekly"),
        ("haftalık", "weekly"),
        ("günlük kota", "daily quota"),
        ("Günlük", "Daily"),
        ("Bugün", "Today"),
        ("bugün", "today"),
        ("sıfırlandı", "reset"),
        ("kaldı", "left"),
    ]

    private static let exact: [String: String] = [
        // Biçimli metinler
        "Güncellendi %@": "Updated %@", "Bugün %@": "Today %@", "Eski değer: %@ (güncellenemedi)": "Stale value: %@ (could not refresh)",
        "Sıfırlanma %@": "Resets in %@", "· sıfırlandı": "· reset",
        "Anthropic hız sınırı uyguladığı için son alınan değer gösteriliyor (%@); güncel olmayabilir.": "Anthropic rate-limited the request, so the last value received is shown (%@); it may be out of date.",
        "Limit yüzdesi alınamadı: %@": "Could not read the limit: %@", "Limit alınamadı: %@": "Could not read the limit: %@",
        "Uç nokta %lld döndürdü": "The endpoint returned %lld",
        "Limit ve token: Codex oturum kayıtlarından (son kayıt %@).": "Limit and tokens: from Codex session logs (latest entry %@).",
        "Oturum yenilenemedi (%lld); Gemini CLI'ı bir kez çalıştırıp giriş yap": "Could not renew the session (%lld); run the Gemini CLI once and sign in",
        "Oturum süresi dolmuş ve yenilemek için Gemini CLI kurulu değil (npm i -g @google/gemini-cli, sonra bir kez giriş yap)": "Session expired and the Gemini CLI is not installed to renew it (npm i -g @google/gemini-cli, then sign in once)",
        "Kota yanıtı %lld döndürdü%@": "The quota response returned %lld%@",
        "Son senkron: %@": "Last sync: %@", " Sonraki otomatik mesaj: %@.": " Next auto-message: %@.",
        "%lld gün": "%lld days", "%lld aktif gün": "%lld active days", "%@ · kullanım yok": "%@ · no usage", "%@ · %@ token · %@": "%@ · %@ tokens · %@",
        "Anthropic bu hesaba geçici bir hız sınırı koydu. %@ sonrasında kendiliğinden düzelir, bir şey yapmana gerek yok. Uygulamayı o zamana kadar yeniden başlatma.":
            "Anthropic has temporarily rate-limited this account. It fixes itself after %@; you don't need to do anything. Don't restart the app until then.",
        "Claude limitini Claude Code'dan al": "Read the Claude limit from Claude Code",
        "Claude Code, her cevaptan sonra limit yüzdesini yerel bir dosyaya yazar; Meowly onu okur. Ağ gerekmez, Anthropic'in hız sınırına takılmaz. Açınca ~/.claude/settings.json dosyasına küçük bir durum satırı betiği eklenir (önce yedeği alınır, başka bir durum satırı varsa dokunulmaz). Yalnızca Claude Code çalışırken güncellenir.":
            "Claude Code writes the limit percentage to a local file after every reply and Meowly reads it. No network, no Anthropic rate limit. Turning it on adds a small status line script to ~/.claude/settings.json (backed up first; an existing status line is left alone). It only updates while Claude Code is running.",
        "Claude Code durum satırı bağlı": "Claude Code status line connected",
        "İsteğe bağlı. Bildirim ve otomasyon sayfasından açarsan limit, ağa gitmeden Claude Code'dan okunur. Açtıktan sonra Claude Code'ta bir mesaj yazınca veri gelir.": "Optional. If you turn it on under Notifications & automation, the limit is read from Claude Code without the network. Data arrives after you send a message in Claude Code.",
        "Limit: Claude Code'un durum satırından (yerel, ağ gerekmez).": "Limit: from the Claude Code status line (local, no network).",
        "Başka bir durum satırı tanımlı, değiştirilmedi": "Another status line is already set; left unchanged",
        "Limiti Claude Code komutuyla ölç": "Measure the limit with the Claude Code command",
        "Anthropic'in adresi hız sınırına takılınca ya da değer eskiyince, Claude Code komutu tek satırlık küçük bir Haiku isteği atıp güncel limiti ölçer. Terminal gerekmez, Claude uygulamasını kullananlar için de çalışır. Yalnızca yeni Claude kullanımı olunca ve en sık 5 dakikada bir yapılır; çok az token harcar.":
            "When Anthropic's endpoint is rate-limited or the value is stale, the Claude Code command sends one tiny Haiku request and measures the current limit. No terminal needed; it works for Claude app users too. Only runs after new Claude usage and at most every 5 minutes; uses very few tokens.",
        "Limit: Claude Code komutundan ölçüldü (tek satırlık küçük bir istek).": "Limit: measured with the Claude Code command (one tiny request).",
        // Sayfa adları
        "Kullanım": "Usage", "Sağlayıcılar": "Providers", "Senkronizasyon": "Sync", "Menü çubuğu": "Menu bar",
        "Görünüm": "Appearance", "Bildirim ve otomasyon": "Notifications & automation", "Hakkında": "About",
        // Popover
        "Tümü": "All", "Ayarlar": "Settings", "5 SAATLİK": "5 HOUR", "HAFTALIK": "WEEKLY", "Pencere sıfırlandı": "Window reset",
        "Hiçbir sağlayıcı seçili değil. Ayarlardan aç.": "No provider selected. Turn some on in Settings.",
        "Veri alınamadı. Ayrıntı için Sağlayıcılar sayfasına bak.": "No data. See the Providers page for details.",
        "Veri alınamadı.": "No data.", "HAVUZLAR": "POOLS", "KOTALAR": "QUOTAS", "TOKEN": "TOKENS", "bugün": "today",
        "kaldı": "left", "sıfırlandı": "reset", "şimdi": "now",
        "Bitti": "Used up", "Çok hızlı": "Very fast", "Hızlı": "Fast", "Rahat": "Relaxed", "Dengeli": "Balanced",
        "Sağlıklı": "Healthy", "Dikkat": "Careful", "Kritik": "Critical",
        "Kesin": "Exact", "Tahmini": "Estimate", "Veri yok": "No data",
        // Temalar ve stiller
        "Sistem": "System", "Koyu": "Dark", "Açık": "Light", "Kağıt": "Paper", "Cam": "Glass",
        "Yüzde": "Percent", "Çubuk": "Bar", "Çubuk + yüzde": "Bar + percent", "Halka": "Ring", "Nokta": "Dot", "Pil": "Battery", "Segment": "Segments",
        // Kullanım sayfası
        "24 saat": "24 hours", "7 gün": "7 days", "30 gün": "30 days", "Sağlayıcı": "Provider",
        "BUGÜN": "TODAY", "DÜN": "YESTERDAY", "SON 7 GÜN": "LAST 7 DAYS", "SON 30 GÜN": "LAST 30 DAYS",
        "düne göre": "vs yesterday", "önceki haftaya göre": "vs previous week",
        "TAHMİNİ API MALİYETİ": "ESTIMATED API COST",
        "Aynı token'ları API fiyatıyla alsaydın ödeyeceğin tutar. Abonelik ücretin değil, tahminidir.":
            "What the same tokens would cost at API prices. Not your subscription fee; this is an estimate.",
        "Bugün": "Today",
        "EN ÇOK KULLANILAN PROJELER": "TOP PROJECTS", "Son 30 gün · yalnızca Claude Code, klasör adıyla": "Last 30 days · Claude Code only, by folder name",
        "Henüz proje verisi yok.": "No project data yet.",
        "Paylaşım kartı": "Share card", "Cache dahil": "Include cache", " · cache dahil": " · incl. cache", "Hepsi": "All",
        "Token kullanımı": "Token usage", " toplam": " total", "Bu aralıkta kayıt yok.": "No records in this range.",
        "Zaman": "Time", "Dağılım": "Breakdown",
        "TOPLAM TOKEN": "TOTAL TOKENS", "tüm zamanlar": "all time", "GÜNCEL SERİ": "CURRENT STREAK", "kesintisiz kullanım": "days in a row",
        "bugün kullanım yok": "no usage today", "EN UZUN SERİ": "LONGEST STREAK", "EN YOĞUN GÜN": "BUSIEST DAY",
        "GÜNLÜK ORT.": "DAILY AVG.", "aktif gün başına": "per active day",
        "Bir günün üzerine gel, o günkü kullanımı gör.": "Hover over a day to see that day's usage.",
        "Etkinlik": "Activity", "Son 53 hafta": "Last 53 weeks", "Sağlayıcıya göre renk": "Color by provider", "Yoğunluk": "Intensity",
        "Az": "Less", "Pzt": "Mon", "Çar": "Wed", "Cum": "Fri", "Çok": "More",
        "Son 7 gün": "Last 7 days", "token": "tokens", "API değeri": "API value",
        // Kurulum kontrolü
        "KURULUM KONTROLÜ": "SETUP CHECK", "Claude Code kurulu": "Claude Code is installed",
        "Terminalde `claude` komutunu bir kez çalıştırıp giriş yap. Meowly limiti oradaki oturumdan okur.":
            "Run `claude` once in a terminal and sign in. Meowly reads the limit from that session.",
        "Claude limiti okunuyor": "Claude limit is readable",
        "Claude Code'a giriş yapılı değilse ya da oturum bittiyse limit görünmez. Terminalde `claude` çalıştır, gerekirse /login yaz.":
            "If you are not signed in to Claude Code, or the session expired, the limit won't show. Run `claude` in a terminal and type /login if needed.",
        "Codex kullanımı bulundu": "Codex usage found",
        "Codex'i hiç kullanmadıysan bu normal. Kullanınca kendiliğinden görünür.": "Normal if you never used Codex. It shows up once you do.",
        "Codex limiti okunuyor": "Codex limit is readable",
        "Codex ile bir mesaj yazınca limit değeri kayıtlara düşer.": "The limit appears in the logs after you send a message in Codex.",
        "Gemini CLI oturumu var": "Gemini CLI session found",
        "Gemini istemiyorsan boş bırak. Kullanacaksan terminalde `gemini` çalıştırıp giriş yap.": "Skip it if you don't want Gemini. Otherwise run `gemini` in a terminal and sign in.",
        // Sağlayıcılar sayfası
        "Menü çubuğunda ve popover'da görünecek sağlayıcıları seç.": "Choose which providers appear in the menu bar and popover.",
        "Limit": "Limit",
        // Menü çubuğu / görünüm
        "ÖNİZLEME": "PREVIEW", "Etiketleri göster": "Show labels",
        "5 saatlik için \"5s\", haftalık için \"H\", günlük kota için \"G\". Aynı yüzdeler olsa bile hangisi olduğu anlaşılır.":
            "\"5h\" for the 5-hour limit, \"W\" for weekly, \"D\" for daily quota, so you can tell which is which even when the percentages match.",
        "5 saatlik limit": "5-hour limit", "Menü çubuğunda göster.": "Show in the menu bar.", "Haftalık limit": "Weekly limit",
        "Hangi sağlayıcıların görüneceğini Sağlayıcılar sayfasından seçersin.": "Pick which providers appear on the Providers page.",
        "Tema popover'a ve bu pencereye uygulanır. Menü çubuğu her zaman sistemin açık/koyu görünümüne uyar.":
            "The theme applies to the popover and this window. The menu bar always follows the system light/dark appearance.",
        "Dil": "Language", "Arayüz dilini seç. Değişiklik hemen uygulanır.": "Choose the interface language. Changes apply immediately.",
        // Bildirim ve otomasyon
        "Limit bildirimleri": "Limit notifications", "Kalan %15 ve %5'in altına inince, ayrıca limit yenilenince bildirim gönderir.":
            "Notifies you when the remainder drops below 15% and 5%, and when the limit resets.",
        "Bitiş tahmini notu": "Forecast note", "Kartların altında \"Bu hızla biter: 5 saatlik 19:40\" satırını gösterir.":
            "Shows a line like \"At this pace runs out: 5-hour 19:40\" under the cards.",
        "Sonraki otomatik mesaj notu": "Next auto-message note",
        "Claude kartında pencere başlatıcının bir sonraki mesaj saatini gösterir. Başlatıcı kapalıysa zaten görünmez.":
            "Shows when the window starter will send its next message on the Claude card. Hidden when the starter is off.",
        "Haftalık özet": "Weekly summary",
        "Pazartesi sabahı geçen haftanın toplam token'ını, en yoğun gününü ve en çok kullandığın aracı bildirir. Bildirimler kapalıysa gelmez.":
            "On Monday morning, notifies you of last week's total tokens, busiest day and most used tool. Not sent if notifications are off.",
        "Girişte otomatik başlat": "Launch at login", "Mac'e giriş yapınca uygulama kendiliğinden açılır.": "The app opens automatically when you log in to your Mac.",
        "Claude oturumunu otomatik yenile": "Refresh Claude session automatically",
        "Claude Code oturumunun süresi dolarsa (yaklaşık 8 saatte bir) limit değeri eski kalır. Bu açıkken Meowly, Claude Code'a tek satırlık bir istek attırıp oturumunu yeniletir. Çok az token harcar, anahtarlarına dokunmaz.":
            "When the Claude Code session expires (roughly every 8 hours) the limit value goes stale. With this on, Meowly has Claude Code send one tiny request so the session renews. Uses very few tokens and never touches your keys.",
        "Claude pencere başlatıcı": "Claude window starter",
        "5 saatlik pencere bittiğinde Haiku'ya tek satırlık bir istek gönderip yeni pencereyi erken başlatır. Ek limit vermez, yalnızca pencerenin başlama saatini belirler. Her istek çok az token harcar.":
            "When the 5-hour window ends, sends Haiku a one-line request to start the next window early. It gives no extra limit, it only decides when the window starts. Each request uses very few tokens.",
        " Sıfırlanma saati bekleniyor.": " Waiting for the reset time.",
        // Hakkında
        "Yapay zekâ araçlarının limitini ve token kullanımını menü çubuğundan takip et.": "Track the limits and token usage of your AI tools from the menu bar.",
        "Veri doğruluğu etiketleri": "Data accuracy labels", "Sağlayıcının kendi verisi ya da kayıtlardan birebir.": "The provider's own data, or taken exactly from logs.",
        "Tahmin ya da kayıt yapısından çözümlenmiş alanlar.": "Estimates or fields inferred from the log structure.",
        "Kaynak yok ya da erişilemedi; uydurma değer gösterilmez.": "No source or not reachable; no made-up values are shown.",
        "Veri kaynakları": "Data sources",
        "Claude: ~/.claude kayıtları (token), Anthropic kullanım uç noktası (limit)": "Claude: ~/.claude logs (tokens), Anthropic usage endpoint (limit)",
        "Codex: ~/.codex oturum kayıtları (limit ve token)": "Codex: ~/.codex session logs (limit and tokens)",
        "Antigravity: yerel konuşma veritabanı (token), yerel dil sunucusu (limit)": "Antigravity: local conversation database (tokens), local language server (limit)",
        "Gemini: Google Code Assist kota API'si (Gemini CLI oturumuyla)": "Gemini: Google Code Assist quota API (via the Gemini CLI session)",
        // Senkronizasyon
        "15 saniye": "15 seconds", "30 saniye": "30 seconds", "1 dakika": "1 minute", "2 dakika": "2 minutes", "5 dakika": "5 minutes",
        "10 dakika": "10 minutes", "15 dakika": "15 minutes", "Yalnızca elle": "Manual only",
        "Otomatik yenileme": "Automatic refresh",
        "Verilerin arka planda ne sıklıkla okunacağı. Yerel kayıtlar (token geçmişi, Codex, Antigravity) her yenilemede okunur, 15 saniye bile sorun olmaz.":
            "How often data is read in the background. Local logs (token history, Codex, Antigravity) are read on every refresh; even 15 seconds is fine.",
        "Şimdi senkronize et": "Sync now", "Okunuyor…": "Reading…", "Henüz okunmadı.": "Not read yet.", "Senkronize et": "Sync",
        "Kaynak başına sınırlar": "Per-source limits",
        "Claude limit yüzdesi: Anthropic'in kullanım bilgisi sınırlı olduğu için seçtiğin süre ne olursa olsun en sık 1 dakikada bir sorgulanır; hız sınırına takılırsa 5 dakika beklenir. Arada son alınan değer gösterilir, yenilenen pencereler hemen %100 olur.":
            "Claude limit percentage: because Anthropic rate-limits usage info, it is queried at most once a minute whatever you choose; if rate-limited it waits 5 minutes. The last value is shown in between, and windows that reset show 100% right away.",
        "Gemini kotası: en sık 2 dakikada bir.": "Gemini quota: at most every 2 minutes.",
        "Antigravity limiti: IDE açıksa her yenilemede yerel sunucudan okunur, kapalıysa okunamaz.": "Antigravity limit: read from the local server on every refresh while the IDE is open; unreadable when it is closed.",
        "Codex ve token geçmişi: yerel dosyalardan, her yenilemede.": "Codex and token history: from local files, on every refresh.",
        // Bildirimler ve mesajlar
        "Meowly haftalık özet": "Meowly weekly summary",
        "%@: limit azaldı": "%@: limit is running low", "%@: limit yenilendi": "%@: limit reset",
        "%@ için kalan %@ altına düştü (%lld%%).": "%@ remainder dropped below %@ (%lld%%).",
        "%@ penceresi sıfırlandı, %%%lld kaldı.": "%@ window reset, %%%lld left.",
        "Geçen hafta %@ token.": "Last week: %@ tokens.", " En yoğun gün %@ (%@).": " Busiest day %@ (%@).", " En çok %@.": " Most used: %@.",
        "GitHub profil kartı": "GitHub profile card",
        "Token kullanımının ısı haritasını, kendi profil reposundaki bir resim olarak günceller. Resimde yalnızca günlük toplam token vardır (proje, model, sohbet yok). Her güncelleme profilinde bir commit olarak görünür.": "Keeps a heatmap of your token usage up to date as an image in your own profile repo. The image holds only daily token totals (no projects, models or chats). Each update shows up as a commit on your profile.",
        "GitHub kullanıcı adı": "GitHub username",
        "Resim, kullanıcıadı/kullanıcıadı adlı profil reposuna yüklenir. Repo yoksa GitHub'da bu adla boş bir herkese açık repo aç.": "The image is uploaded to your profile repo (username/username). If it does not exist, create an empty public repo with that name on GitHub.",
        "GitHub anahtarı": "GitHub token",
        "GitHub > Settings > Developer settings > Fine-grained tokens: yalnızca bu profil reposunu seç, izin olarak Contents: Read and write ver. Anahtar yalnızca bu Mac'in Anahtar Zinciri'nde durur.": "GitHub > Settings > Developer settings > Fine-grained tokens: select only this profile repo and grant Contents: Read and write. The token stays only in this Mac's Keychain.",
        "Kayıtlı": "Saved",
        "Kaldır": "Remove",
        "Kaydet": "Save",
        "Ne sıklıkta güncellensin": "Update frequency",
        "Veri değişmediyse ya da o gün kullanım yoksa hiçbir şey gönderilmez.": "Nothing is sent if the data did not change or there was no usage that day.",
        "Günde bir": "Daily",
        "Haftada bir": "Weekly",
        "Elle": "Manual",
        "Servislere göre renklendir": "Color by service",
        "Kapalıysa tek renk (yeşil tonları).": "When off, a single color (shades of green).",
        "Koyu kart": "Dark card",
        "Kapalıysa açık renkli kart.": "When off, a light card.",
        "Şimdi gönder": "Send now",
        "Sonra README'ne bu satırı ekle: ": "Then add this line to your README: ",
        "Markdown'ı kopyala": "Copy Markdown",
        "Markdown kopyalandı": "Markdown copied",
        "Kullanıcı adı ve anahtar gerekli": "Username and token required",
        "Kart değişmemiş, gönderilmedi": "Card unchanged, not sent",
        "GitHub anahtarı geçersiz ya da yetkisiz (repo içeriğine yazma izni gerekir)": "GitHub token is invalid or unauthorized (needs write access to repo contents)",
        "Gönderildi": "Sent",
        "%@/%@ reposu bulunamadı. GitHub'da bu adla bir repo aç (README'li, herkese açık).": "Repo %@/%@ not found. Create a public repo with that name on GitHub (with a README).",
        "GitHub hatası (%lld)": "GitHub error (%lld)",
        "GitHub'a bağlanılamadı": "Could not reach GitHub",
        "Denetleniyor…": "Checking…",
        "Güncelleme bilgisi okunamadı": "Could not read update info",
        "Yeni sürüm var: %@": "New version: %@",
        "%@ hazır": "%@ is ready",
        "Ayarlar > Hakkında bölümünden güncelleyebilirsin.": "You can update from Settings > About.",
        "Güncelsin": "You are up to date",
        "Güncelleme sunucusuna bağlanılamadı": "Could not reach the update server",
        "İndiriliyor…": "Downloading…",
        "Uygulama bu konumda güncellenemiyor. Yeni sürümü siteden indir.": "The app cannot be updated in this location. Download the new version from the site.",
        "Bu sürüm imzalı değil, kendiliğinden güncellenemez. Yeni sürümü siteden indir.": "This build is not signed and cannot update itself. Download the new version from the site.",
        "İndirilemedi": "Download failed",
        "İndirilen dosya doğrulanamadı (SHA-256 uyuşmuyor). Güncelleme yapılmadı.": "The downloaded file could not be verified (SHA-256 mismatch). Nothing was changed.",
        "Doğrulanıyor…": "Verifying…",
        "Zip açılamadı": "Could not unzip",
        "Zip içinde uygulama bulunamadı": "No app found in the zip",
        "Yeni sürümün imzası bu uygulamayla eşleşmiyor. Güncelleme yapılmadı.": "The new version's signature does not match this app. Nothing was changed.",
        "Sürüm bilgisi tutmuyor. Güncelleme yapılmadı.": "Version info does not match. Nothing was changed.",
        "Yeniden başlatılıyor…": "Restarting…",
        "Güncelleme başarısız oldu": "Update failed",
        "Sürüm %@": "Version %@",
        "%@ sürümüne güncelle": "Update to %@",
        "Güncellemeleri denetle": "Check for updates",
        "Güncellemeleri kendiliğinden denetle": "Check for updates automatically",
        "Günde bir kez sitedeki küçük bir dosyaya bakar; yalnızca sürüm numarası okunur, sana dair bir şey gönderilmez. Yeni sürüm varsa bildirim gelir, yüklemek için sen onaylarsın.": "Once a day it looks at a small file on the site; only the version number is read and nothing about you is sent. If there is a new version you get a notification and you confirm the install.",
        "Güncelleme adresi tanımlı değil": "No update address is set",
        "Tonti'yi gizle": "Hide Tonti",
        "5 saatlik %%%lld · Haftalık %%%lld": "5-hour %%%lld · Weekly %%%lld",
        "Tonti · 5 saatlik %%%lld · Haftalık %%%lld": "Tonti · 5-hour %%%lld · Weekly %%%lld",
        "✎ Not alıyorum…": "✎ Taking notes…",
        "✎ Yeni sayfa açtım": "✎ New page",
        "Oturumu kaydettim": "Session saved",
        " oldu": " in",
        "Biraz esneyelim mi?": "Time for a stretch?",
        "Bugün %@ token!": "%@ tokens today!",
        "Deftere yazdım": "Noted it down",
        "Dün": "Yesterday",
        " · %lld proje": " · %lld projects",
        "Rapor hazır, bana tıkla": "Report ready, click me",
        "%lld dk": "%lldm", "%lld sa": "%lldh", "%lld sa %lld dk": "%lldh %lldm",
        "Tonti'yi kutuya koy": "Put Tonti in the box",
        "Tonti'yi masaüstüne çıkar": "Bring Tonti to the desktop",
        "Tonti'nin balonları": "Tonti's speech bubbles",
        "Tonti başının üstündeki balonda ne yaptığını söyler. Sohbet içeriği okunmaz; yalnızca proje adı, süre ve token sayısı kullanılır.": "Tonti says what you're up to in a bubble above its head. Chat content is never read; only the project name, time and token counts are used.",
        "Günlük": "Diary", "Raporlar": "Reports", "senin kedin, senin günlüğün": "your cat, your diary",
        "Tonti bu sayfayı hazırlıyor.": "Tonti is preparing this page.",
        "Tonti yapay zekâyla çalıştığın oturumları ve attığın commit'leri buraya yazar. Sohbetlerin okunmaz.": "Tonti writes down your AI coding sessions and your commits here. Your chats are never read.",
        "Bu gün için not yok.": "Nothing logged this day.", "Claude ya da Codex ile çalıştığında Tonti buraya yazar.": "When you work with Claude or Codex, Tonti writes it here.",
        "Haftalık": "Weekly", "Aylık": "Monthly", "Yıllık": "Yearly",
        "Günlük rapor": "Daily report", "Haftalık rapor": "Weekly report", "Aylık rapor": "Monthly report", "Yıllık rapor": "Yearly report",
        "Ne üzerinde çalıştığını gün, hafta, ay ya da yıl olarak gör; standup notu ya da paylaşım için kopyala.": "See what you worked on by day, week, month or year; copy it for a standup note or to share.",
        "MARKDOWN KOPYALA": "COPY MARKDOWN", "RESMİ KOPYALA": "COPY IMAGE", "RESMİ KAYDET": "SAVE IMAGE", "Kopyalandı": "Copied", "Kaydedildi": "Saved",
        "Tonti not aldı · bilgisayarında, sohbetler okunmadan": "Noted by Tonti · on your Mac, chats never read",
        "ÇIKANLAR": "SHIPPED", "Bu dönemde kayıt yok.": "Nothing logged in this period.", "+%lld commit daha": "+%lld more commits", "%lld commit": "%lld commits", "BAŞKA DEPOLARDA": "OTHER REPOS", "Kapalı": "Off", "Limitlerin burada": "Your limits are here", "Menü çubuğunda 5 saatlik ve haftalık kalan limitini görürsün. Soldaki kısma basınca ayrıntılar açılır; sağdaki kutuya basınca ben kutuya girerim.": "The menu bar shows your 5-hour and weekly limits. Click the left part for details; click the box on the right and I'll hop inside.", "Bu benim, Tonti": "This is me, Tonti", "Üstüme gelip imleci gezdirirsen mırlarım. Enseden tutup istediğin yere taşıyabilirsin. Bana tıklarsan uygulamayı açarım.": "Hover over me and I purr. Pick me up by the scruff and put me anywhere. Click me and I'll open the app.", "Çalışırken not alırım": "I take notes while you work", "Claude ya da Codex ile çalışınca Günlük'e yazarım. Raporlar'da günlük, haftalık ve aylık özetin hazır olur; Kullanım'da token'ların. Sohbetlerini okumam.": "When you work with Claude or Codex I write it in the Diary. Reports has your daily, weekly and monthly summary; Usage has your tokens. I never read your chats.", "Her şey ayarlarda": "Everything is in settings", "Limit göstergesini, balonları ve bilgisayar günlüğünün her katmanını tek tek açıp kapatabilirsin. İzin gereken şeyleri sen onaylarsın; kendiliğinden hiçbir izin istemem.": "You can turn the limit meter, the bubbles and every layer of the computer log on or off one by one. You approve anything that needs permission; I never ask on my own.", "Hepsi bu Mac'te kalır": "It all stays on this Mac", "Burada neyi not alıp neyi asla almadığımı görürsün. Sunucum ve hesabım yok; hiçbir şey gönderilmez. Bu turu Ayarlar → Hakkında'dan yeniden görebilirsin.": "Here you can see what I note and what I never take. I have no server and no account; nothing is sent anywhere. You can replay this tour in Settings → About.", "ATLA": "SKIP", "İLERİ →": "NEXT →", "BAŞLAYALIM": "LET'S GO", "Tanıtım turu": "Intro tour", "Menü çubuğunu, kediyi ve ayarları kısaca gösteren beş adımlık tur.": "A five-step tour of the menu bar, the cat and the settings.", "Turu göster": "Show tour", "Beni sevdiysen GitHub'da bir yıldız verir misin?": "If you like me, would you give Meowly a star on GitHub?", "Böylece başkaları da Meowly'yi bulur. Yalnızca bir kez soruyorum.": "It helps others find Meowly too. I'm only asking once.", "⭐ YILDIZ VER": "⭐ STAR", "SONRA": "LATER", "Bir daha sorma": "Don't ask again", "Meowly'yi beğendiysen bir yıldız başkalarının da bulmasına yardım eder. Hata ya da fikir için de buraya yazabilirsin.": "If you like Meowly, a star helps others find it. You can also report bugs or share ideas there.", "⭐ Yıldız ver": "⭐ Star", "Hata bildir": "Report a bug", "KAYDET": "SAVE", "TAMAM": "OK", "Kedinin adı": "Cat's name", "Masaüstündeki kedin ve uygulamadaki metinler bu adı kullanır.": "Your desktop cat and the app's texts use this name.", "Merhaba! Benim adım ne olsun?": "Hi! What should my name be?", "Sonra ayarlardan değiştirebilirsin.": "You can change it later in settings.", "pencere başlıkları": "window titles", "siteler": "sites", "takvim": "calendar", "İzin gerekenler: %@. Ne aldığını ve ne almadığını ayarlarda tek tek anlattım.": "Needs permission: %@. Settings explain exactly what is and isn't recorded.", "Etkin": "On", "Yapay zekâ araçlarının token kullanımı, tahmini maliyeti ve geçmişi. Yalnızca sayılar okunur.": "Token usage, estimated cost and history of your AI tools. Only numbers are read.", "+%lld proje daha": "+%lld more projects",
        "proje": "projects", "Çalışma": "Worked", "Oturum": "Sessions", "Commit": "Commits", "Proje": "Projects",
        "İSTATİSTİKLER": "STATS", "Limit en son %@ tarihinde ölçüldü; birkaç dakika içinde yeniden ölçülür.": "Limit last measured at %@; it will be re-measured in a few minutes.",
        " Son başarılı mesaj: %@.": " Last successful message: %@.", " Henüz mesaj gönderilmedi.": " No message sent yet.",
        "Bilgisayarda": "On the computer", "Toplantı": "Meetings", "toplantı": "meetings", "BİLGİSAYARDA": "ON THE COMPUTER",
        "YAPAY ZEKÂ": "AI", "BİLGİSAYAR": "COMPUTER", "SAAT SAAT": "HOUR BY HOUR", "TAKVİM": "CALENDAR", "DOSYA DEĞİŞİKLİKLERİ": "FILE CHANGES",
        "+%lld daha": "+%lld more", "Bilgisayar günlüğü": "Computer log",
        "Bu gün için bilgisayar kaydı yok.": "No computer activity for this day.", "Tonti, bilgisayar günlüğü açıkken uygulamaları not alır.": "Tonti notes your apps while the computer log is on.",
        "Tonti yapay zekâ oturumlarını, commit'lerini ve bilgisayarda ne yaptığını buraya yazar. Sohbetlerin ve ekranın okunmaz.": "Tonti writes down your AI sessions, commits and what you do on the computer. Your chats and screen are never read.",
        "Tonti bilgisayarda ne yaptığını not alır; Günlük ve Raporlar bununla dolar. Her şey yalnızca bu Mac'te kalır, hiçbir yere gönderilmez.": "Tonti notes what you do on the computer to fill your diary and reports. Everything stays on this Mac and is never sent anywhere.",
        "NOT ALIR": "NOTES", "ASLA ALMAZ": "NEVER TAKES",
        "hangi uygulamada ne kadar kaldığın": "how long you spend in each app", "pencere başlıkları (dosya, belge adı)": "window titles (file, document name)",
        "tarayıcıda yalnızca site adı (github.com gibi)": "only the site name in browsers (like github.com)", "projelerinde kaç dosya değişikliği oldu": "how many file changes happened in your projects",
        "takvimdeki toplantıların": "your calendar meetings", "ekran görüntüsü ya da ekran kaydı": "screenshots or screen recordings",
        "bastığın tuşlar, yazdığın metinler": "your keystrokes or what you type", "sayfa içerikleri, tam adresler, sohbetler": "page contents, full URLs, chats",
        "dosyaların içeriği": "file contents", "parola uygulamalarının pencere adları": "window names of password managers",
        "Uygulama süreleri": "App time", "Pencere başlıkları": "Window titles", "Tarayıcı siteleri": "Browser sites", "Dosya değişiklikleri": "File changes", "Takvim": "Calendar",
        "Öndeki uygulama değiştikçe not alınır; 5 dakika fare ve klavye kullanmazsan boşta sayılır. İzin gerekmez.": "Noted whenever the front app changes; 5 minutes without mouse or keyboard counts as idle. No permission needed.",
        "Hangi dosyada ya da belgede çalıştığını gösterir (\"CatPet.swift — ullage-cat\" gibi). Erişilebilirlik izni ister; bu izin yalnızca öndeki pencerenin adını okumak için kullanılır, tuşlarına ya da ekranına bakmaz. Tarayıcılarda başlık saklanmaz.": "Shows which file or document you worked on (like \"CatPet.swift — ullage-cat\"). Needs Accessibility permission, used only to read the front window's name; it never looks at your keys or screen. Browser titles are not stored.",
        "Tarayıcıda hangi sitede ne kadar kaldığını yalnızca alan adıyla tutar (youtube.com gibi). Sayfa başlığı, tam adres ve içerik saklanmaz; gizli/incognito pencereler atlanır. Chrome, Arc, Brave, Edge ve Safari için tarayıcı başına bir kez izin sorulabilir; Firefox desteklenmez.": "Keeps how long you stay on each site by domain only (like youtube.com). Page titles, full URLs and contents are not stored; private/incognito windows are skipped. Chrome, Arc, Brave, Edge and Safari may each ask once; Firefox is not supported.",
        "Yapay zekâ araçlarıyla çalıştığın proje klasörlerinde saatte kaç değişiklik olduğunu sayar. Dosya adı ve içeriği saklanmaz. macOS ilk seferde klasör erişimi sorabilir.": "Counts changes per hour in the project folders you use with AI tools. File names and contents are not stored. macOS may ask for folder access once.",
        "Günlükte ve raporlarda toplantılarını gösterir. Takvimi yalnızca okur, değiştirmez. Paylaşılan raporlara toplantı adları girmez, yalnızca toplam süre.": "Shows your meetings in the diary and reports. Read-only. Shared reports include only total meeting time, never meeting names.",
        "Tüm bilgisayar kayıtları silinecek. Emin misin?": "All computer log records will be deleted. Are you sure?", "SİL": "DELETE", "VAZGEÇ": "CANCEL", "Silindi": "Deleted",
        "BİLGİSAYAR KAYITLARINI SİL": "DELETE COMPUTER LOG", "Yapay zekâ oturumları ve commit'ler etkilenmez.": "AI sessions and commits are not affected.",
        "Çalışıyor": "Working", "İzin gerekmez": "No permission needed", "İZİN VER": "ALLOW",
        "Tonti daha fazlasını not alabilir": "Tonti can take better notes", "AYARLARA GİT": "OPEN SETTINGS",
        "Pencere başlıkları, siteler ve takvim için izin gerekiyor. Ne aldığını ve ne almadığını ayarlarda tek tek anlattım.": "Window titles, sites and calendar need permission. Settings explain exactly what is and isn't collected.",
        "Geliştirme": "Development", "Tarayıcı": "Browser", "İletişim": "Communication", "Tasarım ve medya": "Design & media", "Verimlilik": "Productivity", "Okuma": "Reading", "Eğlence": "Entertainment", "Araçlar": "Utilities", "Diğer": "Other", "Soldaki kısım menüyü açar, sağdaki kutu Tonti'yi kutuya koyar ya da masaüstüne çıkarır.": "The left part opens the menu; the box on the right puts Tonti in the box or back on the desktop.",
        "Masaüstünde yaşayan kedin Tonti çalışmanı not alır, raporunu hazırlar; yapay zekâ limitlerin menü çubuğunda.": "Tonti, the cat on your desktop, takes notes of your work and prepares your reports; your AI limits sit in the menu bar.", "daha az göster": "show less", "GitHub kartı": "GitHub card",
        "Pencere, menü ve menü çubuğundaki gösterge birlikte değişir.": "The window, the menu and the menu bar indicator change together.",
        "Token sayılarına önbellekten okunan (cache) token'lar da eklenir. Kapalıyken yalnızca yeni okunan ve yazılan token'lar sayılır; sayılar daha küçük ve gerçek işe daha yakın olur.": "Adds tokens read from cache to token counts. When off, only newly read and written tokens are counted; numbers are smaller and closer to the actual work.",
        "O GÜN": "THAT DAY", "çalışma": "worked", "oturum": "sessions", "commit": "commits", "SON 15 HAFTA": "LAST 15 WEEKS", "BU HAFTA": "THIS WEEK", "Projesiz": "No project",
        "Günlükte commit'leri göster": "Show commits in the diary",
        "Oturum sırasında proje klasörüne attığın commit başlıkları Günlük'e ve raporlara yazılır. Yalnızca senin commit'lerin okunur, bilgisayardan çıkmaz. İlk açılışta macOS klasör izni sorabilir.": "Commit titles you make in the project folder during a session go into the diary and reports. Only your own commits are read and nothing leaves your Mac. macOS may ask for folder access the first time.",
        "Çalışırken not alır (proje ve süre)": "Takes notes while you work (project and time)",
        "Uzun oturumda mola hatırlatır": "Suggests a break in long sessions",
        "Günün ilk açılışında dünün özeti": "Yesterday's summary on the first open of the day",
        "Günün token eşiklerini kutlar": "Celebrates daily token milestones",
        "Sevilince mırlar": "Purrs when petted",
        "Mırr…": "Purr…",
        "claude komutu bulunamadı": "claude command not found", "Zaman aşımı": "Timed out", "Limit henüz ölçülmedi": "Limit not measured yet", "Yalnızca Claude Code, klasör adıyla": "Claude Code only, by folder name", "90 gün": "90 days", "Bu araçta kalan limit okunmuyor; yalnızca token sayılıyor.": "This tool has no limit to read; only tokens are counted.", "Claude girişi süresi dolmuş: Terminal'de claude yazıp /login yap.": "Claude sign-in expired: run claude in Terminal and type /login.", "Yeni pencere başlatıldı": "New window started",
        "Başlatılıyor…": "Starting…",
        // Sağlayıcı notları ve hataları
        "Token: ~/.claude kayıtlarından (Claude Code ve Desktop), cache okumaları hariç.": "Tokens: from ~/.claude logs (Claude Code and Desktop), cache reads excluded.",
        "Limit: Anthropic kullanım uç noktasından (resmi olmayan, Claude Code oturumuyla).": "Limit: from Anthropic's usage endpoint (unofficial, via the Claude Code session).",
        "Anthropic hız sınırı uyguladı; biraz sonra yeniden denenecek": "Anthropic rate-limited the request; will retry shortly",
        "Keychain okunamadı": "Could not read the Keychain",
        "Claude Code oturumu Keychain'de bulunamadı ya da erişim reddedildi": "Claude Code session not found in the Keychain, or access was denied",
        "Oturum süresi dolmuş; Claude Code'u açınca yenilenir": "Session expired; it renews when you open Claude Code",
        "Anthropic hız sınırı uyguladı (429)": "Anthropic rate-limited the request (429)",
        "Yanıtta limit alanı yok": "No limit field in the response", "Ağ hatası": "Network error",
        "Codex kayıtları sıkıştırılmış (.zst). Okumak için zstd gerekli: brew install zstd": "Codex logs are compressed (.zst). zstd is needed to read them: brew install zstd",
        "Codex kayıtlarında limit bilgisi bulunamadı.": "No limit information found in the Codex logs.",
        "Son kayıttan beri 5 saatlik pencere sıfırlandı; yeni kullanım olmadığı için %100 gösteriliyor.": "The 5-hour window has reset since the last log entry; showing 100% as there has been no new usage.",
        "Haftalık pencere sıfırlandı; yeni kullanım olmadığı için %100 gösteriliyor.": "The weekly window has reset; showing 100% as there has been no new usage.",
        "Token alanları (girdi, cache yazma, çıktı, cache okuma) kayıt yapısından çıkarıldı; Antigravity resmi bir şema yayınlamıyor, bu yüzden tahminidir.":
            "Token fields (input, cache write, output, cache read) were inferred from the log structure; Antigravity publishes no official schema, so these are estimates.",
        "Yerel konuşma kayıtlarında token bulunamadı.": "No tokens found in the local conversation logs.",
        "Limit: Antigravity'nin yerel dil sunucusundan (IDE açıkken).": "Limit: from Antigravity's local language server (while the IDE is open).",
        "Antigravity IDE çalışmıyor; açınca okunur": "Antigravity IDE is not running; it is read once you open it", "Yerel sunucudan kota okunamadı": "Could not read the quota from the local server",
        "Gemini CLI oturumu bulunamadı (~/.gemini/oauth_creds.json)": "Gemini CLI session not found (~/.gemini/oauth_creds.json)",
        "Oturum süresi dolmuş ve yenileme anahtarı yok": "Session expired and there is no refresh token",
        "Bu Google hesabında Gemini Code Assist lisansı yok, Google kota bilgisini vermiyor (SUBSCRIPTION_REQUIRED)": "This Google account has no Gemini Code Assist license, so Google gives no quota information (SUBSCRIPTION_REQUIRED)",
        "Limit: Google Code Assist kota API'sinden (Gemini CLI oturumuyla). Gemini'de 5 saatlik/haftalık pencere yok, günlük kota var.": "Limit: from the Google Code Assist quota API (via the Gemini CLI session). Gemini has no 5-hour/weekly window, only a daily quota.",
        "Token sayısı yerelde tutulmuyor.": "Token counts are not stored locally.",
        // Notlar
        "Bu hızla biter: ": "At this pace runs out: ", "Sonraki otomatik mesaj: %@": "Next auto-message: %@",
        "5 saatlik %@": "5-hour %@", "haftalık %@": "weekly %@",
    ]
}


// MARK: - Kedinin adı

/// Kedinin adı kullanıcıya göre değişir (varsayılan Tonti). Metinlerdeki "Tonti" bu adla değiştirilir;
/// Türkçe ekler ('nin, 'yi) ünlü uyumuna göre yeniden kurulur.
enum CatName {
    static let key = "catName"
    static var current: String {
        let n = (UserDefaults.standard.string(forKey: key) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        return n.isEmpty ? "Tonti" : n
    }
    static var isSet: Bool { UserDefaults.standard.string(forKey: key) != nil }

    static func apply(_ s: String) -> String {
        guard s.contains("Tonti") else { return s }
        let n = current
        guard n != "Tonti" else { return s }
        return s.replacingOccurrences(of: "Tonti'nin", with: n + genitive(n))
            .replacingOccurrences(of: "Tonti'yi", with: n + accusative(n))
            .replacingOccurrences(of: "Tonti's", with: n + "'s")
            .replacingOccurrences(of: "Tonti", with: n)
    }

    private static let vowels = Set("aeıioöuüAEIİOÖUÜ")
    /// Son ünlüye göre dar ünlü: ı, i, u, ü.
    private static func narrow(_ n: String) -> String {
        let v = n.lowercased(with: Locale(identifier: "tr")).last { vowels.contains($0) } ?? "e"
        switch v { case "a", "ı": return "ı"; case "o", "u": return "u"; case "ö", "ü": return "ü"; default: return "i" }
    }
    private static func endsWithVowel(_ n: String) -> Bool { n.last.map { vowels.contains($0) } ?? false }
    static func genitive(_ n: String) -> String { "'" + (endsWithVowel(n) ? "n" : "") + narrow(n) + "n" }
    static func accusative(_ n: String) -> String { "'" + (endsWithVowel(n) ? "y" : "") + narrow(n) }
}
