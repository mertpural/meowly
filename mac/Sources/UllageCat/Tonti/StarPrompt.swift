import AppKit
import SwiftUI

/// GitHub'da yıldız rica eder: yalnızca nazikçe sorar, karşılığında hiçbir şey vermez (GitHub kuralları).
/// En az 7 gün kullanımdan sonra bir kez sorar; "Sonra" denirse 14 gün sonra son bir kez daha sorar; sonra hiç sormaz.
/// Hiçbir şey gönderilmez; "Yıldız ver" yalnızca depo sayfasını tarayıcıda açar.
@MainActor
enum StarPrompt {
    static let repoURL = URL(string: "https://github.com/mertpural/meowly")!
    static let issuesURL = URL(string: "https://github.com/mertpural/meowly/issues")!
    private static let d = UserDefaults.standard
    private static var panel: NSPanel?

    /// İlk kullanım anı (bir kez yazılır).
    static func markFirstUse() {
        if d.double(forKey: "firstUseAt") == 0 { d.set(Date().timeIntervalSince1970, forKey: "firstUseAt") }
    }

    /// 0: henüz sorulmadı, 1: "Sonra" dendi, 2: bitti (verdi ya da bir daha sorma).
    private static var state: Int { d.integer(forKey: "starAskState") }

    static var isDue: Bool {
        if ProcessInfo.processInfo.environment["MEOWLY_STAR"] != nil { return true }   // geliştirme
        let now = Date().timeIntervalSince1970
        let first = d.double(forKey: "firstUseAt")
        guard first > 0 else { return false }
        switch state {
        case 0: return now - first >= 7 * 86400
        case 1: return now - d.double(forKey: "starAskAt") >= 14 * 86400
        default: return false
        }
    }

    static func showIfDue(_ state: AppState) {
        guard panel == nil, CatName.isSet, isDue else { return }
        let p = NSPanel(contentRect: NSRect(x: 0, y: 0, width: 400, height: 300), styleMask: [.titled, .closable, .fullSizeContentView], backing: .buffered, defer: false)
        p.titlebarAppearsTransparent = true
        p.titleVisibility = .hidden
        p.isReleasedWhenClosed = false
        p.level = .floating
        p.backgroundColor = NSColor(TC.paper)
        p.appearance = NSAppearance(named: TC.dark ? .darkAqua : .aqua)
        let finish: (Int) -> Void = { s in
            d.set(s, forKey: "starAskState"); d.set(Date().timeIntervalSince1970, forKey: "starAskAt")
            p.close(); panel = nil
        }
        let v = FirstMouseHostingView(rootView: AnyView(PromptView(
            star: { NSWorkspace.shared.open(repoURL); finish(2) },
            later: { finish(self.state == 0 ? 1 : 2) },
            never: { finish(2) }
        ).environmentObject(state)))
        v.sizingOptions = []
        p.contentView = v
        p.center()
        panel = p
        NSApp.activate(ignoringOtherApps: true)
        p.makeKeyAndOrderFront(nil)
    }

    private struct PromptView: View {
        let star: () -> Void, later: () -> Void, never: () -> Void
        var body: some View {
            VStack(spacing: 12) {
                CatSprite(name: CatAnim.has("content") ? "content" : "idle_0", height: 74)
                Text(L.t("Beni sevdiysen GitHub'da bir yıldız verir misin?")).font(TC.mono(13.5, .bold)).multilineTextAlignment(.center)
                Text(L.t("Böylece başkaları da Meowly'yi bulur. Yalnızca bir kez soruyorum."))
                    .font(TC.mono(11)).foregroundStyle(TC.sub).multilineTextAlignment(.center)
                HStack(spacing: 8) {
                    PxButton(title: L.t("⭐ YILDIZ VER"), filled: true, action: star)
                    PxButton(title: L.t("SONRA"), action: later)
                }
                Button(L.t("Bir daha sorma"), action: never).buttonStyle(.plain).font(TC.mono(10.5)).foregroundStyle(TC.sub)
            }
            .foregroundStyle(TC.ink)
            .padding(24)
            .frame(width: 400, height: 300)
            .background(TC.paper)
        }
    }
}
