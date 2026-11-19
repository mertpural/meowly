import SwiftUI
import AppKit
import ApplicationServices

// MARK: - Ayarlar: Bilgisayar günlüğü

@MainActor
final class LogSettingsModel: ObservableObject {
    @Published var tick = 0
    private var obs: NSObjectProtocol?
    init() {
        // Sistem Ayarları'ndan dönünce izin durumları yeniden okunur.
        obs = NotificationCenter.default.addObserver(forName: NSApplication.didBecomeActiveNotification, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.tick += 1 }
        }
    }
    func refresh() { tick += 1 }
}

struct ComputerLogSettings: View {
    @EnvironmentObject var state: AppState
    @StateObject private var m = LogSettingsModel()
    @StateObject private var confirm = Flag()
    final class Flag: ObservableObject { @Published var on = false; @Published var note = "" }

    var body: some View {
        let t = state.theme
        let _ = m.tick
        VStack(alignment: .leading, spacing: 10) {
            Text(L.t("Tonti bilgisayarda ne yaptığını not alır; Günlük ve Raporlar bununla dolar. Her şey yalnızca bu Mac'te kalır, hiçbir yere gönderilmez."))
                .font(t.font(12.5, .semibold)).fixedSize(horizontal: false, vertical: true)
            HStack(alignment: .top, spacing: 24) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(L.t("NOT ALIR")).font(t.font(10, .bold)).foregroundStyle(TC.pink)
                    bullet(L.t("hangi uygulamada ne kadar kaldığın"))
                    bullet(L.t("pencere başlıkları (dosya, belge adı)"))
                    bullet(L.t("tarayıcıda yalnızca site adı (github.com gibi)"))
                    bullet(L.t("projelerinde kaç dosya değişikliği oldu"))
                    bullet(L.t("takvimdeki toplantıların"))
                }
                VStack(alignment: .leading, spacing: 4) {
                    Text(L.t("ASLA ALMAZ")).font(t.font(10, .bold)).foregroundStyle(TC.pink)
                    bullet(L.t("ekran görüntüsü ya da ekran kaydı"))
                    bullet(L.t("bastığın tuşlar, yazdığın metinler"))
                    bullet(L.t("sayfa içerikleri, tam adresler, sohbetler"))
                    bullet(L.t("dosyaların içeriği"))
                    bullet(L.t("parola uygulamalarının pencere adları"))
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .card(t, padding: 16)
        .tourAnchor("computer.privacy")

        layerRow(.apps, L.t("Uygulama süreleri"),
                 L.t("Öndeki uygulama değiştikçe not alınır; 5 dakika fare ve klavye kullanmazsan boşta sayılır. İzin gerekmez."))
        layerRow(.titles, L.t("Pencere başlıkları"),
                 L.t("Hangi dosyada ya da belgede çalıştığını gösterir (\"CatPet.swift — ullage-cat\" gibi). Erişilebilirlik izni ister; bu izin yalnızca öndeki pencerenin adını okumak için kullanılır, tuşlarına ya da ekranına bakmaz. Tarayıcılarda başlık saklanmaz."))
        layerRow(.sites, L.t("Tarayıcı siteleri"),
                 L.t("Tarayıcıda hangi sitede ne kadar kaldığını yalnızca alan adıyla tutar (youtube.com gibi). Sayfa başlığı, tam adres ve içerik saklanmaz; gizli/incognito pencereler atlanır. Chrome, Arc, Brave, Edge ve Safari için tarayıcı başına bir kez izin sorulabilir; Firefox desteklenmez."))
        layerRow(.files, L.t("Dosya değişiklikleri"),
                 L.t("Yapay zekâ araçlarıyla çalıştığın proje klasörlerinde saatte kaç değişiklik olduğunu sayar. Dosya adı ve içeriği saklanmaz. macOS ilk seferde klasör erişimi sorabilir."))
        layerRow(.calendar, L.t("Takvim"),
                 L.t("Günlükte ve raporlarda toplantılarını gösterir. Takvimi yalnızca okur, değiştirmez. Paylaşılan raporlara toplantı adları girmez, yalnızca toplam süre."))

        HStack(spacing: 10) {
            if confirm.on {
                Text(L.t("Tüm bilgisayar kayıtları silinecek. Emin misin?")).font(t.font(12, .semibold))
                PxButton(title: L.t("SİL"), filled: true) {
                    HistoryStore.shared.deleteComputerLog(); confirm.on = false; confirm.note = L.t("Silindi")
                }
                PxButton(title: L.t("VAZGEÇ")) { confirm.on = false }
            } else {
                PxButton(title: L.t("BİLGİSAYAR KAYITLARINI SİL")) { confirm.on = true; confirm.note = "" }
                Text(confirm.note.isEmpty ? L.t("Yapay zekâ oturumları ve commit'ler etkilenmez.") : confirm.note)
                    .font(t.font(11)).foregroundStyle(t.sub)
            }
        }
    }

    private func bullet(_ s: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 6) {
            Text("·").font(TC.mono(12, .bold))
            Text(s).font(TC.mono(11.5)).foregroundStyle(TC.sub).fixedSize(horizontal: false, vertical: true)
        }
    }

