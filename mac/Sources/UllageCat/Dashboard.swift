import SwiftUI

final class FirstMouseHostingView: NSHostingView<AnyView> {
    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
}

@MainActor
enum DashboardWindow {
    private static var window: NSWindow?
    /// Tur, pencere içindeki öğeleri gösterebilsin diye açık pencere.
    static var current: NSWindow? { window }
    private static var closeObserver: NSObjectProtocol?

    static func show(state: AppState) {
        if window == nil {
            // İlk tıklama da düğmeye gider (pencere arkadayken "İzin ver"e basmak boşa gitmesin).
            let host = FirstMouseHostingView(rootView: AnyView(TontiRoot().environmentObject(state)))
            host.sizingOptions = []   // pencere boyunu kullanıcı belirler; uzun içerik pencereyi büyütmez
            let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1080, height: 720),
                             styleMask: [.titled], backing: .buffered, defer: false)
            w.contentView = host
            w.title = AppInfo.name
            // Başlık çubuğu ayrı gri bir şerit olmasın: içerik altına uzanır, renk temadan gelir.
            w.styleMask = [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView]
            w.titlebarAppearsTransparent = true
            w.titleVisibility = .hidden
            w.isMovableByWindowBackground = true
            w.setContentSize(NSSize(width: 1080, height: 720))
            w.minSize = NSSize(width: 860, height: 560)
            w.center()
            w.isReleasedWhenClosed = false
            // Kapanınca pencere ve SwiftUI içeriği tamamen bırakılır; bellek açılıştaki düzeyine döner.
            closeObserver = NotificationCenter.default.addObserver(forName: NSWindow.willCloseNotification, object: w, queue: .main) { _ in
                MainActor.assumeIsolated {
                    if let o = closeObserver { NotificationCenter.default.removeObserver(o) }
                    closeObserver = nil
                    window?.contentView = nil
                    window = nil
                    // Boşalan belleği bekletmeden işletim sistemine geri ver.
                    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { malloc_zone_pressure_relief(nil, 0) }
                }
            }
            window = w
        }
        applyAppearance(dark: state.tontiDark)
        NSApp.activate(ignoringOtherApps: true)
        window?.makeKeyAndOrderFront(nil)
    }

    static func applyAppearance(dark: Bool) {
        window?.appearance = NSAppearance(named: dark ? .darkAqua : .aqua)
        window?.backgroundColor = NSColor(TC.side)
    }
}

enum DashPage: String, CaseIterable, Identifiable {
    case history, providers, sync, menubar, appearance, automation, about
    var id: String { rawValue }

    var title: String {
        switch self {
        case .history: "Kullanım"
        case .providers: "Sağlayıcılar"
        case .sync: "Senkronizasyon"
        case .menubar: "Menü çubuğu"
        case .appearance: "Görünüm"
        case .automation: "Bildirim ve otomasyon"
        case .about: "Hakkında"
        }
    }

    var icon: String {
        switch self {
        case .history: "chart.bar.fill"
        case .providers: "square.grid.2x2.fill"
        case .sync: "arrow.triangle.2.circlepath"
        case .menubar: "menubar.rectangle"
        case .appearance: "paintpalette.fill"
        case .automation: "bell.badge.fill"
        case .about: "info.circle.fill"
        }
    }
}

struct DashboardView: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        let t = state.theme
        HStack(spacing: 0) {
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 8) {
                    LogoMark(size: 30, palette: t.logo)
                    Text(AppInfo.name).font(t.font(17, .bold))
                }
                .padding(.bottom, 14).padding(.horizontal, 6)
                ForEach(DashPage.allCases) { p in
                    let sel = state.dashPage == p.rawValue
                    HStack(spacing: 9) {
                        Image(systemName: p.icon).frame(width: 18).foregroundStyle(sel ? t.accent : t.sub)
                        Text(L.t(p.title)).font(t.font(13, sel ? .semibold : .regular))
                        Spacer()
                    }
                    .padding(.horizontal, 10).padding(.vertical, 8)
                    .background(sel ? t.accent.opacity(0.16) : Color.clear, in: RoundedRectangle(cornerRadius: max(6, t.radius - 4)))
                    .contentShape(Rectangle())
                    .contentShape(Rectangle())
                    .onTapGesture { state.dashPage = p.rawValue }
                }
                Spacer()
            }
            .padding(14)
            .frame(width: 230)
            .background(t.card.opacity(0.6))

            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text(L.t(DashPage(rawValue: state.dashPage)?.title ?? "")).font(t.font(26, .bold))
                    page
                }
                .padding(24)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .foregroundStyle(t.text)
        .background(t.bg)
        .themed(t)
        .environment(\.locale, L.locale)
    }

    @ViewBuilder private var page: some View {
        switch DashPage(rawValue: state.dashPage) ?? .history {
        case .history: HistoryPage()
        case .providers: ProvidersPage()
        case .sync: SyncPage()
        case .menubar: MenuBarPage()
        case .appearance: AppearancePage()
        case .automation: AutomationPage()
        case .about: AboutPage()
        }
    }
}

