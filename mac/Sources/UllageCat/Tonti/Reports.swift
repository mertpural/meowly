import SwiftUI
import AppKit
import UniformTypeIdentifiers

// MARK: - Dönem

enum ReportKind: String, CaseIterable, Identifiable {
    case day, week, month, year
    var id: String { rawValue }
    var tab: String { switch self { case .day: "Günlük"; case .week: "Haftalık"; case .month: "Aylık"; case .year: "Yıllık" } }
    var title: String { switch self { case .day: "Günlük rapor"; case .week: "Haftalık rapor"; case .month: "Aylık rapor"; case .year: "Yıllık rapor" } }
    /// Kartta proje başına gösterilen en fazla commit.
    var perProject: Int { switch self { case .day: 8; case .week: 5; case .month: 4; case .year: 3 } }
}

/// Kediden "dünün raporu" gibi bir istekle açılış.
enum ReportNav { static var pending: (ReportKind, Date)? }

struct ReportPeriod {
    let kind: ReportKind
    let start: Date
    let end: Date

    static func containing(_ d: Date, _ k: ReportKind) -> ReportPeriod {
        var cal = Calendar.current
        cal.firstWeekday = 2   // pazartesi
        let comp: Calendar.Component = [ReportKind.day: .day, .week: .weekOfYear, .month: .month, .year: .year][k]!
        let iv = cal.dateInterval(of: comp, for: d) ?? DateInterval(start: cal.startOfDay(for: d), duration: 86400)
        return ReportPeriod(kind: k, start: iv.start, end: iv.end)
    }

    func shifted(_ n: Int) -> ReportPeriod {
        let comp: Calendar.Component = [ReportKind.day: .day, .week: .weekOfYear, .month: .month, .year: .year][kind]!
        let d = Calendar.current.date(byAdding: comp, value: n, to: start) ?? start
        return .containing(d, kind)
    }

    var isCurrentOrFuture: Bool { end > Date() }

    var label: String {
        let f = DateFormatter(); f.locale = L.locale
        switch kind {
        case .day: f.dateFormat = "d MMMM yyyy, EEEE"; return f.string(from: start)
        case .week:
            f.dateFormat = "d MMM"
            let last = end.addingTimeInterval(-1)
            let y = DateFormatter(); y.locale = L.locale; y.dateFormat = "yyyy"
            return "\(f.string(from: start)) – \(f.string(from: last)) \(y.string(from: last))"
        case .month: f.dateFormat = "LLLL yyyy"; return f.string(from: start).capitalized(with: L.locale)
        case .year: f.dateFormat = "yyyy"; return f.string(from: start)
        }
    }
}

// MARK: - Veri

struct ReportData {
    struct Project: Identifiable { let name: String; let time: TimeInterval; let tokens: Int; let commits: [GitLog.Commit]; var id: String { name } }
    var period: ReportPeriod
    var projects: [Project] = []
    var active: TimeInterval = 0
    var sessions = 0
    var tokens = 0
    var commits: Int { projects.reduce(0) { $0 + $1.commits.count } }
    var busiest: (Date, TimeInterval)?
    /// Grafik için dilimler (hafta/ay: gün, yıl: ay) ve etiketleri.
    var buckets: [(label: String, time: TimeInterval)] = []
    /// Bilgisayar günlüğü özeti. Paylaşılan çıktıya yalnızca uygulama adları, kategoriler ve toplantı toplamı girer.
    var computer = ComputerDay()
    var meetingTime: TimeInterval { computer.meetings.reduce(0) { $0 + $1.end.timeIntervalSince($1.start) } }

