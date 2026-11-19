import SwiftUI
import AppKit

// MARK: - Görsel dil: kâğıt zemin, mürekkep çerçeve, piksel basamaklı köşeler (kedinin balonu gibi)

enum TC {
    /// Ayarlardaki Açık/Koyu seçimi.
    static var dark: Bool { UserDefaults.standard.bool(forKey: "tontiDark") }
    private static func c(_ l: (Double, Double, Double), _ d: (Double, Double, Double)) -> Color {
        let v = dark ? d : l
        return Color(red: v.0, green: v.1, blue: v.2)
    }
    static var paper: Color { c((0.984, 0.973, 0.953), (0.071, 0.071, 0.078)) }
    static var side: Color { c((0.953, 0.933, 0.906), (0.098, 0.098, 0.106)) }
    static var card: Color { c((1, 1, 1), (0.135, 0.135, 0.145)) }
    static var ink: Color { c((0.086, 0.047, 0.047), (0.94, 0.94, 0.93)) }
    static var sub: Color { c((0.54, 0.50, 0.49), (0.62, 0.62, 0.65)) }
    static var line: Color { c((0.89, 0.86, 0.83), (0.25, 0.25, 0.27)) }
    static var shadow: Color { dark ? Color.black.opacity(0.45) : Color(red: 0.086, green: 0.047, blue: 0.047).opacity(0.14) }
    static let pink = Color(red: 0.91, green: 0.56, blue: 0.64)
    static var pinkBg: Color { c((0.98, 0.91, 0.93), (0.33, 0.18, 0.23)) }
    static let amber = Color(red: 0.81, green: 0.55, blue: 0.16)
    static var amberBg: Color { c((0.98, 0.94, 0.86), (0.32, 0.25, 0.14)) }
    static func mono(_ size: CGFloat, _ w: Font.Weight = .regular) -> Font { .system(size: size, weight: w, design: .monospaced) }
    /// Isı haritası tonları (boştan yoğuna).
    static var heat: [Color] { [
        c((0.93, 0.91, 0.88), (0.19, 0.19, 0.21)), Color(red: 0.96, green: 0.80, blue: 0.84),
        Color(red: 0.94, green: 0.65, blue: 0.71), Color(red: 0.89, green: 0.48, blue: 0.58), Color(red: 0.72, green: 0.31, blue: 0.41),
    ] }
}

/// Köşeleri iki basamaklı kesik dikdörtgen.
struct PixelRect: Shape {
    var step: CGFloat = 2
    func path(in r: CGRect) -> Path {
        let s = step
        var p = Path()
        p.move(to: CGPoint(x: r.minX + 2 * s, y: r.minY))
        p.addLine(to: CGPoint(x: r.maxX - 2 * s, y: r.minY)); p.addLine(to: CGPoint(x: r.maxX - 2 * s, y: r.minY + s))
        p.addLine(to: CGPoint(x: r.maxX - s, y: r.minY + s)); p.addLine(to: CGPoint(x: r.maxX - s, y: r.minY + 2 * s))
        p.addLine(to: CGPoint(x: r.maxX, y: r.minY + 2 * s)); p.addLine(to: CGPoint(x: r.maxX, y: r.maxY - 2 * s))
        p.addLine(to: CGPoint(x: r.maxX - s, y: r.maxY - 2 * s)); p.addLine(to: CGPoint(x: r.maxX - s, y: r.maxY - s))
        p.addLine(to: CGPoint(x: r.maxX - 2 * s, y: r.maxY - s)); p.addLine(to: CGPoint(x: r.maxX - 2 * s, y: r.maxY))
        p.addLine(to: CGPoint(x: r.minX + 2 * s, y: r.maxY)); p.addLine(to: CGPoint(x: r.minX + 2 * s, y: r.maxY - s))
        p.addLine(to: CGPoint(x: r.minX + s, y: r.maxY - s)); p.addLine(to: CGPoint(x: r.minX + s, y: r.maxY - 2 * s))
        p.addLine(to: CGPoint(x: r.minX, y: r.maxY - 2 * s)); p.addLine(to: CGPoint(x: r.minX, y: r.minY + 2 * s))
        p.addLine(to: CGPoint(x: r.minX + s, y: r.minY + 2 * s)); p.addLine(to: CGPoint(x: r.minX + s, y: r.minY + s))
        p.addLine(to: CGPoint(x: r.minX + 2 * s, y: r.minY + s)); p.closeSubpath()
        return p
    }
}