struct SettingRow<C: View>: View {
    @EnvironmentObject var state: AppState
    let title: String
    var subtitle: String? = nil
    @ViewBuilder var control: C

    var body: some View {
        let t = state.theme
        HStack(alignment: .center, spacing: 16) {
            VStack(alignment: .leading, spacing: 3) {
                Text(title).font(t.font(13, .semibold))
                if let s = subtitle {
                    Text(s).font(t.font(11)).foregroundStyle(t.sub).fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: 12)
            control
        }
        .card(t, padding: 14)
    }
}

// MARK: Sayfalar

struct ProvidersPage: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        let t = state.theme
        SetupCheckCard()
        Text(L.t("Menü çubuğunda ve popover'da görünecek sağlayıcıları seç.")).font(t.font(12)).foregroundStyle(t.sub)
        ForEach(Provider.allCases) { p in
            let u = state.usage[p] ?? .loading()
            HStack(spacing: 14) {
                BrandBadge(provider: p, size: 34)
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 8) {
                        Text(p.name).font(t.font(14, .semibold))
                        Text(L.t("Limit")).font(t.font(10)).foregroundStyle(t.sub)
                        ConfidenceTag(c: u.limitConfidence)
                        Text(L.t("Token")).font(t.font(10)).foregroundStyle(t.sub)
                        ConfidenceTag(c: u.tokenConfidence)
                    }
                    ForEach(u.notes, id: \.self) { n in
                        Text(L.t(n)).font(t.font(10)).foregroundStyle(t.sub).fixedSize(horizontal: false, vertical: true)
                    }
                }
                Spacer()
                Toggle("", isOn: state.binding(for: p)).toggleStyle(PxSwitch()).labelsHidden()
            }
            .card(t, padding: 14)
        }
    }
}

struct StyleChoice: View {
    @EnvironmentObject var state: AppState
    let style: MetricStyle
    let selected: Bool
    let action: () -> Void

    var body: some View {
        let t = state.theme
        VStack(spacing: 8) {
            // Önizleme menü çubuğundaki gibi: açık temada beyaz kutu, koyu temada siyah kutu.
            let look = MenuBarLabel.look(t, isDark: state.tontiDark)
            ZStack {
                Rectangle().fill(Color(white: state.tontiDark ? 0.30 : 0.80))
                MetricGlyph(left: 0.57, style: style, tag: nil, text: look.number)
                    .padding(.horizontal, 10).padding(.vertical, 3)
                    .background(look.fill, in: RoundedRectangle(cornerRadius: 3))
            }
            .frame(height: 44)
            Text(L.t(style.label)).font(t.font(11, selected ? .semibold : .regular))
        }
        .padding(8)
        .frame(maxWidth: .infinity)
        .background(t.card, in: RoundedRectangle(cornerRadius: t.radius))
        .overlay(RoundedRectangle(cornerRadius: t.radius).stroke(selected ? t.accent : t.border, lineWidth: selected ? 2 : max(0.5, t.borderW)))
        .contentShape(Rectangle())
        .onTapGesture(perform: action)
    }
}