    static func build(_ p: ReportPeriod) -> ReportData {
        var d = ReportData(period: p)
        let ss = HistoryStore.shared.sessions(from: p.start, to: p.end)
        d.sessions = ss.count
        d.active = ss.reduce(0) { $0 + $1.duration }
        d.tokens = ss.reduce(0) { $0 + $1.tokens }
        var byName: [String: (time: TimeInterval, tokens: Int, path: String?)] = [:]
        for s in ss where !s.project.isEmpty {
            var v = byName[s.project] ?? (0, 0, nil)
            v.time += s.duration; v.tokens += s.tokens; v.path = v.path ?? s.path
            byName[s.project] = v
        }
        var projects = byName.map { name, v in
            Project(name: name, time: v.time, tokens: v.tokens,
                    commits: v.path.map { GitLog.commits(path: $0, from: p.start, to: p.end) } ?? [])
        }
        // Oturumu olmayan ama commit atılmış depolar da çıkanlara girer (süresiz).
        let known = Set(byName.values.compactMap(\.path))
        for o in GitLog.otherCommits(from: p.start, to: p.end, excluding: known) {
            if let i = projects.firstIndex(where: { $0.name == o.name }) {
                if projects[i].commits.isEmpty { projects[i] = Project(name: o.name, time: projects[i].time, tokens: projects[i].tokens, commits: o.commits) }
            } else {
                projects.append(Project(name: o.name, time: 0, tokens: 0, commits: o.commits))
            }
        }
        d.projects = projects.sorted { ($0.commits.count, $0.time) > ($1.commits.count, $1.time) }

        d.computer = ComputerDay.summary(from: p.start, to: min(p.end, Date()))
        let cal = Calendar.current
        var perDay: [Date: TimeInterval] = [:]
        for s in ss { perDay[cal.startOfDay(for: s.start), default: 0] += s.duration }
        d.busiest = perDay.max { $0.value < $1.value }.map { ($0.key, $0.value) }
        let f = DateFormatter(); f.locale = L.locale
        switch p.kind {
        case .day: break
        case .week, .month:
            f.dateFormat = p.kind == .week ? "EEEEE" : "d"
            var t = p.start
            while t < p.end { d.buckets.append((f.string(from: t), perDay[t] ?? 0)); t = cal.date(byAdding: .day, value: 1, to: t)! }
        case .year:
            f.dateFormat = "MMMMM"
            var t = p.start
            while t < p.end {
                let n = cal.date(byAdding: .month, value: 1, to: t)!
                d.buckets.append((f.string(from: t), perDay.filter { $0.key >= t && $0.key < n }.reduce(0) { $0 + $1.value }))
                t = n
            }
        }
        return d
    }

    var markdown: String {
        var out = "## \(L.t(period.kind.title)) · \(period.label)\n\n"
        for pr in projects {
            out += "**\(pr.name)** · " + (pr.time > 0 ? CatController.duration(pr.time) : L.f("%lld commit", pr.commits.count)) + "\n"
            for c in pr.commits.reversed() { out += "- \(c.subject)\n" }
            out += "\n"
        }
        if computer.active > 0 {
            out += "**\(L.t("Bilgisayarda")) · \(CatController.duration(computer.active))**\n"
            for a in computer.apps.prefix(5) { out += "- \(a.name): \(CatController.duration(a.time))\n" }
            if meetingTime > 0 { out += "- \(L.t("Toplantı")): \(CatController.duration(meetingTime))\n" }
            out += "\n"
        }
        out += "\(L.t("Çalışma")): \(CatController.duration(active)) · \(L.t("Oturum")): \(sessions) · \(L.t("Commit")): \(commits) · \(L.t("Proje")): \(projects.count) · Token: \(formatTokens(tokens))\n"
        return out
    }
}

@MainActor
final class ReportsModel: ObservableObject {
    @Published var period: ReportPeriod
    @Published var data: ReportData?
    @Published var note = ""
    private var gen = 0

    init() {
        if let (k, d) = ReportNav.pending { period = .containing(d, k); ReportNav.pending = nil }
        else { period = .containing(Date(), .week) }
    }

    func load() {
        gen += 1
        let g = gen, p = period
        DispatchQueue.global(qos: .userInitiated).async {
            let d = ReportData.build(p)
            DispatchQueue.main.async { if g == self.gen { self.data = d } }
        }
    }

    /// Tür değişince bakılan an korunur: dönem bugünü içeriyorsa bugün, değilse dönemin başı.
    /// (Eskiden hep dönemin başı alınıyordu; Yıllık'tan Günlük'e geçince 1 Ocak açılıp her şey 0 görünüyordu.)
    func set(_ k: ReportKind) {
        let now = Date()
        let ref = (period.start <= now && now < period.end) ? now : min(period.start, now)
        period = .containing(ref, k); load()
    }
    func shift(_ n: Int) {
        let p = period.shifted(n)
        guard p.start <= Date() else { return }
        period = p; load()
    }
}

// MARK: - Sayfa