extension View {
    /// Piksel çerçeveli kart; `lift` ile sağ alta düz gölge.
    func pxCard(_ fill: Color? = nil, padding: CGFloat = 12, lift: Bool = true) -> some View {
        self.padding(padding)
            .background(PixelRect().fill(fill ?? TC.card))
            .overlay(PixelRect().strokeBorder(TC.ink, lineWidth: 2))
            .background(PixelRect().fill(lift ? TC.shadow : .clear).offset(x: 3, y: 3))
    }
}

extension PixelRect: InsettableShape {
    func inset(by amount: CGFloat) -> some InsettableShape { InsetPixel(base: self, inset: amount) }
}
struct InsetPixel: InsettableShape {
    var base: PixelRect; var inset: CGFloat
    func path(in r: CGRect) -> Path { base.path(in: r.insetBy(dx: inset, dy: inset)) }
    func inset(by amount: CGFloat) -> InsetPixel { InsetPixel(base: base, inset: inset + amount) }
}

struct PxButton: View {
    let title: String
    var filled = false
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            Text(title).font(TC.mono(11, .bold)).foregroundStyle(filled ? TC.paper : TC.ink)
                .padding(.horizontal, 10).padding(.vertical, 5)
                .background(PixelRect().fill(filled ? TC.ink : TC.card))
                .overlay(PixelRect().strokeBorder(TC.ink, lineWidth: 2))
                .contentShape(Rectangle())
        }.buttonStyle(.plain)
    }
}

/// Varsayılan düğmeler (Senkronize et, Güncellemeleri denetle…) piksel temada çerçeveli görünsün.
struct PxButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var enabled
    func makeBody(configuration c: Configuration) -> some View {
        c.label
            .font(TC.mono(11, .bold))
            .foregroundStyle(c.isPressed ? TC.paper : TC.ink)
            .padding(.horizontal, 10).padding(.vertical, 5)
            .background(PixelRect().fill(c.isPressed ? TC.ink : TC.card))
            .overlay(PixelRect().strokeBorder(TC.ink, lineWidth: 2))
            .opacity(enabled ? 1 : 0.45)
            .contentShape(Rectangle())
    }
}

/// Seçenek listesi: piksel çerçeveli düğme, tıklanınca menü açılır.
struct PxMenu<V: Hashable>: View {
    let items: [(label: String, value: V)]
    @Binding var selection: V
    var body: some View {
        Menu {
            ForEach(Array(items.enumerated()), id: \.offset) { _, it in
                Button(it.label) { selection = it.value }
            }
        } label: {
            Text((items.first { $0.value == selection }?.label ?? "") + "  ▾")
        }
        .menuStyle(.button)
        .buttonStyle(PxButtonStyle())
        .menuIndicator(.hidden)
        .fixedSize()
    }
}

/// Piksel anahtar: çentikli çerçeve, açıkken dolu ve kare topuz sağda.
struct PxSwitch: ToggleStyle {
    func makeBody(configuration c: Configuration) -> some View {
        Button { withAnimation(.easeOut(duration: 0.12)) { c.isOn.toggle() } } label: {
            ZStack(alignment: c.isOn ? .trailing : .leading) {
                PixelRect().fill(c.isOn ? TC.pink : TC.card)
                Rectangle().fill(c.isOn ? TC.paper : TC.ink).frame(width: 12, height: 12).padding(.horizontal, 5)
            }
            .frame(width: 42, height: 22)
            .overlay(PixelRect().strokeBorder(TC.ink, lineWidth: 2))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityValue(c.isOn ? L.t("Etkin") : L.t("Kapalı"))
    }
}

/// Satıra sığmayan öğeleri alt satıra geçirir (ayar sekmeleri gibi).
struct FlowLayout: Layout {
    var spacing: CGFloat = 8
    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let w = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, row: CGFloat = 0, maxX: CGFloat = 0
        for v in subviews {
            let s = v.sizeThatFits(.unspecified)
            if x > 0 && x + s.width > w { x = 0; y += row + spacing; row = 0 }
            x += s.width + spacing; row = max(row, s.height); maxX = max(maxX, x - spacing)
        }
        return CGSize(width: proposal.width ?? maxX, height: y + row)
    }
    func placeSubviews(in b: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = b.minX, y = b.minY, row: CGFloat = 0
        for v in subviews {
            let s = v.sizeThatFits(.unspecified)
            if x > b.minX && x + s.width > b.maxX { x = b.minX; y += row + spacing; row = 0 }
            v.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(s))
            x += s.width + spacing; row = max(row, s.height)
        }
    }
}

