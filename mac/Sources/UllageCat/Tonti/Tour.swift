import AppKit
import SwiftUI

/// İlk açılış turu: beş kısa adım, gerçek öğeleri (menü çubuğundaki çip, kedi) pembe bir çerçeveyle ve okla gösterir.
/// Yalnızca yeni kurulumda, kedinin adı girildikten sonra bir kez çıkar; Ayarlar → Hakkında'dan tekrar açılır.
/// Hiçbir şey okunmaz ya da gönderilmez; yalnızca iki küçük pencere çizilir ve tur bitince kapanır.
/// Tur için pencere içindeki öğelerin konumu (SwiftUI pencere koordinatı, sol üst başlangıç).
@MainActor
enum TourAnchors {
    static var rects: [String: CGRect] = [:]
    /// Öğenin ekran dikdörtgeni (AppKit, sol alt başlangıç).
    static func screenRect(_ id: String) -> CGRect? {
        guard let r = rects[id], let w = DashboardWindow.current, w.isVisible else { return nil }
        return CGRect(x: w.frame.minX + r.minX, y: w.frame.maxY - r.maxY, width: r.width, height: r.height)
    }
}

extension View {
    /// Turun gösterebileceği öğe: konumu her değiştiğinde kaydedilir (yalnızca bir sayı, çizim yok).
    func tourAnchor(_ id: String) -> some View {
        background(GeometryReader { g in
            Color.clear
                .onAppear { TourAnchors.rects[id] = g.frame(in: .global) }
                .onChange(of: g.frame(in: .global)) { _, r in TourAnchors.rects[id] = r }
        })
    }
}

/// İlk açılış turu: beş kısa adım. Önce menü çubuğundaki çipi ve kediyi, sonra uygulama penceresini açıp Günlük/Raporlar'ı,
/// Ayarlar'ı ve gizlilik kartını gerçekten gösterir; kart her adımda ilgili öğenin yanında, oku ona dönük durur.
/// Yalnızca yeni kurulumda, kedinin adı girildikten sonra bir kez çıkar; Ayarlar → Hakkında'dan tekrar açılır.
@MainActor
enum Tour {
    static let doneKey = "tourDone"
    private static var card: NSPanel?
    private static var index = 0
    private static weak var state: AppState?

    enum Target { case menubar, cat, window(String, page: String) }
    enum Arrow { case up, down, left, right, none }
    struct Step { let title: String; let body: String; let target: Target; let art: String; let prefer: Arrow }

    static var steps: [Step] { [
        Step(title: L.t("Limitlerin burada"),
             body: L.t("Menü çubuğunda 5 saatlik ve haftalık kalan limitini görürsün. Soldaki kısma basınca ayrıntılar açılır; sağdaki kutuya basınca ben kutuya girerim."),
             target: .menubar, art: "type_0", prefer: .up),
        Step(title: L.t("Bu benim, Tonti"),
             body: L.t("Üstüme gelip imleci gezdirirsen mırlarım. Enseden tutup istediğin yere taşıyabilirsin. Bana tıklarsan uygulamayı açarım."),
             target: .cat, art: "content", prefer: .right),
        Step(title: L.t("Çalışırken not alırım"),
             body: L.t("Claude ya da Codex ile çalışınca Günlük'e yazarım. Raporlar'da günlük, haftalık ve aylık özetin hazır olur; Kullanım'da token'ların. Sohbetlerini okumam."),
             target: .window("sidebar.nav", page: "diary"), art: "idle_2", prefer: .left),
        Step(title: L.t("Her şey ayarlarda"),
             body: L.t("Limit göstergesini, balonları ve bilgisayar günlüğünün her katmanını tek tek açıp kapatabilirsin. İzin gereken şeyleri sen onaylarsın; kendiliğinden hiçbir izin istemem."),
             target: .window("settings.tabs", page: "appearance"), art: "groomC", prefer: .up),
        Step(title: L.t("Hepsi bu Mac'te kalır"),
             body: L.t("Burada neyi not alıp neyi asla almadığımı görürsün. Sunucum ve hesabım yok; hiçbir şey gönderilmez. Bu turu Ayarlar → Hakkında'dan yeniden görebilirsin."),
             target: .window("computer.privacy", page: "computer"), art: "sleep", prefer: .up),
    ] }