struct MenuBarPage: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        let t = state.theme
        VStack(alignment: .leading, spacing: 8) {
            Text(L.t("ÖNİZLEME")).font(t.font(10, .bold)).foregroundStyle(t.sub)
            HStack(spacing: 14) {
                if let img = MenuBarController.shared.image {
                    Image(nsImage: img)
                        .padding(.horizontal, 14).padding(.vertical, 7)
                        .background(Color(white: state.tontiDark ? 0.30 : 0.80))
                }
                Text(L.t("Soldaki kısım menüyü açar, sağdaki kutu Tonti'yi kutuya koyar ya da masaüstüne çıkarır."))
                    .font(t.font(11)).foregroundStyle(t.sub).fixedSize(horizontal: false, vertical: true)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .card(t, padding: 14)

        SettingRow(title: L.t("Etiketleri göster"), subtitle: L.t("5 saatlik için \"5s\", haftalık için \"H\", günlük kota için \"G\". Aynı yüzdeler olsa bile hangisi olduğu anlaşılır.")) {
            Toggle("", isOn: $state.showTags).toggleStyle(PxSwitch()).labelsHidden()
        }

        SettingRow(title: L.t("5 saatlik limit"), subtitle: L.t("Menü çubuğunda göster.")) {
            Toggle("", isOn: $state.showFive).toggleStyle(PxSwitch()).labelsHidden()
        }
        if state.showFive {
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 10), count: 3), spacing: 10) {
                ForEach(MetricStyle.allCases) { s in
                    StyleChoice(style: s, selected: state.styleFive == s) { state.styleFive = s }
                }
            }
        }

        SettingRow(title: L.t("Haftalık limit"), subtitle: L.t("Menü çubuğunda göster.")) {
            Toggle("", isOn: $state.showWeek).toggleStyle(PxSwitch()).labelsHidden()
        }
        if state.showWeek {
            LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 10), count: 3), spacing: 10) {
                ForEach(MetricStyle.allCases) { s in
                    StyleChoice(style: s, selected: state.styleWeek == s) { state.styleWeek = s }
                }
            }
        }
        Text(L.t("Hangi sağlayıcıların görüneceğini Sağlayıcılar sayfasından seçersin.")).font(t.font(11)).foregroundStyle(t.sub)
    }
}

struct ThemeChoice: View {
    @EnvironmentObject var state: AppState
    let theme: Theme

    var body: some View {
        let selected = state.themeId == theme.id
        let t = state.theme
        VStack(alignment: .leading, spacing: 8) {
            ZStack(alignment: .leading) {
                RoundedRectangle(cornerRadius: theme.radius).fill(theme.bg)
                VStack(alignment: .leading, spacing: 6) {
                    RoundedRectangle(cornerRadius: max(2, theme.radius - 8)).fill(theme.card)
                        .overlay(RoundedRectangle(cornerRadius: max(2, theme.radius - 8)).stroke(theme.border, lineWidth: theme.borderW))
                        .frame(height: 22)
                    HStack(spacing: 6) {
                        Capsule().fill(theme.accent).frame(width: 46, height: 8)
                        Spacer()
                        Text("62%").font(theme.font(11, .bold)).foregroundStyle(theme.text)
                    }
                }
                .padding(10)
            }
            .frame(height: 74)
            .overlay(RoundedRectangle(cornerRadius: theme.radius).stroke(theme.border, lineWidth: theme.borderW))
            Text(L.t(theme.name)).font(t.font(12, selected ? .bold : .medium))
        }
        .padding(8)
        .background(t.card, in: RoundedRectangle(cornerRadius: t.radius + 2))
        .overlay(RoundedRectangle(cornerRadius: t.radius + 2).stroke(selected ? t.accent : Color.clear, lineWidth: 2))
        .contentShape(Rectangle())
        .onTapGesture { state.themeId = theme.id }
    }
}