    private func layerRow(_ l: LogLayer, _ title: String, _ sub: String) -> some View {
        let on = l.on
        let acc = l.access
        return SettingRow(title: title, subtitle: sub) {
            VStack(alignment: .trailing, spacing: 6) {
                Toggle("", isOn: Binding(get: { on }, set: { l.set($0); ComputerLog.shared.settingsChanged(); m.refresh() }))
                    .toggleStyle(PxSwitch()).labelsHidden()
                if on {
                    switch acc {
                    case .ok: Text(L.t("Çalışıyor")).font(TC.mono(10.5, .semibold)).foregroundStyle(TC.sub)
                    case .notNeeded: Text(L.t("İzin gerekmez")).font(TC.mono(10.5)).foregroundStyle(TC.sub)
                    case .needed:
                        PxButton(title: L.t("İZİN VER"), filled: true) { grant(l) }
                    }
                }
            }
        }
    }

    private func grant(_ l: LogLayer) {
        switch l {
        case .titles:
            _ = AXIsProcessTrustedWithOptions([kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary)
        case .sites:
            if !AXIsProcessTrusted() {
                _ = AXIsProcessTrustedWithOptions([kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary)
            }
            // Açık tarayıcılar için otomasyon izni (her biri için bir kez sorulur).
            let running = NSWorkspace.shared.runningApplications.compactMap(\.bundleIdentifier).filter { AppCategory.browsers.contains($0) && $0 != "org.mozilla.firefox" }
            DispatchQueue.global().async { for b in running { _ = Browser.automation(b, ask: true) } }
        case .calendar:
            CalendarLog.request { _ in DispatchQueue.main.async { m.refresh() } }
        case .apps, .files: break
        }
        m.refresh()
    }
}

/// Günlük'ün üstünde, izin bekleyen açık katmanlar varsa bir kez gösterilen kart.
struct PermissionNudge: View {
    @EnvironmentObject var state: AppState
    @AppStorage("logNudgeHidden") private var hidden = false
    var body: some View {
        let missing = LogLayer.allCases.filter { $0.on && $0.access == .needed }
        if !hidden && !missing.isEmpty {
            HStack(spacing: 12) {
                CatSprite(name: "idle_2", height: 40)
                VStack(alignment: .leading, spacing: 3) {
                    Text(L.t("Tonti daha fazlasını not alabilir")).font(TC.mono(12.5, .bold))
                    // Yalnızca gerçekten izni eksik olan katmanlar yazılır.
                    let names = missing.map { l -> String in
                        switch l {
                        case .titles: return L.t("pencere başlıkları")
                        case .sites: return L.t("siteler")
                        case .calendar: return L.t("takvim")
                        default: return ""
                        }
                    }.filter { !$0.isEmpty }
                    Text(L.f("İzin gerekenler: %@. Ne aldığını ve ne almadığını ayarlarda tek tek anlattım.", names.joined(separator: ", ")))
                        .font(TC.mono(11)).foregroundStyle(TC.sub).fixedSize(horizontal: false, vertical: true)
                }
                Spacer()
                PxButton(title: L.t("AYARLARA GİT"), filled: true) { state.dashPage = "computer" }
                PxButton(title: "✕") { hidden = true }
            }
            .pxCard(padding: 12)
        }
    }
}

// MARK: - Günlük: bilgisayar görünümü

struct ComputerDay {
    struct App: Identifiable {
        let bundle: String, name: String, category: String
        var time: TimeInterval
        var titles: [(String, TimeInterval)]
        var domains: [(String, TimeInterval)]
        var id: String { bundle }
    }
    var apps: [App] = []
    var active: TimeInterval = 0
    var hours = [TimeInterval](repeating: 0, count: 24)
    var categories: [(String, TimeInterval)] = []
    var files: [(project: String, count: Int)] = []
    var meetings: [CalendarLog.Meeting] = []

    /// Raporlar için hafif özet: yalnızca uygulama toplamları (başlık/site yok), toplantılar.
    static func summary(from: Date, to: Date) -> ComputerDay {
        var d = ComputerDay()
        var cats: [String: TimeInterval] = [:]
        for a in HistoryStore.shared.appTotals(from: from, to: to) where a.time > 0 {
            d.apps.append(App(bundle: a.bundle, name: a.app, category: a.category, time: a.time, titles: [], domains: []))
            d.active += a.time
            cats[a.category, default: 0] += a.time
        }
        d.categories = cats.sorted { $0.value > $1.value }.map { ($0.key, $0.value) }
        d.meetings = CalendarLog.meetings(from: from, to: to)
        return d
    }

    static func build(from: Date, to: Date) -> ComputerDay {
        var d = ComputerDay()
        let spans = HistoryStore.shared.spans(from: from, to: to).filter { $0.duration >= 3 }
        var byApp: [String: (name: String, cat: String, time: TimeInterval, titles: [String: TimeInterval], domains: [String: TimeInterval])] = [:]
        var cats: [String: TimeInterval] = [:]
        let cal = Calendar.current
        for s in spans {
            var v = byApp[s.bundle] ?? (s.app, s.category, 0, [:], [:])
            v.time += s.duration
            if let t = s.title { v.titles[t, default: 0] += s.duration }
            if let dm = s.domain { v.domains[dm, default: 0] += s.duration }
            byApp[s.bundle] = v
            cats[s.category, default: 0] += s.duration
            d.active += s.duration
            // Saatlik dağılım (saat sınırını aşan kayıtlar bölünür).
            var a = s.start
            while a < s.end {
                let h = cal.component(.hour, from: a)
                let next = min(s.end, cal.dateInterval(of: .hour, for: a)?.end ?? s.end)
                d.hours[h] += next.timeIntervalSince(a)
                a = next
            }
        }
        d.apps = byApp.map { b, v in
            App(bundle: b, name: v.name, category: v.cat, time: v.time,
                titles: v.titles.sorted { $0.value > $1.value }.prefix(6).map { ($0.key, $0.value) },
                domains: v.domains.sorted { $0.value > $1.value }.prefix(8).map { ($0.key, $0.value) })
        }.sorted { $0.time > $1.time }
        d.categories = cats.sorted { $0.value > $1.value }.map { ($0.key, $0.value) }
        d.files = HistoryStore.shared.fileActivity(from: from, to: to)
        d.meetings = CalendarLog.meetings(from: from, to: to)
        return d
    }
}

enum AppIcons {
    private static var cache: [String: NSImage] = [:]
    static func icon(_ bundle: String) -> NSImage? {
        if let i = cache[bundle] { return i }
        guard let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundle) else { return nil }
        let i = NSWorkspace.shared.icon(forFile: url.path)
        i.size = NSSize(width: 32, height: 32)
        cache[bundle] = i
        return i
    }
}

struct ComputerDayView: View {
    let data: ComputerDay
    @StateObject private var ex = Expanded()
    final class Expanded: ObservableObject { @Published var set: Set<String> = [] }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if data.apps.isEmpty {
                VStack(spacing: 10) {
                    CatSprite(name: "sleep", height: 64)
                    Text(L.t("Bu gün için bilgisayar kaydı yok.")).font(TC.mono(12, .semibold))
                    Text(L.t("Tonti, bilgisayar günlüğü açıkken uygulamaları not alır.")).font(TC.mono(11)).foregroundStyle(TC.sub)
                }.frame(maxWidth: .infinity).padding(.top, 50)
            } else {
                hourStrip
                ForEach(data.apps.prefix(25)) { a in appRow(a) }
            }
        }
    }

    private var hourStrip: some View {
        let mx = max(60, data.hours.max() ?? 60)
        return VStack(alignment: .leading, spacing: 6) {
            Text(L.t("SAAT SAAT")).font(TC.mono(10, .bold)).foregroundStyle(TC.sub).kerning(0.8)
            HStack(alignment: .bottom, spacing: 3) {
                ForEach(0..<24, id: \.self) { h in
                    VStack(spacing: 3) {
                        Rectangle().fill(data.hours[h] > 0 ? TC.pink : TC.line).frame(height: max(3, 44 * data.hours[h] / mx))
                        Text(h % 3 == 0 ? String(format: "%02d", h) : " ").font(TC.mono(8.5)).foregroundStyle(TC.sub)
                    }.frame(maxWidth: .infinity)
                }
            }.frame(height: 60, alignment: .bottom)
        }
        .frame(maxWidth: .infinity, alignment: .leading).pxCard()
    }

    private func appRow(_ a: ComputerDay.App) -> some View {
        let open = ex.set.contains(a.bundle)
        let details = a.titles.isEmpty ? a.domains : a.titles
        return VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 10) {
                if let i = AppIcons.icon(a.bundle) { Image(nsImage: i).resizable().frame(width: 22, height: 22) }
                VStack(alignment: .leading, spacing: 1) {
                    Text(a.name).font(TC.mono(12.5, .bold))
                    Text(L.t(a.category)).font(TC.mono(9.5)).foregroundStyle(TC.sub)
                }
                Spacer()
                Text(CatController.duration(a.time)).font(TC.mono(12, .semibold))
            }
            GeometryReader { g in Rectangle().fill(TC.pink).frame(width: g.size.width * a.time / max(1, data.apps.first?.time ?? 1)) }.frame(height: 4)
            if !details.isEmpty {
                ForEach(Array(details.prefix(open ? 8 : 2).enumerated()), id: \.offset) { _, d in
                    HStack {
                        Text(d.0).font(TC.mono(11)).lineLimit(1).truncationMode(.middle)
                        Spacer()
                        Text(CatController.duration(d.1)).font(TC.mono(10)).foregroundStyle(TC.sub)
                    }
                }
                if details.count > 2 {
                    Button { if open { ex.set.remove(a.bundle) } else { ex.set.insert(a.bundle) } } label: {
                        Text(open ? L.t("daha az göster") : L.f("+%lld daha", details.count - 2)).font(TC.mono(10.5, .semibold)).foregroundStyle(TC.pink).underline()
                    }.buttonStyle(.plain)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .pxCard()
    }
}