struct ReportsPage: View {
    @StateObject private var m = ReportsModel()

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(L.t("Raporlar")).font(TC.mono(20, .bold))
                    Text(L.t("Ne üzerinde çalıştığını gün, hafta, ay ya da yıl olarak gör; standup notu ya da paylaşım için kopyala."))
                        .font(TC.mono(11)).foregroundStyle(TC.sub)
                }
                Spacer()
            }
            .padding(.horizontal, 24).padding(.top, 22)

            HStack(spacing: 8) {
                ForEach(ReportKind.allCases) { k in PxButton(title: L.t(k.tab).uppercased(with: L.locale), filled: m.period.kind == k) { m.set(k) } }
                Spacer()
                PxButton(title: "‹") { m.shift(-1) }
                Text(m.period.label).font(TC.mono(12, .bold)).frame(minWidth: 200)
                PxButton(title: "›") { m.shift(1) }
            }
            .padding(.horizontal, 24).padding(.vertical, 14)

            ScrollView(showsIndicators: false) {
                VStack(spacing: 14) {
                    if let d = m.data {
                        ReportCard(data: d, interactive: true).id(d.period.start)
                        HStack(spacing: 8) {
                            PxButton(title: L.t("MARKDOWN KOPYALA")) { copyMarkdown(d) }
                            PxButton(title: L.t("RESMİ KOPYALA"), filled: true) { copyImage(d) }
                            PxButton(title: L.t("RESMİ KAYDET")) { saveImage(d) }
                            if !m.note.isEmpty { Text(m.note).font(TC.mono(11)).foregroundStyle(TC.sub) }
                        }
                    } else {
                        ProgressView().padding(40)
                    }
                }
                .frame(maxWidth: .infinity).padding(.bottom, 24)
            }
        }
        .onAppear { m.load() }
        .onChange(of: m.data?.period.start) { _, _ in
            // Geliştirme: ULLAGECAT_REPORT_PNG=yol ile kart resmi dosyaya yazılır.
            if let path = ProcessInfo.processInfo.environment["ULLAGECAT_REPORT_PNG"], let d = m.data,
               let img = render(d), let t = img.tiffRepresentation, let r = NSBitmapImageRep(data: t),
               let png = r.representation(using: .png, properties: [:]) { try? png.write(to: URL(fileURLWithPath: path)) }
        }
    }

    private func flash(_ s: String) {
        m.note = s
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { if m.note == s { m.note = "" } }
    }

    private func copyMarkdown(_ d: ReportData) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(d.markdown, forType: .string)
        flash(L.t("Kopyalandı"))
    }

    @MainActor private func render(_ d: ReportData) -> NSImage? {
        let r = ImageRenderer(content: ReportCard(data: d).padding(16).background(TC.paper))
        r.scale = 2
        return r.nsImage
    }

    private func copyImage(_ d: ReportData) {
        guard let img = render(d) else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.writeObjects([img])
        flash(L.t("Kopyalandı"))
    }

    private func saveImage(_ d: ReportData) {
        guard let img = render(d), let tiff = img.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff),
              let png = rep.representation(using: .png, properties: [:]) else { return }
        let p = NSSavePanel()
        p.allowedContentTypes = [.png]
        p.nameFieldStringValue = "\(AppInfo.name) \(d.period.label).png"
        if p.runModal() == .OK, let url = p.url { try? png.write(to: url); flash(L.t("Kaydedildi")) }
    }
}

// MARK: - Kart (ekranda ve paylaşılan resimde aynı)