    /// Yeni kurulumda kedinin adı girilince çağrılır.
    static func startIfNeeded(_ s: AppState) {
        guard UserDefaults.standard.object(forKey: doneKey) == nil else { return }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.7) { start(s) }
    }

    static func start(_ s: AppState) {
        guard card == nil else { return }
        state = s
        index = ProcessInfo.processInfo.environment["MEOWLY_TOUR_STEP"].flatMap(Int.init) ?? 0   // geliştirme: belirli adımdan başlat
        show()
    }

    private static func finish() {
        card?.close(); card = nil
        UserDefaults.standard.set(true, forKey: doneKey)
    }

    private static let size = CGSize(width: 344, height: 246)

    private static func show() {
        let all = steps
        guard index < all.count else { finish(); return }
        let st = all[index]
        // Pencere adımları: önce ilgili sayfa açılır, yerleşim bitince kart konur.
        if case .window(_, let page) = st.target, let s = state {
            card?.orderOut(nil)
            s.openDashboard(page)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.45) { place(st) }
        } else {
            place(st)
        }
    }

    private static func place(_ st: Step) {
        let screen = NSScreen.main?.visibleFrame ?? CGRect(x: 0, y: 0, width: 1440, height: 900)
        var target: CGRect?
        switch st.target {
        case .menubar: target = MenuBarController.shared.buttonFrame
        case .cat: target = CatController.shared.catFrame ?? MenuBarController.shared.buttonFrame
        case .window(let id, _): target = TourAnchors.screenRect(id)
        }

        func clampX(_ x: CGFloat) -> CGFloat { min(max(x, screen.minX + 8), screen.maxX - size.width - 8) }
        func clampY(_ y: CGFloat) -> CGFloat { min(max(y, screen.minY + 8), screen.maxY - size.height - 8) }
        var origin = CGPoint(x: screen.midX - size.width / 2, y: screen.midY - size.height / 2 + 40)
        var arrow = Arrow.none, arrowAt: CGFloat = 0
        if let t = target {
            var dir = st.prefer
            // Tercih edilen tarafa sığmıyorsa karşı tarafa geçilir.
            if dir == .right, t.minX - size.width < screen.minX { dir = .left }
            if dir == .left, t.maxX + size.width > screen.maxX { dir = .right }
            if dir == .up, t.minY - size.height < screen.minY { dir = .down }
            switch dir {
            case .up:    origin = CGPoint(x: clampX(t.midX - size.width / 2), y: t.minY - size.height + 4); arrowAt = t.midX - origin.x
            case .down:  origin = CGPoint(x: clampX(t.midX - size.width / 2), y: t.maxY - 4); arrowAt = t.midX - origin.x
            case .right: origin = CGPoint(x: t.minX - size.width + 2, y: clampY(t.midY - size.height / 2)); arrowAt = t.midY - origin.y
            case .left:  origin = CGPoint(x: t.maxX - 2, y: clampY(t.midY - size.height / 2)); arrowAt = t.midY - origin.y
            case .none:  break
            }
            arrow = dir
        }

        let view = TourCard(step: st, index: index, count: steps.count, arrow: arrow, arrowAt: arrowAt,
                            next: { index += 1; show() }, skip: { finish() })
        if card == nil {
            let p = NSPanel(contentRect: CGRect(origin: origin, size: size), styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
            p.isOpaque = false; p.backgroundColor = .clear; p.hasShadow = false
            p.level = .popUpMenu
            p.hidesOnDeactivate = false
            p.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
            p.isReleasedWhenClosed = false
            card = p
        }
        guard let p = card else { return }
        p.appearance = NSAppearance(named: TC.dark ? .darkAqua : .aqua)
        let host = FirstMouseHostingView(rootView: AnyView(view))
        host.sizingOptions = []
        p.contentView = host
        p.setFrame(CGRect(origin: origin, size: size), display: true)
        p.orderFrontRegardless()
    }
}

