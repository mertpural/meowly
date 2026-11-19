import AppKit
import SwiftUI

/// Kedinin adını değiştiren piksel metin kutusu. Değişince tüm metinler yeni adla yeniden çizilir.
struct CatNameField: View {
    @EnvironmentObject var state: AppState
    @StateObject private var m = NameModel()
    var body: some View {
        HStack(spacing: 8) {
            TextField("Tonti", text: $m.text)
                .textFieldStyle(.plain)
                .font(TC.mono(12, .bold))
                .foregroundStyle(TC.ink)
                .frame(width: 150)
                .padding(.horizontal, 8).padding(.vertical, 5)
                .background(PixelRect().fill(TC.card))
                .overlay(PixelRect().strokeBorder(TC.ink, lineWidth: 2))
                .onSubmit { CatNamePrompt.save(m.text, state) }
            PxButton(title: L.t("KAYDET"), filled: m.text != CatName.current) { CatNamePrompt.save(m.text, state) }
        }
    }
    final class NameModel: ObservableObject { @Published var text = CatName.current }
}

/// İlk açılışta kedinin adını soran küçük pencere.
@MainActor
enum CatNamePrompt {
    private static var panel: NSPanel?

    static func save(_ raw: String, _ state: AppState?) {
        let n = String(raw.trimmingCharacters(in: .whitespacesAndNewlines).prefix(20))
        UserDefaults.standard.set(n.isEmpty ? "Tonti" : n, forKey: CatName.key)
        state?.objectWillChange.send()
        MenuBarController.shared.refresh()
    }

    static func showIfNeeded(_ state: AppState) {
        guard !CatName.isSet, panel == nil else { return }
        let p = NSPanel(contentRect: NSRect(x: 0, y: 0, width: 380, height: 300), styleMask: [.titled, .closable, .fullSizeContentView], backing: .buffered, defer: false)
        p.titlebarAppearsTransparent = true
        p.titleVisibility = .hidden
        p.isReleasedWhenClosed = false
        p.level = .floating
        p.backgroundColor = NSColor(TC.paper)
        p.appearance = NSAppearance(named: TC.dark ? .darkAqua : .aqua)   // sistem değil, uygulamanın açık/koyu seçimi
        let close = { p.close(); panel = nil }
        let v = FirstMouseHostingView(rootView: AnyView(PromptView(done: { name in save(name, state); close(); Tour.startIfNeeded(state) }).environmentObject(state)))
        v.sizingOptions = []
        p.contentView = v
        p.center()
        panel = p
        NSApp.activate(ignoringOtherApps: true)
        p.makeKeyAndOrderFront(nil)
    }

    private struct PromptView: View {
        let done: (String) -> Void
        @StateObject private var m = CatNameField.NameModel()
        var body: some View {
            VStack(spacing: 14) {
                CatSprite(name: CatAnim.has("content") ? "content" : "idle_0", height: 78)
                Text(L.t("Merhaba! Benim adım ne olsun?")).font(TC.mono(14, .bold))
                Text(L.t("Sonra ayarlardan değiştirebilirsin.")).font(TC.mono(11)).foregroundStyle(TC.sub)
                TextField("Tonti", text: $m.text)
                    .textFieldStyle(.plain)
                    .font(TC.mono(14, .bold))
                    .multilineTextAlignment(.center)
                    .frame(width: 200)
                    .padding(.horizontal, 8).padding(.vertical, 6)
                    .background(PixelRect().fill(TC.card))
                    .overlay(PixelRect().strokeBorder(TC.ink, lineWidth: 2))
                    .onSubmit { done(m.text) }
                PxButton(title: L.t("TAMAM"), filled: true) { done(m.text) }
            }
            .foregroundStyle(TC.ink)
            .padding(24)
            .frame(width: 380, height: 300)
            .background(TC.paper)
        }
    }
}