/// Uygulama paketindeki kedi karesi (piksel keskin).
struct CatSprite: View {
    let name: String
    var height: CGFloat
    var body: some View {
        if let img = CatSprites.shared.image(name) {
            Image(decorative: img, scale: 1).resizable().interpolation(.none).aspectRatio(contentMode: .fit).frame(height: height)
        }
    }
}

// MARK: - Pencere

enum TontiPage: String, CaseIterable, Identifiable {
    case diary, reports, usage, settings
    var id: String { rawValue }
    var title: String {
        switch self { case .diary: "Günlük"; case .reports: "Raporlar"; case .usage: "Kullanım"; case .settings: "Ayarlar" }
    }
    var glyph: String {
        switch self { case .diary: "✎"; case .reports: "▤"; case .usage: "◔"; case .settings: "⚙" }
    }
    /// Eski sayfa adları (kedi, menü, bildirimler) buraya yönlenir.
    static func from(_ s: String) -> TontiPage {
        switch s {
        case "diary", "history": .diary
        case "reports": .reports
        case "usage": .usage
        default: .settings
        }
    }
}

struct TontiRoot: View {
    @EnvironmentObject var state: AppState

    var body: some View {
        let page = TontiPage.from(state.dashPage)
        HStack(spacing: 0) {
            sidebar(page)
            Rectangle().fill(TC.ink).frame(width: 2)
            Group {
                switch page {
                case .diary: DiaryPage()
                case .reports: ReportsPage().id(state.reportsToken)
                case .usage: LegacyScroll { HistoryPage() }
                case .settings: SettingsPage()
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            .padding(.top, 14)
            .background(TC.paper)
        }
        .ignoresSafeArea()
        .background(TC.paper)
        .foregroundStyle(TC.ink)
        .themed(state.theme)
        .id(state.tontiDark)   // renkler yeniden hesaplansın
        .environment(\.locale, L.locale)
    }

    private func sidebar(_ page: TontiPage) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 10) {
                CatSprite(name: CatAnim.has("content") ? "content" : "idle_0", height: 44)
                VStack(alignment: .leading, spacing: 1) {
                    Text(AppInfo.name.uppercased()).font(TC.mono(15, .bold))
                    Text(L.t("senin kedin, senin günlüğün")).font(TC.mono(9.5)).foregroundStyle(TC.sub)
                }
            }
            .padding(.bottom, 18)
            VStack(alignment: .leading, spacing: 4) {
            ForEach(TontiPage.allCases) { p in
                let on = p == page
                HStack(spacing: 8) {
                    Text(p.glyph).frame(width: 16)
                    Text(L.t(p.title).uppercased(with: L.locale)).font(TC.mono(12, .bold)).kerning(0.6)
                    Spacer()
                }
                .foregroundStyle(on ? TC.ink : TC.sub)
                .padding(.horizontal, 10).padding(.vertical, 8)
                .background(PixelRect().fill(on ? TC.card : .clear))
                .overlay(PixelRect().strokeBorder(on ? TC.ink : .clear, lineWidth: 2))
                .contentShape(Rectangle())
                .onTapGesture { state.dashPage = p.rawValue }
            }
            }
            .tourAnchor("sidebar.nav")
            Spacer()
            LimitMini()
        }
        .padding(16).padding(.top, 24)   // pencere düğmeleri için yer
        .frame(width: 214)
        .background(TC.side)
    }
}

/// Eski (Ullage'den gelen) sayfaları kaydırmalı alanda gösterir.
struct LegacyScroll<C: View>: View {
    @ViewBuilder var content: C
    var body: some View {
        ScrollView(showsIndicators: false) { VStack(alignment: .leading, spacing: 16) { content }.padding(24).frame(maxWidth: .infinity, alignment: .leading) }
    }
}