struct AppearancePage: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        let t = state.theme
        SettingRow(title: L.t("Görünüm"), subtitle: L.t("Pencere, menü ve menü çubuğundaki gösterge birlikte değişir.")) {
            ThemedSegmented(t: t, items: [(L.t("Açık"), false), (L.t("Koyu"), true)], selection: $state.tontiDark)
        }
        SettingRow(title: L.t("Cache dahil"), subtitle: L.t("Token sayılarına önbellekten okunan (cache) token'lar da eklenir. Kapalıyken yalnızca yeni okunan ve yazılan token'lar sayılır; sayılar daha küçük ve gerçek işe daha yakın olur.")) {
            Toggle("", isOn: $state.includeCache).toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Kedinin adı"), subtitle: L.t("Masaüstündeki kedin ve uygulamadaki metinler bu adı kullanır.")) {
            CatNameField()
        }
        CatBubbleSettings()
        SettingRow(title: L.t("Dil"), subtitle: L.t("Arayüz dilini seç. Değişiklik hemen uygulanır.")) {
            ThemedSegmented(t: t, items: [(L.t("Sistem"), "system"), ("Türkçe", "tr"), ("English", "en")], selection: $state.language)
        }
        SettingRow(title: L.t("Günlükte commit'leri göster"), subtitle: L.t("Oturum sırasında proje klasörüne attığın commit başlıkları Günlük'e ve raporlara yazılır. Yalnızca senin commit'lerin okunur, bilgisayardan çıkmaz. İlk açılışta macOS klasör izni sorabilir.")) {
            Toggle("", isOn: Binding(get: { UserDefaults.standard.object(forKey: "journalCommits") as? Bool ?? true },
                                     set: { UserDefaults.standard.set($0, forKey: "journalCommits") })).toggleStyle(PxSwitch()).labelsHidden()
        }
    }
}

struct AutomationPage: View {
    @EnvironmentObject var state: AppState

    private var starterNote: String {
        guard state.starterOn else { return "" }
        var out = ""
        if let at = state.starterFireAt { out = L.f(" Sonraki otomatik mesaj: %@.", AppState.whenText(at)) }
        else { out = L.t(" Sıfırlanma saati bekleniyor.") }
        // Mesaj sohbet kaydı bırakmaz (Claude uygulamasında görünmez); çalıştığı buradan görülür.
        let last = UserDefaults.standard.double(forKey: WindowStarter.lastKey)
        out += last > 0 ? L.f(" Son başarılı mesaj: %@.", AppState.whenText(Date(timeIntervalSince1970: last))) : L.t(" Henüz mesaj gönderilmedi.")
        if !state.starterStatus.isEmpty { out += " (" + state.starterStatus + ")" }
        return out
    }

    var body: some View {
        let t = state.theme
        SettingRow(title: L.t("Limit bildirimleri"), subtitle: L.t("Kalan %15 ve %5'in altına inince, ayrıca limit yenilenince bildirim gönderir.")) {
            Toggle("", isOn: $state.notificationsOn).toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Bitiş tahmini notu"), subtitle: L.t("Kartların altında \"Bu hızla biter: 5 saatlik 19:40\" satırını gösterir.")) {
            Toggle("", isOn: $state.showForecast).toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Sonraki otomatik mesaj notu"), subtitle: L.t("Claude kartında pencere başlatıcının bir sonraki mesaj saatini gösterir. Başlatıcı kapalıysa zaten görünmez.")) {
            Toggle("", isOn: $state.showNextStart).toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Haftalık özet"), subtitle: L.t("Pazartesi sabahı geçen haftanın toplam token'ını, en yoğun gününü ve en çok kullandığın aracı bildirir. Bildirimler kapalıysa gelmez.")) {
            Toggle("", isOn: $state.weeklySummaryOn).toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Limiti Claude Code komutuyla ölç"), subtitle: L.t("Anthropic'in adresi hız sınırına takılınca ya da değer eskiyince, Claude Code komutu tek satırlık küçük bir Haiku isteği atıp güncel limiti ölçer. Terminal gerekmez, Claude uygulamasını kullananlar için de çalışır. Yalnızca yeni Claude kullanımı olunca ve en sık 5 dakikada bir yapılır; çok az token harcar.")) {
            Toggle("", isOn: $state.cliProbe).toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Claude limitini Claude Code'dan al"), subtitle: L.t("Claude Code, her cevaptan sonra limit yüzdesini yerel bir dosyaya yazar; Meowly onu okur. Ağ gerekmez, Anthropic'in hız sınırına takılmaz. Açınca ~/.claude/settings.json dosyasına küçük bir durum satırı betiği eklenir (önce yedeği alınır, başka bir durum satırı varsa dokunulmaz). Yalnızca Claude Code çalışırken güncellenir.") + (state.statusLineMessage.isEmpty ? "" : " — " + L.t(state.statusLineMessage))) {
            Toggle("", isOn: $state.statusLineOn).toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Girişte otomatik başlat"), subtitle: L.t("Mac'e giriş yapınca uygulama kendiliğinden açılır.")) {
            Toggle("", isOn: Binding(get: { state.launchAtLogin }, set: { state.setLaunchAtLogin($0) }))
                .toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Claude oturumunu otomatik yenile"), subtitle: L.t("Claude Code oturumunun süresi dolarsa (yaklaşık 8 saatte bir) limit değeri eski kalır. Bu açıkken Meowly, Claude Code'a tek satırlık bir istek attırıp oturumunu yeniletir. Çok az token harcar, anahtarlarına dokunmaz.")) {
            Toggle("", isOn: $state.refreshLogin).toggleStyle(PxSwitch()).labelsHidden()
        }
        SettingRow(title: L.t("Claude pencere başlatıcı"), subtitle: "5 saatlik pencere bittiğinde Haiku'ya tek satırlık bir istek gönderip yeni pencereyi erken başlatır. Ek limit vermez, yalnızca pencerenin başlama saatini belirler. Her istek çok az token harcar. İstek sohbet geçmişine kaydedilmez, bu yüzden Claude uygulamasında görünmez.\(starterNote)") {
            Toggle("", isOn: $state.starterOn).toggleStyle(PxSwitch()).labelsHidden()
        }
    }
}