/// Tur kartı: kedi resmi, başlık, kısa metin, ilerleme noktaları ve düğmeler. Okun ucu gösterilen öğeye bakar.
private struct TourCard: View {
    let step: Tour.Step, index: Int, count: Int
    let arrow: Tour.Arrow, arrowAt: CGFloat
    let next: () -> Void, skip: () -> Void

    private var alignment: Alignment {
        switch arrow { case .up: .top; case .down: .bottom; case .right: .trailing; case .left: .leading; case .none: .center }
    }

    var body: some View {
        ZStack {
            VStack(alignment: .leading, spacing: 10) {
                HStack(alignment: .bottom, spacing: 12) {
                    CatSprite(name: step.art, height: 52)
                    Text(step.title).font(TC.mono(14, .bold)).fixedSize(horizontal: false, vertical: true)
                }
                Text(step.body).font(TC.mono(11)).foregroundStyle(TC.sub).fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 0)
                HStack {
                    HStack(spacing: 5) {
                        ForEach(0..<count, id: \.self) { i in
                            Rectangle().fill(i == index ? TC.pink : TC.line).frame(width: i == index ? 14 : 7, height: 7)
                        }
                    }
                    Spacer()
                    if index < count - 1 {
                        Button(L.t("ATLA"), action: skip).buttonStyle(.plain).font(TC.mono(10.5, .bold)).foregroundStyle(TC.sub)
                    }
                    PxButton(title: index < count - 1 ? L.t("İLERİ →") : L.t("BAŞLAYALIM"), filled: true, action: next)
                }
            }
            .foregroundStyle(TC.ink)
            .padding(16)
            .frame(width: 316, height: 218)
            .background(PixelRect().fill(TC.card).shadow(color: TC.shadow, radius: 0, x: 4, y: 4))   // gölge yalnızca zemine, yazıya değil
            .overlay(PixelRect().strokeBorder(TC.ink, lineWidth: 2))
            .frame(width: 344, height: 246)
            .overlay(alignment: alignment) { arrowShape }
        }
        .frame(width: 344, height: 246)
    }

    @ViewBuilder private var arrowShape: some View {
        switch arrow {
        case .up: PixelArrow(dir: 0).frame(width: 26, height: 12).offset(x: arrowAt - 172, y: 3)
        case .down: PixelArrow(dir: 1).frame(width: 26, height: 12).offset(x: arrowAt - 172, y: -3)
        case .right: PixelArrow(dir: 2).frame(width: 12, height: 26).offset(x: -3, y: 123 - arrowAt)
        case .left: PixelArrow(dir: 3).frame(width: 12, height: 26).offset(x: 3, y: 123 - arrowAt)
        case .none: EmptyView()
        }
    }
}

/// Basamaklı (piksel) üçgen ok: ucu 0 yukarı, 1 aşağı, 2 sağa.
private struct PixelArrow: View {
    let dir: Int
    var body: some View {
        Canvas { ctx, size in
            let rows = 6, cell: CGFloat = 2
            for i in 0..<rows {
                let w = CGFloat(i * 2 + 2) * cell
                var r: CGRect
                switch dir {
                case 0: r = CGRect(x: (size.width - w) / 2, y: CGFloat(i) * cell, width: w, height: cell)              // ucu yukarı
                case 1: r = CGRect(x: (size.width - w) / 2, y: CGFloat(rows - 1 - i) * cell, width: w, height: cell)   // ucu aşağı
                case 2: r = CGRect(x: CGFloat(rows - 1 - i) * cell, y: (size.height - w) / 2, width: cell, height: w) // ucu sağa
                default: r = CGRect(x: CGFloat(i) * cell, y: (size.height - w) / 2, width: cell, height: w)            // ucu sola
                }
                ctx.fill(Path(r), with: .color(TC.ink))
                let inner = r.insetBy(dx: dir >= 2 ? 0 : 2, dy: dir >= 2 ? 2 : 0)
                if inner.width > 0 && inner.height > 0 && i > 0 { ctx.fill(Path(inner), with: .color(TC.card)) }
            }
        }
    }
}