struct ComingSoon: View {
    let title: String
    var body: some View {
        VStack(spacing: 12) {
            CatSprite(name: "sleep", height: 70)
            Text(L.t(title)).font(TC.mono(18, .bold))
            Text(L.t("Tonti bu sayfayı hazırlıyor.")).font(TC.mono(12)).foregroundStyle(TC.sub)
        }.frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

/// Kenar çubuğunun altındaki küçük limit kutusu.
struct LimitMini: View {
    @EnvironmentObject var state: AppState
    var body: some View {
        let rows: [(String, Usage)] = [(Provider.claude, "CLAUDE"), (Provider.codex, "CODEX")].compactMap { p, n in
            guard state.enabled.contains(p), let u = state.usage[p], u.limitConfidence != .none, u.fiveRow != nil || u.weekRow != nil else { return nil }
            return (n, u)
        }
        if !rows.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                ForEach(rows, id: \.0) { name, u in
                    VStack(alignment: .leading, spacing: 4) {
                        Text(name).font(TC.mono(9.5, .semibold)).foregroundStyle(TC.sub).kerning(0.8)
                        if let r = u.fiveRow { bar(L.t("5 saatlik"), r.left) }
                        if let r = u.weekRow { bar(L.t("Haftalık"), r.left) }
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .pxCard(padding: 10, lift: false)
            .onTapGesture { state.dashPage = "usage" }
        }
    }
    private func bar(_ title: String, _ left: Double) -> some View {
        VStack(spacing: 2) {
            HStack { Text(title).font(TC.mono(10.5)); Spacer(); Text("\(Int((left * 100).rounded()))%").font(TC.mono(10.5, .bold)) }
            GeometryReader { g in
                ZStack(alignment: .leading) {
                    Rectangle().fill(TC.card)
                    Rectangle().fill(left < 0.15 ? TC.pink : TC.ink).frame(width: max(0, g.size.width - 4) * left).padding(2)
                }.overlay(Rectangle().strokeBorder(TC.ink, lineWidth: 2))
            }.frame(height: 9)
        }
    }
}

// MARK: - Ayarlar: eski sayfalar sekmelerle

struct SettingsPage: View {
    @EnvironmentObject var state: AppState
    private let tabs: [(String, String)] = [("appearance", "Tonti"), ("providers", "Sağlayıcılar"), ("menubar", "Menü çubuğu"),
                                            ("automation", "Bildirim ve otomasyon"), ("computer", "Bilgisayar günlüğü"), ("sync", "Senkronizasyon"), ("github", "GitHub kartı"), ("about", "Hakkında")]
    var body: some View {
        let cur = tabs.contains { $0.0 == state.dashPage } ? state.dashPage : "appearance"
        VStack(alignment: .leading, spacing: 0) {
            Text(L.t("Ayarlar")).font(TC.mono(20, .bold)).padding([.top, .horizontal], 24)
            // Dar pencerede sekmeler kesilmesin: sığmayan alt satıra geçer.
            FlowLayout(spacing: 8) {
                ForEach(tabs, id: \.0) { id, title in
                    PxButton(title: L.t(title), filled: id == cur) { state.dashPage = id }
                }
            }.tourAnchor("settings.tabs").padding(.horizontal, 24).padding(.vertical, 12)
            LegacyScroll {
                switch cur {
                case "providers": ProvidersPage()
                case "menubar": MenuBarPage()
                case "automation": AutomationPage()
                case "sync": SyncPage()
                case "about": AboutPage()
                case "github": GitHubProfileSettings()
                case "computer": ComputerLogSettings()
                default: AppearancePage()
                }
            }
        }
    }
}

// MARK: - Günlük

@MainActor
final class DiaryModel: ObservableObject {
    struct Entry: Identifiable { let session: WorkSession; let commits: [GitLog.Commit]; var id: String { session.id } }
    /// ULLAGECAT_DIARY=-1 gibi bir değerle (geliştirme) Günlük o günle açılır.
    @Published var day = Calendar.current.date(byAdding: .day, value: Int(ProcessInfo.processInfo.environment["ULLAGECAT_DIARY"] ?? "0") ?? 0,
                                               to: Calendar.current.startOfDay(for: Date()))!
    @Published var entries: [Entry] = []
    @Published var heat: [(day: Date, tokens: Int)] = []
    @Published var weekProjects: [(name: String, time: TimeInterval)] = []
    @Published var loading = false
    @Published var computer = ComputerDay()
    /// Oturumu olmayan depolara o gün atılan commit'ler (ör. oturum başka klasörde açılmışsa).
    @Published var otherCommits: [(name: String, commits: [GitLog.Commit])] = []
    /// "ai" (yapay zekâ oturumları) ya da "pc" (bilgisayar). MEOWLY_DIARY_MODE ile geliştirmede seçilir.
    @Published var mode = ProcessInfo.processInfo.environment["MEOWLY_DIARY_MODE"] ?? UserDefaults.standard.string(forKey: "diaryMode") ?? "ai" {
        didSet { UserDefaults.standard.set(mode, forKey: "diaryMode") }
    }
    private var gen = 0

    func load() {
        gen += 1
        let g = gen, d = day
        loading = true
        DispatchQueue.global(qos: .userInitiated).async {
            let store = HistoryStore.shared
            let ss = store.sessions(from: d, to: d.addingTimeInterval(86400)).filter { $0.duration >= 60 || $0.tokens > 0 }
            var byPath: [String: [GitLog.Commit]] = [:]
            for p in Set(ss.compactMap(\.path)) { byPath[p] = GitLog.commits(path: p, day: d) }
            // Hiç token'ı ve commit'i olmayan (ör. yalnızca açılıp kapanan) oturumlar gösterilmez.
            let entries = ss.reversed().map { s in Entry(session: s, commits: s.path.map { GitLog.commits(for: s, in: byPath[$0] ?? []) } ?? []) }
                .filter { $0.session.tokens > 0 || !$0.commits.isEmpty }
            let heat = store.dailyTokens(days: 7 * 15)
            let weekStart = Calendar.current.date(byAdding: .day, value: -6, to: Calendar.current.startOfDay(for: Date()))!
            var wp: [String: TimeInterval] = [:]
            for s in store.sessions(from: weekStart) where !s.project.isEmpty { wp[s.project, default: 0] += s.duration }
            let top = wp.sorted { $0.value > $1.value }.prefix(5).map { (name: $0.key, time: $0.value) }
            let pc = ComputerDay.build(from: d, to: d.addingTimeInterval(86400))
            let others = GitLog.otherCommits(from: d, to: d.addingTimeInterval(86400), excluding: Set(byPath.keys)).map { (name: $0.name, commits: $0.commits) }
            DispatchQueue.main.async {
                guard g == self.gen else { return }
                self.entries = Array(entries); self.heat = heat; self.weekProjects = top; self.computer = pc; self.otherCommits = others; self.loading = false
            }
        }
    }

    func shift(_ days: Int) {
        guard let d = Calendar.current.date(byAdding: .day, value: days, to: day), d <= Date() else { return }
        day = d; load()
    }
}

struct DiaryPage: View {
    @StateObject private var m = DiaryModel()
    private static let dayFmt: DateFormatter = { let f = DateFormatter(); f.locale = L.locale; f.dateFormat = "d MMMM yyyy, EEEE"; return f }()

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            header.padding(.horizontal, 24).padding(.top, 22).padding(.bottom, 10)
            PermissionNudge().padding(.horizontal, 24).padding(.bottom, 10)
            HStack(spacing: 8) {
                PxButton(title: L.t("YAPAY ZEKÂ"), filled: m.mode == "ai") { m.mode = "ai" }
                PxButton(title: L.t("BİLGİSAYAR"), filled: m.mode == "pc") { m.mode = "pc" }
            }.padding(.horizontal, 24).padding(.bottom, 12)
            HStack(alignment: .top, spacing: 20) {
                if m.mode == "pc" {
                    ScrollView(showsIndicators: false) { ComputerDayView(data: m.computer).padding(.bottom, 24).padding(.trailing, 4) }
                    ScrollView(showsIndicators: false) { ComputerSide(data: m.computer) }.frame(width: 250)
                } else {
                    ScrollView(showsIndicators: false) { timeline.padding(.bottom, 24).padding(.trailing, 4) }
                    ScrollView(showsIndicators: false) { side }.frame(width: 250)   // uzun olursa pencereyi taşırmasın
                }
            }
            .padding(.horizontal, 24)
        }
        .onAppear { m.load() }
    }

    private var header: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 3) {
                Text(L.t("Günlük")).font(TC.mono(20, .bold))
                Text(L.t("Tonti yapay zekâ oturumlarını, commit'lerini ve bilgisayarda ne yaptığını buraya yazar. Sohbetlerin ve ekranın okunmaz."))
                    .font(TC.mono(11)).foregroundStyle(TC.sub)
            }
            Spacer()
            PxButton(title: "‹") { m.shift(-1) }
            Text(Self.dayFmt.string(from: m.day)).font(TC.mono(12, .bold)).frame(minWidth: 190)
            PxButton(title: "›") { m.shift(1) }
            PxButton(title: L.t("BUGÜN"), filled: Calendar.current.isDateInToday(m.day)) { m.day = Calendar.current.startOfDay(for: Date()); m.load() }
        }
    }

    // Zaman çizelgesi: solda saat, sağda oturum kartı.
    @ViewBuilder private var timeline: some View {
        if m.entries.isEmpty && m.otherCommits.isEmpty && !m.loading {
            VStack(spacing: 10) {
                CatSprite(name: "sleep", height: 64)
                Text(L.t("Bu gün için not yok.")).font(TC.mono(12, .semibold))
                Text(L.t("Claude ya da Codex ile çalıştığında Tonti buraya yazar.")).font(TC.mono(11)).foregroundStyle(TC.sub)
            }.frame(maxWidth: .infinity).padding(.top, 60)
        } else {
            VStack(alignment: .leading, spacing: 14) {
                ForEach(m.entries) { e in EntryRow(entry: e) }
                if !m.otherCommits.isEmpty { otherCard }
            }
        }
    }

    /// Oturumla eşleşmeyen depolardaki commit'ler.
    private var otherCard: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L.t("BAŞKA DEPOLARDA")).font(TC.mono(10, .bold)).foregroundStyle(TC.sub).kerning(0.8)
            ForEach(m.otherCommits, id: \.name) { o in
                VStack(alignment: .leading, spacing: 4) {
                    HStack { Text(o.name).font(TC.mono(12.5, .bold)); Spacer(); Text(L.f("%lld commit", o.commits.count)).font(TC.mono(10.5)).foregroundStyle(TC.sub) }
                    ForEach(Array(o.commits.reversed().prefix(6).enumerated()), id: \.offset) { _, c in
                        HStack(alignment: .firstTextBaseline, spacing: 6) {
                            Text(Self.clock.string(from: c.date)).font(TC.mono(10)).foregroundStyle(TC.sub)
                            Text(c.subject).font(TC.mono(11)).lineLimit(2)
                        }
                    }
                    if o.commits.count > 6 { Text(L.f("+%lld commit daha", o.commits.count - 6)).font(TC.mono(10.5)).foregroundStyle(TC.sub) }
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading).pxCard()
        .padding(.leading, 84)   // oturum kartlarıyla aynı hizada
    }

    private static let clock: DateFormatter = { let f = DateFormatter(); f.dateFormat = "HH:mm"; return f }()

    private var side: some View {
        VStack(alignment: .leading, spacing: 16) {
            let ss = m.entries.map(\.session)
            VStack(alignment: .leading, spacing: 10) {
                Text(L.t("O GÜN")).font(TC.mono(10, .bold)).foregroundStyle(TC.sub).kerning(0.8)
                LazyVGrid(columns: [GridItem(.flexible(), alignment: .leading), GridItem(.flexible(), alignment: .leading)], spacing: 10) {
                    stat(CatController.duration(ss.reduce(0) { $0 + $1.duration }), L.t("çalışma"))
                    stat("\(ss.count)", L.t("oturum"))
                    stat("\(m.entries.reduce(0) { $0 + $1.commits.count } + m.otherCommits.reduce(0) { $0 + $1.commits.count })", L.t("commit"))
                    stat(formatTokens(ss.reduce(0) { $0 + $1.tokens }), L.t("token"))
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading).pxCard()

            VStack(alignment: .leading, spacing: 8) {
                Text(L.t("SON 15 HAFTA")).font(TC.mono(10, .bold)).foregroundStyle(TC.sub).kerning(0.8)
                HeatGrid(days: m.heat, selected: m.day) { d in m.day = d; m.load() }
            }
            .frame(maxWidth: .infinity, alignment: .leading).pxCard()

            if !m.weekProjects.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    Text(L.t("BU HAFTA")).font(TC.mono(10, .bold)).foregroundStyle(TC.sub).kerning(0.8)
                    let mx = m.weekProjects.first?.time ?? 1
                    ForEach(m.weekProjects, id: \.name) { p in
                        VStack(alignment: .leading, spacing: 3) {
                            HStack { Text(p.name).font(TC.mono(11, .semibold)).lineLimit(1); Spacer(); Text(CatController.duration(p.time)).font(TC.mono(10.5)).foregroundStyle(TC.sub) }
                            GeometryReader { g in Rectangle().fill(TC.pink).frame(width: g.size.width * p.time / mx) }.frame(height: 5)
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading).pxCard()
            }
            Spacer()
        }
    }

    private func stat(_ v: String, _ k: String) -> some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(v).font(TC.mono(15, .bold)).lineLimit(1).minimumScaleFactor(0.7)
            Text(k.uppercased(with: L.locale)).font(TC.mono(9)).foregroundStyle(TC.sub).kerning(0.6)
        }
    }
}