/// Bilgisayar görünümünün sağ sütunu.
struct ComputerSide: View {
    let data: ComputerDay
    private static let hm: DateFormatter = { let f = DateFormatter(); f.dateFormat = "HH:mm"; return f }()

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            VStack(alignment: .leading, spacing: 8) {
                Text(L.t("BİLGİSAYARDA")).font(TC.mono(10, .bold)).foregroundStyle(TC.sub).kerning(0.8)
                Text(CatController.duration(data.active)).font(TC.mono(18, .bold))
                ForEach(data.categories.prefix(6), id: \.0) { c in
                    HStack { Text(L.t(c.0)).font(TC.mono(11)); Spacer(); Text(CatController.duration(c.1)).font(TC.mono(10.5)).foregroundStyle(TC.sub) }
                }
            }.frame(maxWidth: .infinity, alignment: .leading).pxCard()

            if !data.meetings.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text(L.t("TAKVİM")).font(TC.mono(10, .bold)).foregroundStyle(TC.sub).kerning(0.8)
                    ForEach(data.meetings, id: \.self) { mt in
                        VStack(alignment: .leading, spacing: 1) {
                            Text(mt.title).font(TC.mono(11, .semibold)).lineLimit(1)
                            Text("\(Self.hm.string(from: mt.start))–\(Self.hm.string(from: mt.end))").font(TC.mono(10)).foregroundStyle(TC.sub)
                        }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading).pxCard()
            }

            if !data.files.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    Text(L.t("DOSYA DEĞİŞİKLİKLERİ")).font(TC.mono(10, .bold)).foregroundStyle(TC.sub).kerning(0.8)
                    ForEach(data.files.prefix(6), id: \.project) { f in
                        HStack { Text(f.project).font(TC.mono(11, .semibold)).lineLimit(1); Spacer(); Text("\(f.count)").font(TC.mono(10.5)).foregroundStyle(TC.sub) }
                    }
                }.frame(maxWidth: .infinity, alignment: .leading).pxCard()
            }
            Spacer()
        }
    }
}