struct AboutPage: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        let t = state.theme
        HStack(spacing: 16) {
            CatSprite(name: CatAnim.has("content") ? "content" : "idle_0", height: 58)
                .frame(width: 72, height: 72)
                .background(RoundedRectangle(cornerRadius: 16).fill(TC.paper))
                .overlay(RoundedRectangle(cornerRadius: 16).strokeBorder(TC.ink, lineWidth: 2))
            VStack(alignment: .leading, spacing: 4) {
                Text(AppInfo.name).font(t.font(22, .bold))
                Text(L.t("Masaüstünde yaşayan kedin Tonti çalışmanı not alır, raporunu hazırlar; yapay zekâ limitlerin menü çubuğunda."))
                    .font(t.font(12)).foregroundStyle(t.sub).fixedSize(horizontal: false, vertical: true)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .card(t, padding: 16)
        UpdateCard()
        SettingRow(title: L.t("Tanıtım turu"), subtitle: L.t("Menü çubuğunu, kediyi ve ayarları kısaca gösteren beş adımlık tur.")) {
            Button(L.t("Turu göster")) { Tour.start(state) }.buttonStyle(PxButtonStyle())
        }
        SettingRow(title: L.t("GitHub"), subtitle: L.t("Meowly'yi beğendiysen bir yıldız başkalarının da bulmasına yardım eder. Hata ya da fikir için de buraya yazabilirsin.")) {
            HStack(spacing: 8) {
                Button(L.t("⭐ Yıldız ver")) { NSWorkspace.shared.open(StarPrompt.repoURL) }.buttonStyle(PxButtonStyle())
                Button(L.t("Hata bildir")) { NSWorkspace.shared.open(StarPrompt.issuesURL) }.buttonStyle(PxButtonStyle())
            }
        }
        VStack(alignment: .leading, spacing: 10) {
            Text(L.t("Veri doğruluğu etiketleri")).font(t.font(13, .semibold))
            HStack(spacing: 8) { ConfidenceTag(c: .exact); Text(L.t("Sağlayıcının kendi verisi ya da kayıtlardan birebir.")).font(t.font(12)).foregroundStyle(t.sub) }
            HStack(spacing: 8) { ConfidenceTag(c: .estimate); Text(L.t("Tahmin ya da kayıt yapısından çözümlenmiş alanlar.")).font(t.font(12)).foregroundStyle(t.sub) }
            HStack(spacing: 8) { ConfidenceTag(c: .none); Text(L.t("Kaynak yok ya da erişilemedi; uydurma değer gösterilmez.")).font(t.font(12)).foregroundStyle(t.sub) }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .card(t, padding: 16)
        VStack(alignment: .leading, spacing: 8) {
            Text(L.t("Veri kaynakları")).font(t.font(13, .semibold))
            Text(L.t("Claude: ~/.claude kayıtları (token), Anthropic kullanım uç noktası (limit)")).font(t.font(12)).foregroundStyle(t.sub)
            Text(L.t("Codex: ~/.codex oturum kayıtları (limit ve token)")).font(t.font(12)).foregroundStyle(t.sub)
            Text(L.t("Antigravity: yerel konuşma veritabanı (token), yerel dil sunucusu (limit)")).font(t.font(12)).foregroundStyle(t.sub)
            Text(L.t("Gemini: Google Code Assist kota API'si (Gemini CLI oturumuyla)")).font(t.font(12)).foregroundStyle(t.sub)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .card(t, padding: 16)
    }
}


struct SyncPage: View {
    @EnvironmentObject var state: AppState

    private static let options: [(String, Int)] = [
        ("15 saniye", 15), ("30 saniye", 30), ("1 dakika", 60), ("2 dakika", 120), ("5 dakika", 300),
        ("10 dakika", 600), ("15 dakika", 900), ("Yalnızca elle", 0),
    ]

    var body: some View {
        let t = state.theme
        SettingRow(title: L.t("Otomatik yenileme"), subtitle: L.t("Verilerin arka planda ne sıklıkla okunacağı. Yerel kayıtlar (token geçmişi, Codex, Antigravity) her yenilemede okunur, 15 saniye bile sorun olmaz.")) {
            PxMenu(items: Self.options.map { (L.t($0.0), $0.1) }, selection: $state.refreshInterval)
        }
        SettingRow(
            title: L.t("Şimdi senkronize et"),
            subtitle: state.syncing ? L.t("Okunuyor…") : (state.updatedAt.map { L.f("Son senkron: %@", Self.stamp($0)) } ?? L.t("Henüz okunmadı."))
        ) {
            Button(L.t("Senkronize et")) { state.refresh(force: true) }.buttonStyle(PxButtonStyle())
        }

        VStack(alignment: .leading, spacing: 8) {
            Text(L.t("Kaynak başına sınırlar")).font(t.font(13, .semibold))
            Text(L.t("Claude limit yüzdesi: Anthropic'in kullanım bilgisi sınırlı olduğu için seçtiğin süre ne olursa olsun en sık 1 dakikada bir sorgulanır; hız sınırına takılırsa 5 dakika beklenir. Arada son alınan değer gösterilir, yenilenen pencereler hemen %100 olur."))
            Text(L.t("Gemini kotası: en sık 2 dakikada bir."))
            Text(L.t("Antigravity limiti: IDE açıksa her yenilemede yerel sunucudan okunur, kapalıysa okunamaz."))
            Text(L.t("Codex ve token geçmişi: yerel dosyalardan, her yenilemede."))
        }
        .font(t.font(12)).foregroundStyle(t.sub)
        .fixedSize(horizontal: false, vertical: true)
        .frame(maxWidth: .infinity, alignment: .leading)
        .card(t, padding: 16)
    }

    static func stamp(_ d: Date) -> String {
        let f = DateFormatter(); f.dateStyle = .none; f.timeStyle = .medium
        return f.string(from: d)
    }
}

/// Tonti'nin konuşma balonu ayarları. Balonlarda limit yazmaz; o an ne yaptığını ve çalışma notlarını söyler.
struct CatBubbleSettings: View {
    @EnvironmentObject var state: AppState
    @AppStorage("catBubbles") private var on = true
    @AppStorage("catBubbleWork") private var work = true
    @AppStorage("catBubbleBreak") private var rest = true
    @AppStorage("catBubbleDaily") private var daily = true
    @AppStorage("catBubbleRecord") private var record = true
    @AppStorage("catBubbleChat") private var chat = true

    var body: some View {
        let t = state.theme
        SettingRow(title: L.t("Tonti'nin balonları"), subtitle: L.t("Tonti başının üstündeki balonda ne yaptığını söyler. Sohbet içeriği okunmaz; yalnızca proje adı, süre ve token sayısı kullanılır.")) {
            Toggle("", isOn: $on).toggleStyle(PxSwitch()).labelsHidden()
        }
        if on {
            VStack(alignment: .leading, spacing: 10) {
                ThemedCheck(t: t, title: L.t("Çalışırken not alır (proje ve süre)"), isOn: $work)
                ThemedCheck(t: t, title: L.t("Uzun oturumda mola hatırlatır"), isOn: $rest)
                ThemedCheck(t: t, title: L.t("Günün ilk açılışında dünün özeti"), isOn: $daily)
                ThemedCheck(t: t, title: L.t("Günün token eşiklerini kutlar"), isOn: $record)
                ThemedCheck(t: t, title: L.t("Sevilince mırlar"), isOn: $chat)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .card(t, padding: 14)
        }
    }
}