struct EntryRow: View {
    let entry: DiaryModel.Entry
    private static let hm: DateFormatter = { let f = DateFormatter(); f.dateFormat = "HH:mm"; return f }()

    var body: some View {
        let s = entry.session
        HStack(alignment: .top, spacing: 14) {
            VStack(alignment: .trailing, spacing: 2) {
                Text(Self.hm.string(from: s.start)).font(TC.mono(15, .bold))
                Text(CatController.duration(s.duration)).font(TC.mono(10)).foregroundStyle(TC.sub)
            }.frame(width: 76, alignment: .trailing)
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 6) {
                    Text(s.project.isEmpty ? L.t("Projesiz") : s.project).font(TC.mono(13, .bold))
                    Spacer()
                    ForEach(s.providers.sorted(), id: \.self) { p in tag(p) }
                }
                if entry.commits.isEmpty {
                    Text("\(Self.hm.string(from: s.start))–\(Self.hm.string(from: s.end))").font(TC.mono(11)).foregroundStyle(TC.sub)
                } else {
                    VStack(alignment: .leading, spacing: 3) {
                        ForEach(entry.commits, id: \.self) { c in
                            HStack(alignment: .firstTextBaseline, spacing: 6) {
                                Text("✓").font(TC.mono(11, .bold)).foregroundStyle(TC.pink)
                                Text(c.subject).font(TC.mono(11.5)).fixedSize(horizontal: false, vertical: true)
                            }
                        }
                    }
                }
                Text(formatTokens(s.tokens) + " token").font(TC.mono(10.5)).foregroundStyle(TC.sub)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .pxCard()
        }
    }

    private func tag(_ p: String) -> some View {
        let claude = p == "claude"
        return Text(p.uppercased()).font(TC.mono(9.5, .bold))
            .padding(.horizontal, 6).padding(.vertical, 2)
            .background(Rectangle().fill(claude ? TC.pinkBg : TC.amberBg))
            .overlay(Rectangle().strokeBorder(TC.ink, lineWidth: 1.5))
    }
}