struct ReportCard: View {
    let data: ReportData
    /// Ekranda "+N commit daha" tıklanınca o projenin tüm commit'leri açılır; paylaşılan resimde kısa hali kalır.
    var interactive = false
    @StateObject private var ex = Expanded()
    final class Expanded: ObservableObject { @Published var set: Set<String> = [] }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(L.t(data.period.kind.title)).font(TC.mono(20, .bold))
                    Text(data.period.label).font(TC.mono(11)).foregroundStyle(TC.sub)
                }
                Spacer()
                HStack(spacing: 6) {
                    Text(AppInfo.name).font(TC.mono(10.5, .bold))
                    CatSprite(name: CatAnim.has("content") ? "content" : "idle_0", height: 34)
                }
            }
            Rectangle().fill(TC.ink).frame(height: 2)
            HStack(alignment: .top, spacing: 22) {
                shipped.frame(maxWidth: .infinity, alignment: .leading)
                Rectangle().fill(TC.line).frame(width: 2)
                numbers.frame(width: 170, alignment: .leading)
            }
            if data.computer.active > 0 { computerRow }
            if !data.buckets.isEmpty { chart }
            Text("✎ " + L.t("Tonti not aldı · bilgisayarında, sohbetler okunmadan"))
                .font(TC.mono(9.5)).foregroundStyle(TC.sub)
        }
        .frame(width: 660, alignment: .leading)
        .pxCard(padding: 20)
        .foregroundStyle(TC.ink)
    }

    @ViewBuilder private var shipped: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(L.t("ÇIKANLAR")).font(TC.mono(10, .bold)).foregroundStyle(TC.pink).kerning(0.8)
            if data.projects.isEmpty {
                Text(L.t("Bu dönemde kayıt yok.")).font(TC.mono(11.5)).foregroundStyle(TC.sub)
            }
            ForEach(data.projects.prefix(interactive ? data.projects.count : 8)) { pr in
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text(pr.name).font(TC.mono(12.5, .bold))
                        Spacer()
                        Text(pr.time > 0 ? CatController.duration(pr.time) : L.f("%lld commit", pr.commits.count)).font(TC.mono(10.5)).foregroundStyle(TC.sub)
                    }
                    let open = ex.set.contains(pr.name)
                    let shown = Array(pr.commits.reversed().prefix(open ? Int.max : data.period.kind.perProject))
                    ForEach(Array(shown.enumerated()), id: \.offset) { _, c in
                        HStack(alignment: .firstTextBaseline, spacing: 6) {
                            Text("✓").font(TC.mono(11, .bold)).foregroundStyle(TC.pink)
                            Text(c.subject).font(TC.mono(11)).lineLimit(2)
                        }
                    }
                    if pr.commits.count > shown.count {
                        let more = Text(L.f("+%lld commit daha", pr.commits.count - shown.count)).font(TC.mono(10.5, interactive ? .semibold : .regular))
                        if interactive {
                            Button { ex.set.insert(pr.name) } label: { more.foregroundStyle(TC.pink).underline() }.buttonStyle(.plain)
                        } else { more.foregroundStyle(TC.sub) }
                    } else if open && pr.commits.count > data.period.kind.perProject {
                        Button { ex.set.remove(pr.name) } label: {
                            Text(L.t("daha az göster")).font(TC.mono(10.5, .semibold)).foregroundStyle(TC.pink).underline()
                        }.buttonStyle(.plain)
                    }
                    if pr.commits.isEmpty {
                        Text(formatTokens(pr.tokens) + " token").font(TC.mono(10.5)).foregroundStyle(TC.sub)
                    }
                }
            }
            if !interactive && data.projects.count > 8 {
                Text(L.f("+%lld proje daha", data.projects.count - 8)).font(TC.mono(10.5)).foregroundStyle(TC.sub)
            }
        }
    }

    private var numbers: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(L.t("İSTATİSTİKLER")).font(TC.mono(10, .bold)).foregroundStyle(TC.pink).kerning(0.8)
            num(CatController.duration(data.active), L.t("çalışma"))
            num("\(data.commits)", L.t("commit"))
            num("\(data.sessions)", L.t("oturum"))
            num("\(data.projects.count)", L.t("proje"))
            num(formatTokens(data.tokens), "token")
            if data.period.kind != .day, let (day, t) = data.busiest {
                let f: DateFormatter = { let f = DateFormatter(); f.locale = L.locale; f.dateFormat = "d MMM EEE"; return f }()
                VStack(alignment: .leading, spacing: 1) {
                    Text(L.t("EN YOĞUN GÜN")).font(TC.mono(9)).foregroundStyle(TC.sub).kerning(0.6)
                    Text(f.string(from: day)).font(TC.mono(12, .bold))
                    Text(CatController.duration(t)).font(TC.mono(10.5)).foregroundStyle(TC.sub)
                }
            }
        }
    }

    private func num(_ v: String, _ k: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 6) {
            Text(v).font(TC.mono(16, .bold))
            Text(k.uppercased(with: L.locale)).font(TC.mono(9)).foregroundStyle(TC.sub).kerning(0.6)
        }
    }

    /// Bilgisayarda: toplam süre, en çok kullanılan 5 uygulama, toplantı toplamı (başlık ve site yok).
    private var computerRow: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(L.t("BİLGİSAYARDA")).font(TC.mono(10, .bold)).foregroundStyle(TC.pink).kerning(0.8)
                Spacer()
                Text(CatController.duration(data.computer.active)).font(TC.mono(11, .bold))
                if data.meetingTime > 0 { Text("· " + L.t("toplantı") + " " + CatController.duration(data.meetingTime)).font(TC.mono(10.5)).foregroundStyle(TC.sub) }
            }
            let mx = max(1, data.computer.apps.first?.time ?? 1)
            HStack(alignment: .top, spacing: 10) {
                ForEach(data.computer.apps.prefix(5)) { a in
                    VStack(alignment: .leading, spacing: 3) {
                        HStack(spacing: 5) {
                            if let i = AppIcons.icon(a.bundle) { Image(nsImage: i).resizable().frame(width: 14, height: 14) }
                            Text(a.name).font(TC.mono(10.5, .semibold)).lineLimit(1)
                        }
                        Rectangle().fill(TC.pink).frame(width: max(4, 100 * a.time / mx), height: 4)
                        Text(CatController.duration(a.time)).font(TC.mono(9.5)).foregroundStyle(TC.sub)
                    }.frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
    }

    private var chart: some View {
        let mx = max(1, data.buckets.map(\.time).max() ?? 1)
        return HStack(alignment: .bottom, spacing: data.buckets.count > 20 ? 2 : 6) {
            ForEach(Array(data.buckets.enumerated()), id: \.offset) { _, b in
                VStack(spacing: 3) {
                    Rectangle().fill(b.time > 0 ? TC.pink : TC.line)
                        .frame(height: max(3, 54 * b.time / mx))
                    if data.buckets.count <= 12 { Text(b.label).font(TC.mono(9)).foregroundStyle(TC.sub) }
                }
                .frame(maxWidth: .infinity)
            }
        }
        .frame(height: 70, alignment: .bottom)
    }
}