/// Haftalar sütun, günler satır. Tıklanan gün Günlük'te açılır.
struct HeatGrid: View {
    let days: [(day: Date, tokens: Int)]
    let selected: Date
    let pick: (Date) -> Void
    var body: some View {
        let mx = max(1, days.map(\.tokens).max() ?? 1)
        let cal = Calendar.current
        // İlk sütun pazartesiden başlasın diye baştaki boşluk.
        let lead = days.first.map { (cal.component(.weekday, from: $0.day) + 5) % 7 } ?? 0
        let cells: [(Date, Int)?] = Array(repeating: nil, count: lead) + days.map { Optional(($0.day, $0.tokens)) }
        let cols = (cells.count + 6) / 7
        HStack(alignment: .top, spacing: 3) {
            ForEach(0..<cols, id: \.self) { c in
                VStack(spacing: 3) {
                    ForEach(0..<7, id: \.self) { r in
                        let i = c * 7 + r
                        if i < cells.count, let cell = cells[i] {
                            let lvl = cell.1 == 0 ? 0 : min(4, 1 + Int(Double(cell.1) / Double(mx) * 3.999))
                            Rectangle().fill(TC.heat[lvl]).frame(width: 11, height: 11)
                                .overlay(Rectangle().strokeBorder(TC.ink, lineWidth: cal.isDate(cell.0, inSameDayAs: selected) ? 1.5 : 0))
                                .onTapGesture { pick(cell.0) }
                                .help(formatTokens(cell.1) + " token")
                        } else {
                            Color.clear.frame(width: 11, height: 11)
                        }
                    }
                }
            }
        }
    }
}
