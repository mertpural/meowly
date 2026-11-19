import AppKit
import IOKit.pwr_mgt
import Combine
import SwiftUI
import QuartzCore

/// Masaüstündeki Tonti: tek küçük şeffaf pencere, üstünde gerektiğinde konuşma balonu.
/// Pil için: sürekli zamanlayıcı yok. Kareler GPU'da (Core Animation) döner; diğer hareketler tek seferlik gecikmeli işler.
/// Ekran uyurken ya da düşük güç modunda hiçbir şey çalışmaz.

// MARK: - Sprite'lar

final class CatSprites {
    static let shared = CatSprites()
    private var cache: [String: CGImage] = [:]

    func image(_ name: String) -> CGImage? {
        if let c = cache[name] { return c }
        guard let url = Bundle.main.url(forResource: name, withExtension: "png", subdirectory: "Cat"),
              let src = CGImageSourceCreateWithURL(url as CFURL, nil),
              let img = CGImageSourceCreateImageAtIndex(src, 0, nil) else { return nil }
        cache[name] = img
        return img
    }
}

// MARK: - Görünüm

/// Kedinin çizildiği görünüm. Kareler ve nefes GPU'da (Core Animation) döner; zamanlayıcı yok.
final class CatImageView: NSView {
    var onClick: (() -> Void)?
    var onDragStart: ((CGPoint) -> Void)?
    var onDrag: ((CGPoint) -> Void)?          // imlecin ekrandaki yeri
    var onDragEnd: (() -> Void)?
    var onMenu: ((NSEvent) -> Void)?
    var onStroke: (() -> Void)?               // imleç kedinin üstünde gezdirildi (okşama)
    let sprite = CALayer()
    /// Yalama gibi hareketlerde başın ayrı oynaması için: aynı karenin üst kısmını gösterir, boyun noktasından iner.
    let head = CALayer()
    private var downAt: CGPoint?
    private var dragged = false
    private var lastHover: CGPoint?
    private var stroke: CGFloat = 0

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.masksToBounds = false
        sprite.magnificationFilter = .nearest
        sprite.contentsScale = 2
        layer?.addSublayer(sprite)
        head.magnificationFilter = .nearest
        head.contentsScale = 2
        head.contentsGravity = .bottom
        head.isHidden = true
        layer?.addSublayer(head)
        anchorBottom()
    }
    required init?(coder: NSCoder) { nil }

    override func layout() { super.layout(); sprite.bounds = bounds }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        trackingAreas.forEach(removeTrackingArea)
        addTrackingArea(NSTrackingArea(rect: .zero, options: [.mouseMoved, .mouseEnteredAndExited, .activeAlways, .inVisibleRect], owner: self))
    }

    /// Normal duruş: alttan hizalı, nefes alttan büyür.
    func anchorBottom() {
        CATransaction.begin(); CATransaction.setDisableActions(true)
        sprite.bounds = bounds
        sprite.contentsGravity = .bottom
        sprite.anchorPoint = CGPoint(x: 0.5, y: 0)
        sprite.position = CGPoint(x: bounds.midX, y: 0)
        sprite.transform = CATransform3DIdentity
        CATransaction.commit()
    }

    /// Baş/gövde ayrımı. `imageH`: karenin yüksekliği (pt), `frac`: üstten başın kapladığı oran.
    func split(_ on: Bool, imageH: CGFloat = 0, frac: CGFloat = 0.52) {
        CATransaction.begin(); CATransaction.setDisableActions(true)
        head.removeAllAnimations()
        if on {
            // Kesimi tam piksel sınırına oturt ve baş katmanını gövdenin 3 piksel üstüne bindir: boyunda çizgi kalmasın.
            let px = max(1, imageH * 2)
            let cut = (px * (1 - frac)).rounded() / px          // gövde oranı (alttan)
            let lap: CGFloat = 3 / px
            let neck = imageH * cut
            sprite.contentsRect = CGRect(x: 0, y: 0, width: 1, height: cut)               // gövde: alt kısım (macOS'ta y aşağıdan başlar)
            head.contentsRect = CGRect(x: 0, y: cut - lap, width: 1, height: 1 - cut + lap) // baş: üst kısım + bindirme
            head.bounds = CGRect(x: 0, y: 0, width: bounds.width, height: imageH * (1 - cut + lap))
            head.anchorPoint = CGPoint(x: 0.5, y: lap / (1 - cut + lap))
            head.position = CGPoint(x: bounds.midX, y: neck)
            head.transform = CATransform3DIdentity
            head.isHidden = false
        } else {
            sprite.contentsRect = CGRect(x: 0, y: 0, width: 1, height: 1)
            head.isHidden = true
            head.contents = nil
        }
        CATransaction.commit()
    }

    /// Enseden tutulma: üstten asılı, üst orta noktadan (ense) sallanır.
    func anchorTop() {
        CATransaction.begin(); CATransaction.setDisableActions(true)
        sprite.bounds = bounds
        sprite.contentsGravity = .top
        sprite.anchorPoint = CGPoint(x: 0.5, y: 1)
        sprite.position = CGPoint(x: bounds.midX, y: bounds.maxY)
        CATransaction.commit()
    }

    // MARK: Esnek sarkma: kare yatay şeritlere bölünür, her şerit ensedeki uzaklığına göre yana kayar.
    // Baş neredeyse sabit kalır, gövde yumuşakça kıvrılır, kalça ve bacaklar en çok sallanır; tek parça dönmez.

    private var strips: [CALayer] = []
    private var stripCenters: [CGFloat] = []      // 0 = ense, 1 = ayak ucu
    private var bendScale: CGFloat = 2

    func startBend(_ img: CGImage, scale: CGFloat) {
        let count = max(8, img.height / 2)          // her şerit 2 piksel satırı: kenarlar düzgün piksel basamağı olur
        endBend()
        bendScale = scale
        let w = CGFloat(img.width) / scale, h = CGFloat(img.height) / scale
        let x0 = (bounds.midX - w / 2).rounded(), top = bounds.maxY
        let sh = h / CGFloat(count)
        CATransaction.begin(); CATransaction.setDisableActions(true)
        for i in 0..<count {
            let l = CALayer()
            l.contents = img
            l.contentsScale = scale
            l.contentsGravity = .resize
            l.magnificationFilter = .nearest; l.minificationFilter = .nearest
            // macOS'ta contentsRect alttan başlar: i. şerit üstten i. dilim. Dilimler yarım nokta bindirilir, aralarında çizgi kalmaz.
            l.contentsRect = CGRect(x: 0, y: 1 - CGFloat(i + 1) / CGFloat(count), width: 1, height: 1 / CGFloat(count))
            l.frame = CGRect(x: x0, y: top - CGFloat(i + 1) * sh - 0.5, width: w, height: sh + 1)
            layer?.addSublayer(l)
            strips.append(l)
            stripCenters.append((CGFloat(i) + 0.5) / CGFloat(count))
        }
        sprite.isHidden = true
        CATransaction.commit()
    }

    /// `offset(s)`: ense uzaklığına (0…1) göre yatay kayma (pt).
    func bend(_ offset: (CGFloat) -> CGFloat) {
        CATransaction.begin(); CATransaction.setDisableActions(true)
        // Kayma kaynak pikseline yuvarlanır: piksel çizim bozulmaz, bükülme el çizimi basamak gibi görünür.
        for (l, s) in zip(strips, stripCenters) { l.transform = CATransform3DMakeTranslation((offset(s) * bendScale).rounded() / bendScale, 0, 0) }
        CATransaction.commit()
    }

    func endBend() {
        guard !strips.isEmpty else { return }
        CATransaction.begin(); CATransaction.setDisableActions(true)
        strips.forEach { $0.removeFromSuperlayer() }
        strips = []; stripCenters = []
        sprite.isHidden = false
        CATransaction.commit()
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
    override func mouseDown(with e: NSEvent) { downAt = NSEvent.mouseLocation; dragged = false }
    override func mouseDragged(with e: NSEvent) {
        guard let d = downAt else { return }
        let p = NSEvent.mouseLocation
        if !dragged, hypot(p.x - d.x, p.y - d.y) > 3 { dragged = true; onDragStart?(p) }
        if dragged { onDrag?(p) }
    }
    override func mouseUp(with e: NSEvent) {
        if dragged { onDragEnd?() } else { onClick?() }
        downAt = nil; dragged = false
    }
    override func rightMouseDown(with e: NSEvent) { onMenu?(e) }

    // Okşama: imleç kedinin üstünde bir süre gidip gelirse.
    override func mouseEntered(with e: NSEvent) { lastHover = nil; stroke = 0 }
    override func mouseExited(with e: NSEvent) { lastHover = nil; stroke = 0 }
    override func mouseMoved(with e: NSEvent) {
        guard downAt == nil else { return }
        let p = NSEvent.mouseLocation
        if let l = lastHover { stroke += hypot(p.x - l.x, p.y - l.y) }
        lastHover = p
        if stroke > 160 { stroke = 0; onStroke?() }
    }
}

private func makePanel(size: CGSize) -> NSPanel {
    let p = NSPanel(contentRect: CGRect(origin: .zero, size: size), styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: true)
    p.isOpaque = false
    p.backgroundColor = .clear
    p.hasShadow = false
    p.level = .floating
    p.collectionBehavior = [.canJoinAllSpaces, .stationary, .ignoresCycle]
    p.isMovableByWindowBackground = false
    p.hidesOnDeactivate = false
    return p
}

// MARK: - Konuşma balonu

/// Piksel kenarlı küçük balon: koyu dış çizgi, beyaz iç, alt ortada basamaklı kuyruk. Yalnızca metin değişince çizilir.
final class BubbleView: NSView {
    var text = "" { didSet { needsDisplay = true } }
    /// Kuyruğun x konumu (balon kenara dayanınca kuyruk yine kediyi gösterir).
    var tailX: CGFloat? { didSet { needsDisplay = true } }
    static let ink = NSColor(srgbRed: 22 / 255, green: 12 / 255, blue: 12 / 255, alpha: 1)
    static let font = NSFont.monospacedSystemFont(ofSize: 10.5, weight: .semibold)
    static let pad: CGFloat = 8, tail: CGFloat = 12, edge: CGFloat = 2

    static func attributed(_ s: String) -> NSAttributedString {
        let p = NSMutableParagraphStyle(); p.alignment = .center; p.lineSpacing = 1
        let lines = s.components(separatedBy: "\n")
        let out = NSMutableAttributedString()
        for (i, l) in lines.enumerated() {
            // İlk satır başlık gibi (koyu), sonrakiler biraz daha açık.
            let c = i == 0 ? ink : ink.withAlphaComponent(0.62)
            let f = i == 0 ? font : NSFont.monospacedSystemFont(ofSize: 10, weight: .medium)
            out.append(NSAttributedString(string: l + (i < lines.count - 1 ? "\n" : ""), attributes: [.font: f, .foregroundColor: c, .paragraphStyle: p]))
        }
        return out
    }

    static func size(for s: String) -> CGSize {
        let r = attributed(s).boundingRect(with: CGSize(width: 180, height: 200), options: [.usesLineFragmentOrigin])
        let w = (r.width.rounded(.up) + pad * 2 + edge * 2 + 6)
        let h = (r.height.rounded(.up) + pad * 1.4 + edge * 2)
        return CGSize(width: max(44, (w / 2).rounded(.up) * 2), height: (h / 2).rounded(.up) * 2 + tail + 2)
    }

    override var isFlipped: Bool { false }

    /// Köşeleri basamaklı (kesik) dikdörtgen.
    private func notched(_ r: CGRect, _ s: CGFloat) {
        NSBezierPath(rect: CGRect(x: r.minX + 2 * s, y: r.minY, width: r.width - 4 * s, height: r.height)).fill()
        NSBezierPath(rect: CGRect(x: r.minX + s, y: r.minY + s, width: r.width - 2 * s, height: r.height - 2 * s)).fill()
        NSBezierPath(rect: CGRect(x: r.minX, y: r.minY + 2 * s, width: r.width, height: r.height - 4 * s)).fill()
    }

    override func draw(_ dirty: NSRect) {
        let e = Self.edge
        let body = CGRect(x: 0, y: Self.tail, width: bounds.width - 2, height: bounds.height - Self.tail - 2)
        let shadowR = body.offsetBy(dx: 2, dy: -2)
        let mid = ((min(max(tailX ?? body.midX, 12), body.maxX - 12)) / 2).rounded() * 2
        // Gölge
        Self.ink.withAlphaComponent(0.18).setFill(); notched(shadowR, e)
        // Dış çizgi + kuyruk
        Self.ink.setFill(); notched(body, e)
        // Kuyruk: aşağı doğru daralan basamaklar (12, 8, 4 pt), içi beyaz ve balona açık.
        let t = Self.tail
        for (i, w) in [CGFloat(14), 10, 6].enumerated() {
            NSBezierPath(rect: CGRect(x: mid - w / 2, y: t - CGFloat(i + 1) * 2 * e, width: w, height: 2 * e)).fill()
        }
        // İç
        NSColor.white.setFill(); notched(body.insetBy(dx: e, dy: e), e)
        NSBezierPath(rect: CGRect(x: mid - 5, y: t - 2 * e, width: 10, height: 3 * e)).fill()
        NSBezierPath(rect: CGRect(x: mid - 3, y: t - 4 * e, width: 6, height: 2 * e)).fill()
        NSBezierPath(rect: CGRect(x: mid - 1, y: t - 5 * e, width: 2, height: e)).fill()
        let a = Self.attributed(text)
        let tr = a.boundingRect(with: CGSize(width: 180, height: 200), options: [.usesLineFragmentOrigin])
        let inner = body.insetBy(dx: e + Self.pad, dy: e)
        a.draw(with: CGRect(x: inner.minX, y: inner.midY - tr.height / 2 - 1, width: inner.width, height: tr.height + 2), options: [.usesLineFragmentOrigin])
    }
}

// MARK: - Animasyonlar

/// Kare listesi (sprite adı, süre).
struct CatAnim {
    var frames: [(String, Double)]
    var loop: Bool = true
    var breathe: Double = 0          // nefes periyodu (sn); 0 = yok
    var headTilt: [String: Double] = [:]   // bu karelerde baş öne/aşağı iner (pt, eksi = aşağı)

    static func has(_ n: String) -> Bool { Bundle.main.url(forResource: n, withExtension: "png", subdirectory: "Cat") != nil }

    static var idle: CatAnim {
        if has("idle_0") && has("groomC") {
            // Oturur, göz kırpar, yana bakar; sonra tek patisini kaldırıp üstünü yalar ve oturuşa döner.
            return CatAnim(frames: [
                ("idle_0", 2.6), ("idle_1", 0.16), ("idle_0", 1.2), ("idle_2", 1.4), ("idle_0", 0.8),
                ("groomA", 0.22), ("groomB", 0.25), ("groomC", 0.32), ("groomD", 0.32), ("groomC", 0.32), ("groomD", 0.32),
                ("groomB", 0.25), ("groomA", 0.22), ("idle_0", 2.2), ("idle_1", 0.16), ("idle_0", 1.6),
            ], breathe: 2.2, headTilt: ["groomB": -1.5, "groomC": -3.5, "groomD": -2.5])
        }
        return CatAnim(frames: [("sit", 3.2), ("blink", 0.16), ("sit", 2.6), ("blink", 0.14), ("sit", 4)], breathe: 2.2)
    }
    /// Yalnızca oturup göz kırpar, etrafına bakar (yalama yok).
    static var calm: CatAnim {
        if has("idle_0") { return CatAnim(frames: [("idle_0", 2.4), ("idle_1", 0.16), ("idle_0", 1.8), ("idle_2", 1.6), ("idle_0", 2)], breathe: 2.2) }
        return idle
    }
    static var laptop: CatAnim {
        if has("type_0") && has("type_1c") {
            // Yalnızca ön patiler klavyeye basar; arka bacaklar ve kuyruk sabit (iki kare, arka kısım aynı).
            var f: [(String, Double)] = []
            for _ in 0..<8 { f.append(("type_0", 0.2)); f.append(("type_1c", 0.2)) }
            f.append(("type_0", 1.6))                   // kısa duraklama, ekrana bakar
            for _ in 0..<5 { f.append(("type_0", 0.2)); f.append(("type_1c", 0.2)) }
            return CatAnim(frames: f, breathe: 2.4)
        }
        return CatAnim(frames: [("laptop", 1)], breathe: 2.4)
    }
    static var sleep: CatAnim { CatAnim(frames: [("sleep", 1)], breathe: 3.2) }
    static var dangle: CatAnim { CatAnim(frames: [(has("dangle") ? "dangle" : "sit", 1)]) }
    /// Sevilince: gözleri memnun kısılmış yüz.
    static var content: CatAnim {
        let n = has("content") ? "content" : (has("idle_1") ? "idle_1" : "blink")
        return CatAnim(frames: [(n, 1)], breathe: 1.4)
    }
    static func still(_ n: String, breathe: Double = 2.4) -> CatAnim { CatAnim(frames: [(n, 1)], breathe: breathe) }
}

// MARK: - Kedi

@MainActor
final class CatController: NSObject {
    static let shared = CatController()

    enum Base { case sit, laptop, grumpy, tired, sleep }

    private weak var state: AppState?
    private var sub: AnyCancellable?
    private var catPanel: NSPanel?
    private let catView = CatImageView(frame: .zero)
    /// Kedinin ekrandaki görünür gövdesinin dikdörtgeni (tur çerçevesi için); kutudaysa ya da gizliyse nil.
    var catFrame: CGRect? {
        guard let f = catPanel?.frame, catPanel?.isVisible == true, !hidden else { return nil }
        return CGRect(x: f.midX - 50, y: f.minY, width: 100, height: 116)
    }
    private var bubblePanel: NSPanel?
    private let bubbleView = BubbleView(frame: .zero)
    private var bubbleHide: DispatchWorkItem?
    private var lastSay = Date.distantPast

    private var food: Double = 1, water: Double = 1
    private var prevFood: Double = 1
    private var base: Base = .sit
    private var firstUpdate = true
    private var busyUntil = Date.distantPast       // tek seferlik hareket sürerken temel animasyona dönülmez
    private var idleWork: DispatchWorkItem?
    private var paused = false
    private var hidden = false
    private var dragging = false
    // Esnek sarkma: gövde ortası (çabuk) ve ayak ucu (gecikmeli, salınımlı) için iki yay; kayma pt cinsinden.
    private var midX: CGFloat = 0, midV: CGFloat = 0
    private var tipX: CGFloat = 0, tipV: CGFloat = 0
    private var vxSmooth: CGFloat = 0
    private var cursor: CGPoint = .zero
    private var prevCursor: CGPoint = .zero
    private var lastFrameT: CFTimeInterval = 0
    private var swingLink: CADisplayLink?
    private var lastAI = Date.distantPast           // son Claude/Codex etkinliği
    private var lastPet = Date.distantPast
    private var spriteTop: CGFloat = 108           // şu anki karenin üst kenarı (pt, pencere tabanından)

    private let catSize = CGSize(width: 145, height: 125)
    /// Tutulunca pencere uzar: sarkan kedi oturan kediden büyük görünsün.
    private let dragSize = CGSize(width: 215, height: 172)   // geniş: sallanırken ayaklar kesilmesin
    private let dangleScale: CGFloat = 1.45          // sprite contentsScale (2 = normal; küçüldükçe büyür)
    private let homeKey = "catHomeOrigin"
    /// Ense noktası: tutulunca imlecin altında kalan yer (pencere içinde, sarkan karenin üst ortası).
    private var scruff: CGPoint { CGPoint(x: dragSize.width / 2, y: dragSize.height - 9) }

    // Menü çubuğundaki kutu: Tonti masaüstünde değilse kutuda oturur.
    private var boxItem: NSStatusItem?
    private var inBox: Bool {
        get { UserDefaults.standard.bool(forKey: "catInBox") }
        set { UserDefaults.standard.set(newValue, forKey: "catInBox") }
    }

    // Balon için çalışma takibi
    private var lastProject: String?
    private var workStart: Date?
    private var breakMark = 0
    private var reportClickUntil = Date.distantPast

    func attach(_ s: AppState) {
        guard catPanel == nil else { return }
        state = s
        setupPanel()
        sub = s.$usage.receive(on: DispatchQueue.main).sink { [weak self] u in self?.update(u) }
        let nc = NSWorkspace.shared.notificationCenter
        nc.addObserver(self, selector: #selector(pause), name: NSWorkspace.screensDidSleepNotification, object: nil)
        nc.addObserver(self, selector: #selector(resume), name: NSWorkspace.screensDidWakeNotification, object: nil)
        NotificationCenter.default.addObserver(self, selector: #selector(powerChanged), name: Notification.Name.NSProcessInfoPowerStateDidChange, object: nil)
        if ProcessInfo.processInfo.isLowPowerModeEnabled { pause() }
        installBox()
        if inBox { hidden = true; catPanel?.orderOut(nil) }
        if ProcessInfo.processInfo.environment["ULLAGECAT_DEMO"] != nil { runDemo(); return }
        if let dir = ProcessInfo.processInfo.environment["ULLAGECAT_SNAP"] {
            // Geliştirme: açılır menüyü ve menü çubuğu göstergesini açık/koyu olarak resme çizer.
            DispatchQueue.main.asyncAfter(deadline: .now() + 6) { [weak self] in
                guard let st = self?.state else { return }
                let orig = st.tontiDark
                PopoverView.snapshot = true
                for dark in [false, true] {
                    st.tontiDark = dark
                    for (v, n) in [(AnyView(PopoverView().environmentObject(st)), "popover"),
                                   (AnyView(MenuBarLabel(forceDark: true).environmentObject(st).padding(8).background(Color(white: 0.25))), "menubar")] {
                        let r = ImageRenderer(content: v); r.scale = 2
                        if let img = r.nsImage, let t = img.tiffRepresentation, let png = NSBitmapImageRep(data: t)?.representation(using: .png, properties: [:]) {
                            try? png.write(to: URL(fileURLWithPath: dir).appendingPathComponent("\(n)_\(dark ? "dark" : "light").png"))
                        }
                    }
                }
                PopoverView.snapshot = false
                st.tontiDark = orig
            }
        }
        if let pg = ProcessInfo.processInfo.environment["MEOWLY_PAGE"] {   // geliştirme: pencereyi bu sayfayla aç
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in self?.state?.openDashboard(pg) }
        }
        if let k = ProcessInfo.processInfo.environment["ULLAGECAT_REPORT"].flatMap(ReportKind.init(rawValue:)) {
            ReportNav.pending = (k, Date())
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in self?.state?.openDashboard("reports") }
        }
        if let out = ProcessInfo.processInfo.environment["MEOWLY_GHSVG"] {   // geliştirme: profil kartını dosyaya yazar, göndermez
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) {
                for (split, dark) in [(true, true), (true, false), (false, true), (false, false)] {
                    try? ProfileCard.svg(split: split, dark: dark).write(toFile: out + "/card_\(split ? "split" : "plain")_\(dark ? "dark" : "light").svg", atomically: true, encoding: .utf8)
                }
            }
        }
        if ProcessInfo.processInfo.environment["MEOWLY_TOUR"] != nil {   // geliştirme: turu hemen aç
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in if let st = self?.state { Tour.start(st) } }
        }
        if ProcessInfo.processInfo.environment["ULLAGECAT_DIARY"] != nil {
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in self?.state?.openDashboard("diary") }
        }
        playBase()
        scheduleIdle()
        // İlk açılışta kedinin adı sorulur (bir kez).
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            // Güncelleyen mevcut kullanıcılara tur gösterilmez; yalnızca yeni kurulumda ad isteminden sonra çıkar.
            if CatName.isSet, UserDefaults.standard.object(forKey: Tour.doneKey) == nil { UserDefaults.standard.set(true, forKey: Tour.doneKey) }
            if let st = self?.state { CatNamePrompt.showIfNeeded(st) }
            StarPrompt.markFirstUse()
        }
        // Bir hafta kullanımdan sonra bir kez yıldız rica eder (ad sorusuyla aynı anda çıkmaz).
        DispatchQueue.main.asyncAfter(deadline: .now() + 20) { [weak self] in
            if let st = self?.state { StarPrompt.showIfDue(st) }
        }
    }

    // MARK: Menü çubuğu kutusu

    /// Kutu artık limit göstergesiyle aynı menü öğesinde (MenuBarController). Burada yalnızca durumu tazelenir.
    func installBox() {
        Self.registerDefaults()
        MenuBarController.shared.refresh()
    }

    func refreshBox() { MenuBarController.shared.refresh() }

    var isInBox: Bool { inBox }

    @objc func toggleBox() {
        if inBox { inBox = false; showCat() } else { inBox = true; hideCat() }
        refreshBox()
    }

    /// Kutu simgesi: limit göstergesiyle aynı zeminde (beyaz ya da siyah) küçük bir kutu; Tonti içindeyse başı görünür.
    static func boxIcon(withCat: Bool, dark: Bool, background: Bool = true) -> NSImage {
        let size = NSSize(width: 30, height: 18)
        let head: CGImage? = withCat ? CatSprites.shared.image("idle_0") : nil
        let fill = dark ? NSColor(srgbRed: 0.06, green: 0.06, blue: 0.065, alpha: 1) : .white
        let ink = dark ? NSColor.white : NSColor(srgbRed: 0.086, green: 0.047, blue: 0.047, alpha: 1)
        let img = NSImage(size: size, flipped: false) { _ in
            fill.setFill()
            if background { NSBezierPath(roundedRect: CGRect(origin: .zero, size: size), xRadius: 3, yRadius: 3).fill() }
            let boxRect = CGRect(x: 7, y: 2, width: 16, height: 8)
            if let head, let crop = head.cropping(to: CGRect(x: 0, y: 0, width: head.width, height: Int(Double(head.height) * 0.55))) {
                let h: CGFloat = 12, w = h * CGFloat(crop.width) / CGFloat(crop.height)
                NSGraphicsContext.current?.imageInterpolation = .none
                NSGraphicsContext.current?.cgContext.draw(crop, in: CGRect(x: (size.width - w) / 2, y: 5, width: w, height: h))
            }
            fill.setFill()
            NSBezierPath(rect: boxRect).fill()
            ink.setFill()
            NSBezierPath(rect: CGRect(x: boxRect.minX, y: boxRect.minY, width: boxRect.width, height: 1.5)).fill()
            NSBezierPath(rect: CGRect(x: boxRect.minX, y: boxRect.minY, width: 1.5, height: boxRect.height)).fill()
            NSBezierPath(rect: CGRect(x: boxRect.maxX - 1.5, y: boxRect.minY, width: 1.5, height: boxRect.height)).fill()
            NSBezierPath(rect: CGRect(x: boxRect.minX - 1, y: boxRect.maxY - 1.5, width: boxRect.width + 2, height: 1.5)).fill()
            NSBezierPath(rect: CGRect(x: boxRect.midX - 3, y: boxRect.minY + 3.5, width: 6, height: 1.5)).fill()
            if !withCat {
                NSBezierPath(rect: CGRect(x: boxRect.minX - 3, y: boxRect.maxY - 1, width: 5, height: 1.5)).fill()
                NSBezierPath(rect: CGRect(x: boxRect.maxX - 2, y: boxRect.maxY - 1, width: 5, height: 1.5)).fill()
            }
            return true
        }
        img.isTemplate = false
        return img
    }

    // MARK: Pencere

    private var home: CGPoint {
        get {
            if let a = UserDefaults.standard.array(forKey: homeKey) as? [Double], a.count == 2 { return CGPoint(x: a[0], y: a[1]) }
            let f = NSScreen.main?.visibleFrame ?? CGRect(x: 0, y: 0, width: 1440, height: 900)
            return CGPoint(x: f.maxX - catSize.width - 40, y: f.minY + 4)
        }
        set { UserDefaults.standard.set([Double(newValue.x), Double(newValue.y)], forKey: homeKey) }
    }

    private func setupPanel() {
        let cp = makePanel(size: catSize)
        cp.acceptsMouseMovedEvents = true
        catView.frame = CGRect(origin: .zero, size: catSize)
        cp.contentView = catView
        catView.anchorBottom()
        catView.onClick = { [weak self] in
            guard let self else { return }
            self.hideBubble()
            // "Rapor hazır" balonundan sonraki tıklama dünün raporunu açar.
            if Date() < self.reportClickUntil, let y = Calendar.current.date(byAdding: .day, value: -1, to: Date()) {
                self.reportClickUntil = .distantPast
                ReportNav.pending = (.day, y)
                self.state?.reportsToken += 1
                self.state?.openDashboard("reports")
            } else {
                self.state?.openDashboard("history")
            }
        }
        catView.onStroke = { [weak self] in self?.pet() }
        catView.onDragStart = { [weak self] p in self?.dragStart(p) }
        catView.onDrag = { [weak self] p in self?.dragTo(p) }
        catView.onDragEnd = { [weak self] in self?.dragEnd() }
        catView.onMenu = { [weak self] e in self?.showMenu(e) }
        catPanel = cp
        cp.setFrameOrigin(clamped(home))
        cp.orderFrontRegardless()

        let bp = makePanel(size: CGSize(width: 60, height: 30))
        bp.ignoresMouseEvents = true
        bubbleView.frame = bp.contentLayoutRect
        bubbleView.autoresizingMask = [.width, .height]
        bp.contentView = bubbleView
        bp.alphaValue = 0
        bubblePanel = bp
    }

    /// Pencereyi görünür ekran alanının içinde tutar.
    private func clamped(_ o: CGPoint) -> CGPoint {
        let c = CGPoint(x: o.x + catSize.width / 2, y: o.y + catSize.height / 2)
        let f = (NSScreen.screens.first { $0.frame.contains(c) } ?? NSScreen.main)?.visibleFrame ?? .infinite
        guard f != .infinite else { return o }
        return CGPoint(x: min(max(o.x, f.minX - 30), f.maxX - catSize.width + 30), y: min(max(o.y, f.minY), f.maxY - catSize.height))
    }

    // MARK: Konuşma balonu

    /// Kedinin başının üstünde kısa bir balon gösterir. Sık konuşmasın diye `force` olmadan en az 20 sn arayla.
    private func say(_ text: String, seconds: Double = 4.5, force: Bool = false) {
        guard !hidden, !paused, !dragging, let cp = catPanel, let bp = bubblePanel else { return }
        guard force || Date().timeIntervalSince(lastSay) > 20 else { return }
        lastSay = Date()
        bubbleView.text = text
        let s = BubbleView.size(for: text)
        var x = (cp.frame.midX - s.width / 2).rounded()
        if let vf = (cp.screen ?? NSScreen.main)?.visibleFrame { x = min(max(x, vf.minX + 4), vf.maxX - s.width - 4) }
        bubbleView.tailX = cp.frame.midX - x
        let y = (cp.frame.minY + spriteTop + 2).rounded()
        bp.setFrame(CGRect(x: x, y: y, width: s.width, height: s.height), display: true)
        if bp.parent == nil { cp.addChildWindow(bp, ordered: .above) }
        bp.orderFront(nil)
        NSAnimationContext.runAnimationGroup { c in c.duration = 0.15; bp.animator().alphaValue = 1 }
        bubbleHide?.cancel()
        let w = DispatchWorkItem { [weak self] in self?.hideBubble() }
        bubbleHide = w
        DispatchQueue.main.asyncAfter(deadline: .now() + seconds, execute: w)
    }

    private func hideBubble() {
        bubbleHide?.cancel()
        guard let bp = bubblePanel, bp.alphaValue > 0 else { return }
        NSAnimationContext.runAnimationGroup({ c in c.duration = 0.2; bp.animator().alphaValue = 0 },
                                             completionHandler: { [weak self] in
            MainActor.assumeIsolated {
                guard let self, let bp = self.bubblePanel, bp.alphaValue == 0 else { return }
                self.catPanel?.removeChildWindow(bp); bp.orderOut(nil)
            }
        })
    }

    // MARK: Tutma: enseden sarkar, ense imlecin tam altında kalır; harekete göre sallanır, bırakınca oturur.

    private func dragStart(_ p: CGPoint) {
        dragging = true
        idleWork?.cancel(); hideBubble()
        midX = 0; midV = 0; tipX = 0; tipV = 0; vxSmooth = 0; lastFrameT = 0
        cursor = p; prevCursor = p
        catView.split(false)
        catPanel?.setFrame(CGRect(origin: catPanel?.frame.origin ?? .zero, size: dragSize), display: false)
        catView.anchorTop()
        play(.dangle)
        if let img = catView.sprite.contents { catView.startBend(img as! CGImage, scale: dangleScale) }
        dragTo(p)
        // Sallanma ekran yenilemesiyle (60/120 Hz) hesaplanır; fare olaylarının düzensiz aralığı titreme yapmaz. Yalnızca tutarken çalışır.
        swingLink?.invalidate()
        swingLink = catView.displayLink(target: self, selector: #selector(swingFrame(_:)))
        swingLink?.add(to: .main, forMode: .common)
    }

    private func dragTo(_ p: CGPoint) {
        guard let cp = catPanel else { return }
        cursor = p
        cp.setFrameOrigin(CGPoint(x: (p.x - scruff.x).rounded(), y: (p.y - scruff.y).rounded()))
    }

    /// Sarkaç gibi: imlecin yumuşatılmış hızına göre hedef açı, yay-sönüm ile oraya gider; imleç durunca hafifçe salınıp diker.
    @objc private func swingFrame(_ l: CADisplayLink) {
        guard dragging else { return }
        let now = l.timestamp
        let dt = CGFloat(lastFrameT == 0 ? 1.0 / 60 : min(1.0 / 30, max(1.0 / 240, now - lastFrameT)))
        lastFrameT = now
        let vx = (cursor.x - prevCursor.x) / dt
        prevCursor = cursor
        vxSmooth += (vx - vxSmooth) * min(1, dt * 10)
        // Hareketin tersine savrulur: gövde ortası çabuk ve az, ayak ucu geç ve çok; durunca ayaklar birkaç kez yumuşakça salınır.
        let target = max(-24, min(24, -vxSmooth * 0.03))
        midV += (200 * (target * 0.35 - midX) - 20 * midV) * dt
        midX += midV * dt
        tipV += (55 * (target - tipX) - 4.5 * tipV) * dt
        tipX += tipV * dt
        let m = midX, t = tipX
        catView.bend { s in
            // Ense sabit; baş bölgesi (s<0.3) neredeyse kıpırdamaz, kalçadan aşağısı ayak ucuna doğru giderek savrulur.
            m * pow(s, 1.6) + (t - m) * pow(s, 3.2)
        }
    }

    private func dragEnd() {
        dragging = false
        swingLink?.invalidate(); swingLink = nil
        catView.endBend()
        guard let cp = catPanel else { return }
        let o = clamped(CGPoint(x: cp.frame.minX + (dragSize.width - catSize.width) / 2, y: cp.frame.minY))
        cp.setFrame(CGRect(origin: o, size: catSize), display: false)
        home = o
        // Bırakınca hemen oturur; yere konar gibi kısa bir esneme (zıplama yok).
        catView.anchorBottom()
        playBase()
        let land = CASpringAnimation(keyPath: "transform.scale.y")
        land.fromValue = 0.94; land.toValue = 1
        land.damping = 14; land.stiffness = 260
        land.duration = land.settlingDuration
        catView.sprite.add(land, forKey: "land")
        scheduleIdle()
    }

    private func showMenu(_ e: NSEvent) {
        let m = NSMenu()
        let s = NSMenuItem(title: L.t("Ayarlar"), action: #selector(openSettings), keyEquivalent: ""); s.target = self; m.addItem(s)
        let h = NSMenuItem(title: L.t("Tonti'yi kutuya koy"), action: #selector(toggleBox), keyEquivalent: ""); h.target = self; m.addItem(h)
        NSMenu.popUpContextMenu(m, with: e, for: catView)
    }

    @objc private func openSettings() { state?.openDashboard("appearance") }
    @objc func hideCat() { hideBubble(); hidden = true; idleWork?.cancel(); catPanel?.orderOut(nil) }
    func showCat() { hidden = false; catPanel?.orderFrontRegardless(); playBase(); scheduleIdle() }

    // MARK: Güç: ekran uykuda ya da düşük güç modunda animasyonlar tamamen durur.

    @objc private func pause() { paused = true; hideBubble(); idleWork?.cancel(); catView.sprite.removeAllAnimations(); catView.head.removeAllAnimations() }
    @objc private func resume() { guard !ProcessInfo.processInfo.isLowPowerModeEnabled else { return }; paused = false; playBase(); scheduleIdle() }
    @objc private func powerChanged() { DispatchQueue.main.async { ProcessInfo.processInfo.isLowPowerModeEnabled ? self.pause() : self.resume() } }

    // MARK: Oynatma

    private func play(_ a: CatAnim) {
        guard !hidden else { return }
        let imgs = a.frames.compactMap { CatSprites.shared.image($0.0) }
        guard let first = imgs.first else { return }
        if !dragging { spriteTop = CGFloat(imgs.map(\.height).max() ?? first.height) / 2 }
        let sp = catView.sprite
        sp.removeAnimation(forKey: "frames"); sp.removeAnimation(forKey: "breathe")
        sp.contentsScale = dragging ? dangleScale : 2
        let useHead = !a.headTilt.isEmpty && !dragging
        catView.split(useHead, imageH: CGFloat(first.height) / 2)
        CATransaction.begin(); CATransaction.setDisableActions(true)
        sp.contents = first
        if useHead { catView.head.contents = first }
        CATransaction.commit()
        guard !paused else { return }
        let total = a.frames.reduce(0) { $0 + $1.1 }
        var times: [NSNumber] = []
        var t = 0.0
        for f in a.frames { times.append(NSNumber(value: t / total)); t += f.1 }
        if imgs.count > 1 {
            let k = CAKeyframeAnimation(keyPath: "contents")
            k.values = imgs; k.keyTimes = times; k.calculationMode = .discrete
            k.duration = total; k.repeatCount = a.loop ? .infinity : 1
            k.isRemovedOnCompletion = false; k.fillMode = .forwards
            sp.add(k, forKey: "frames")
            if useHead { catView.head.add(k, forKey: "frames") }
        }
        if useHead {
            // Yalarken baş hafifçe öne eğilir gibi aşağı iner, sonra yumuşakça kalkar.
            let r = CAKeyframeAnimation(keyPath: "transform.translation.y")
            var vals = a.frames.map { a.headTilt[$0.0] ?? 0 }
            vals.append(vals.first ?? 0)
            r.values = vals; r.keyTimes = times + [1]
            r.calculationMode = .cubic; r.duration = total; r.repeatCount = a.loop ? .infinity : 1
            catView.head.add(r, forKey: "dip")
        }
        if a.breathe > 0 && !dragging {
            let b = CABasicAnimation(keyPath: "transform.scale.y")
            b.fromValue = 1.0; b.toValue = 1.022; b.duration = a.breathe / 2
            b.autoreverses = true; b.repeatCount = .infinity
            b.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
            sp.add(b, forKey: "breathe")
            if useHead {
                // Gövde nefesle uzarken baş da aynı miktar yükselir, arada boşluk olmaz.
                let neck = catView.head.position.y
                let hb = CABasicAnimation(keyPath: "position.y")
                hb.fromValue = neck; hb.toValue = neck * 1.022; hb.duration = a.breathe / 2
                hb.autoreverses = true; hb.repeatCount = .infinity
                hb.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
                catView.head.add(hb, forKey: "breathe")
            }
        }
    }

    private func playBase() {
        guard !dragging else { return }
        switch base {
        case .sit: play(.idle)
        case .laptop: play(.laptop)
        case .grumpy: play(.still("grumpy"))
        case .tired: play(.still("tired", breathe: 3))
        case .sleep: play(.sleep)
        }
    }

    /// Tek seferlik hareket (esneme, sevilme...), bitince temel animasyona döner.
    private func oneShot(_ a: CatAnim, seconds: Double) {
        guard !dragging else { return }
        busyUntil = Date().addingTimeInterval(seconds)
        play(a)
        DispatchQueue.main.asyncAfter(deadline: .now() + seconds) { [weak self] in
            guard let self, !self.dragging, Date() >= self.busyUntil else { return }
            self.playBase()
        }
    }

    /// Sevilince: uyuyorsa esner; değilse gözlerini memnun kısar ve başının üstünden küçük bir kalp yükselir.
    private func pet() {
        guard !dragging, Date().timeIntervalSince(lastPet) > 2.5 else { return }
        lastPet = Date()
        if base == .sleep { oneShot(.still("yawn", breathe: 0), seconds: 1.4); return }
        oneShot(.content, seconds: 2.2)
        popHeart()
        if Double.random(in: 0..<1) < 0.35 { say(.chat, L.t("Mırr…"), seconds: 2) }
    }

    private static let heartImage: CGImage? = {
        let rows = [".##.##.", "#######", "#######", ".#####.", "..###..", "...#..."]
        let s = 4, w = 7 * s, h = 6 * s
        guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        ctx.setFillColor(CGColor(srgbRed: 0.96, green: 0.45, blue: 0.58, alpha: 1))
        for (y, r) in rows.enumerated() { for (x, c) in r.enumerated() where c == "#" { ctx.fill(CGRect(x: x * s, y: (5 - y) * s, width: s, height: s)) } }
        return ctx.makeImage()
    }()

    private static let zImage: CGImage? = {
        let rows = ["#####", "...#.", "..#..", ".#...", "#####"]
        let s = 4, w = 5 * s, h = 5 * s
        guard let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                                  space: CGColorSpace(name: CGColorSpace.sRGB)!, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return nil }
        ctx.setFillColor(CGColor(srgbRed: 0.55, green: 0.6, blue: 0.75, alpha: 1))
        for (y, r) in rows.enumerated() { for (x, c) in r.enumerated() where c == "#" { ctx.fill(CGRect(x: x * s, y: (4 - y) * s, width: s, height: s)) } }
        return ctx.makeImage()
    }()

    /// Uyurken başının üstünden küçük bir "z" süzülür (yalnızca boşta hareket zamanlayıcısıyla, sürekli değil).
    private func popZ() {
        guard let root = catView.layer, let img = Self.zImage else { return }
        for i in 0..<2 {
            let z = CALayer()
            z.contents = img; z.contentsScale = 2; z.magnificationFilter = .nearest
            let side: CGFloat = i == 0 ? 10 : 7
            z.bounds = CGRect(x: 0, y: 0, width: side, height: side)
            z.position = CGPoint(x: catView.bounds.midX + 22 + CGFloat(i) * 8, y: spriteTop + 2)
            z.opacity = 0
            root.addSublayer(z)
            let up = CABasicAnimation(keyPath: "position.y"); up.byValue = 18
            let fade = CAKeyframeAnimation(keyPath: "opacity"); fade.values = [0, 1, 1, 0]; fade.keyTimes = [0, 0.2, 0.7, 1]
            let g = CAAnimationGroup(); g.animations = [up, fade]; g.duration = 1.8
            g.beginTime = CACurrentMediaTime() + Double(i) * 0.6
            g.isRemovedOnCompletion = false; g.fillMode = .forwards
            z.add(g, forKey: "z")
            DispatchQueue.main.asyncAfter(deadline: .now() + 2.6) { z.removeFromSuperlayer() }
        }
    }

    private func popHeart() {
        guard let root = catView.layer, let img = Self.heartImage else { return }
        let h = CALayer()
        h.contents = img; h.contentsScale = 2; h.magnificationFilter = .nearest
        h.bounds = CGRect(x: 0, y: 0, width: 14, height: 12)
        let topY = min(catView.bounds.height - 8, spriteTop)
        h.position = CGPoint(x: catView.bounds.midX + CGFloat.random(in: -10...10), y: topY - 10)
        root.addSublayer(h)
        let up = CABasicAnimation(keyPath: "position.y"); up.byValue = 22
        let fade = CABasicAnimation(keyPath: "opacity"); fade.fromValue = 1; fade.toValue = 0
        let g = CAAnimationGroup(); g.animations = [up, fade]; g.duration = 1.1
        g.timingFunction = CAMediaTimingFunction(name: .easeOut)
        g.isRemovedOnCompletion = false; g.fillMode = .forwards
        h.add(g, forKey: "pop")
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.15) { h.removeFromSuperlayer() }
    }

    // MARK: Durum

    private static let clock: DateFormatter = { let f = DateFormatter(); f.dateFormat = "HH:mm"; return f }()

    private var noteText: String {
        var parts: [String] = []
        if let p = HistoryStore.shared.latestProject() { parts.append(p) }
        if let ws = workStart, Date().timeIntervalSince(ws) >= 60 { parts.append(Self.duration(Date().timeIntervalSince(ws))) }
        return parts.isEmpty ? L.t("✎ Not alıyorum…") : L.t("✎ Not alıyorum…") + "\n" + parts.joined(separator: " · ")
    }

    private func update(_ usage: [Provider: Usage]) {
        let u = usage[.claude].flatMap { $0.limitConfidence == .none ? nil : $0 } ?? usage[.codex]
        food = u?.fiveRow?.left ?? 1
        water = u?.weekRow?.left ?? 1
        let refilled = !firstUpdate && prevFood <= 0.2 && food >= 0.9
        prevFood = food

        lastAI = max(HistoryStore.shared.latestEvent(provider: "claude"), HistoryStore.shared.latestEvent(provider: "codex"))
        let changed = applyBase()
        catView.toolTip = CatName.current
        if refilled { oneShot(.content, seconds: 2.5); popHeart() }
        syncAIWork()
        workTick()
        if firstUpdate {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in self?.dailyNote() }
        }
        if u != nil { firstUpdate = false }
    }

    /// Kullanıcının klavye/fareye en son dokunmasından beri geçen süre (sn). Tek bir sistem çağrısı, yoklama yok.
    private var userIdle: TimeInterval {
        if let v = ProcessInfo.processInfo.environment["MEOWLY_USER_IDLE"].flatMap(Double.init) { return v }   // geliştirme
        return CGEventSource.secondsSinceLastEventType(.combinedSessionState, eventType: CGEventType(rawValue: ~0)!)
    }

    /// Klavyeye en son basılmasından beri geçen süre (sn). İzin gerekmez; yalnızca süre okunur, tuşlar okunmaz.
    private var keyIdle: TimeInterval {
        if let v = ProcessInfo.processInfo.environment["MEOWLY_KEY_IDLE"].flatMap(Double.init) { return v }   // geliştirme
        return CGEventSource.secondsSinceLastEventType(.combinedSessionState, eventType: .keyDown)
    }

    /// Video oynuyor ya da görüntülü görüşme varsa ekran uyanık tutulur: kullanıcı başındadır, boşta sayılmaz.
    private var mediaActive: Bool {
        if ProcessInfo.processInfo.environment["MEOWLY_MEDIA"] != nil { return true }   // geliştirme
        var dict: Unmanaged<CFDictionary>?
        guard IOPMCopyAssertionsStatus(&dict) == kIOReturnSuccess, let d = dict?.takeRetainedValue() as? [String: Int] else { return false }
        return (d["PreventUserIdleDisplaySleep"] ?? 0) > 0
    }

    /// Yapay zekâ şu an çalışıyor mu (son 2 dk'da Claude/Codex olayı ve kullanıcı 10 dk içinde başındaydı).
    /// Yalnızca bu durumda "not alıyorum / oturumu kaydettim" balonları çıkar.
    private var aiWorking: Bool {
        let ai = ProcessInfo.processInfo.environment["MEOWLY_AI_IDLE"].flatMap(Double.init) ?? Date().timeIntervalSince(lastAI)   // geliştirme
        return ai < 120 && userIdle < 10 * 60
    }

    /// Temel duruş (öncelik sırasıyla):
    /// 1. yapay zekâ çalışıyor → laptopta yazar (limit tamamen bittiyse yorgun)
    /// 2. kullanıcı 3 dk klavye/fareye dokunmadı ve video/görüşme yok → uyur
    /// 3. kullanıcı herhangi bir uygulamada yazıyor (son 90 sn'de tuş) → laptopta yazar (Word, Pages, Notion…)
    /// 4. limit bitti → yorgun; limit %15 altı → huysuz
    /// 5. aksi halde (başında ama yazmıyor: okuyor, fareyle geziyor, izliyor) → oturur, yalanır, etrafa bakar
    private func computeBase() -> Base {
        let env = ProcessInfo.processInfo.environment["ULLAGECAT_BASE"]
        if let env, let b = ["sit": Base.sit, "laptop": .laptop, "sleep": .sleep, "tired": .tired, "grumpy": .grumpy][env] { return b }
        let empty = food <= 0.001 || water <= 0.001
        if aiWorking { return empty ? .tired : .laptop }
        if userIdle > 3 * 60 && !mediaActive { return .sleep }
        if keyIdle < 90 { return .laptop }
        if empty { return .tired }
        if food < 0.15 || water < 0.15 { return .grumpy }
        return .sit
    }

    /// Duruşu günceller; uykuya dalarken ve uyanırken önce esner. Değiştiyse true.
    @discardableResult
    private func applyBase() -> Bool {
        let nb = computeBase()
        guard nb != base else { return false }
        let wasSleeping = base == .sleep
        base = nb
        if dragging { return true }
        if wasSleeping != (nb == .sleep) { oneShot(.still("yawn", breathe: 0), seconds: 1.6) }
        else if Date() >= busyUntil { playBase() }
        return true
    }

    /// Tanıtım: tüm balon türlerini örnek metinlerle sırayla gösterir (ULLAGECAT_DEMO=1).
    private var demo = false
    private func runDemo() {
        demo = true
        busyUntil = .distantFuture
        let d = Self.duration
        let steps: [(CatAnim, String, Bool)] = [
            (.laptop, L.t("✎ Not alıyorum…") + "\nullage-cat · " + d(45 * 60), false),
            (.laptop, L.t("✎ Yeni sayfa açtım") + "\nai_limit", false),
            (.laptop, d(90 * 60) + L.t(" oldu") + "\n" + L.t("Biraz esneyelim mi?"), false),
            (.laptop, L.f("Bugün %@ token!", formatTokens(1_000_000)) + "\n" + L.t("Deftere yazdım"), false),
            (.idle, L.t("Oturumu kaydettim") + "\nullage-cat · " + d(80 * 60), false),
            (.content, L.t("Mırr…"), true),
            (.idle, L.t("Dün") + ": " + d(190 * 60) + L.f(" · %lld proje", 2) + "\n" + L.t("Rapor hazır, bana tıkla"), false),
        ]
        for (i, s) in steps.enumerated() {
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.5 + Double(i) * 5) { [weak self] in
                guard let self else { return }
                self.play(s.0)
                if s.2 { self.popHeart() }
                self.say(s.1, seconds: 4.2, force: true)
            }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5 + Double(steps.count) * 5) { [weak self] in
            guard let self else { return }
            self.demo = false; self.busyUntil = .distantPast; self.playBase(); self.scheduleIdle()
        }
    }

    // MARK: Balon içerikleri (limit yok; o an ne yaptığı ve çalışma notları)

    enum Topic: String { case work = "catBubbleWork", rest = "catBubbleBreak", daily = "catBubbleDaily", record = "catBubbleRecord", chat = "catBubbleChat" }

    static func registerDefaults() {
        UserDefaults.standard.register(defaults: ["catBubbles": true, Topic.work.rawValue: true, Topic.rest.rawValue: true,
                                                  Topic.daily.rawValue: true, Topic.record.rawValue: true, Topic.chat.rawValue: true])
    }

    private func allowed(_ t: Topic) -> Bool {
        UserDefaults.standard.bool(forKey: "catBubbles") && UserDefaults.standard.bool(forKey: t.rawValue)
    }

    private func say(_ t: Topic, _ text: String, seconds: Double = 5, force: Bool = false) {
        guard allowed(t), !demo else { return }
        say(text, seconds: seconds, force: force)
    }

    nonisolated static func duration(_ s: TimeInterval) -> String {
        let m = max(1, Int(s / 60)), h = m / 60
        if h == 0 { return L.f("%lld dk", m) }
        return m % 60 == 0 ? L.f("%lld sa", h) : L.f("%lld sa %lld dk", h, m % 60)
    }

    /// Şu anki çalışma oturumunun başlangıcı (Günlük ile aynı oturum hesabı).
    private func sessionStart(now: Date = Date()) -> Date? {
        HistoryStore.shared.sessions(from: now.addingTimeInterval(-12 * 3600))
            .filter { now.timeIntervalSince($0.end) <= HistoryStore.sessionGap }
            .max { $0.end < $1.end }?.start
    }

    /// Yapay zekâ oturumu başladı/bitti mi; balonlar yalnızca bu değişimde çıkar (Word'de yazmak oturum sayılmaz).
    private var aiWasWorking = false
    private func syncAIWork() {
        let now = aiWorking && base != .sleep
        guard now != aiWasWorking else { return }
        aiWasWorking = now
        workChanged(active: now)
    }

    private func workChanged(active: Bool) {
        if active {
            workStart = sessionStart() ?? Date()
            lastProject = HistoryStore.shared.latestProject()
            breakMark = Int(Date().timeIntervalSince(workStart!) / (90 * 60))
            say(.work, noteText, force: true)
        } else if let ws = workStart {
            // Çalışma bitti: oturumu "kaydettim" der.
            let end = HistoryStore.shared.activity(from: ws).last.map { Date(timeIntervalSince1970: $0.ts) } ?? Date()
            if end.timeIntervalSince(ws) >= 5 * 60 {
                let p = lastProject.map { $0 + " · " } ?? ""
                say(.work, L.t("Oturumu kaydettim") + "\n" + p + Self.duration(end.timeIntervalSince(ws)), seconds: 6, force: true)
            }
            workStart = nil
        }
    }

    /// Her yenilemede (yaklaşık 30 sn): proje değişimi, uzun oturumda mola, günün token eşikleri.
    private func workTick() {
        guard aiWasWorking, let ws = workStart else { return }
        let p = HistoryStore.shared.latestProject()
        if let p, p != lastProject {
            lastProject = p
            say(.work, L.t("✎ Yeni sayfa açtım") + "\n" + p, force: true)
            return
        }
        let mark = Int(Date().timeIntervalSince(ws) / (90 * 60))
        if mark > breakMark {
            breakMark = mark
            say(.rest, Self.duration(Date().timeIntervalSince(ws)) + L.t(" oldu") + "\n" + L.t("Biraz esneyelim mi?"), seconds: 7, force: true)
            return
        }
        recordNote()
    }

    private func recordNote() {
        let day = Calendar.current.startOfDay(for: Date())
        let ud = UserDefaults.standard
        let dayKey = String(Int(day.timeIntervalSince1970))
        let steps = [250_000, 500_000, 1_000_000, 2_000_000, 5_000_000, 10_000_000, 20_000_000, 50_000_000, 100_000_000]
        let tokens = HistoryStore.shared.totalTokens(from: day)
        let reached = steps.last { tokens >= $0 } ?? 0
        let prev = ud.string(forKey: "catRecordDay") == dayKey ? ud.integer(forKey: "catRecord") : 0
        guard reached > prev else { return }
        ud.set(dayKey, forKey: "catRecordDay"); ud.set(reached, forKey: "catRecord")
        say(.record, L.f("Bugün %@ token!", formatTokens(reached)) + "\n" + L.t("Deftere yazdım"), force: false)
    }

    /// Günün ilk açılışında dünün kısa özeti.
    private func dailyNote() {
        let ud = UserDefaults.standard
        let today = Calendar.current.startOfDay(for: Date())
        let key = String(Int(today.timeIntervalSince1970))
        guard ud.string(forKey: "catDailyDay") != key else { return }
        ud.set(key, forKey: "catDailyDay")
        let y = today.addingTimeInterval(-86400)
        let ss = HistoryStore.shared.sessions(from: y, to: today)
        let active = ss.reduce(0) { $0 + $1.duration }
        let projects = Set(ss.map(\.project).filter { !$0.isEmpty }).count
        guard active >= 5 * 60 else { return }
        let line = Self.duration(active) + (projects > 0 ? L.f(" · %lld proje", projects) : "")
        say(.daily, L.t("Dün") + ": " + line + "\n" + L.t("Rapor hazır, bana tıkla"), seconds: 8, force: true)
        if allowed(.daily) { reportClickUntil = Date().addingTimeInterval(10 * 60) }
    }

    // MARK: Kendi kendine hareketler

    private func scheduleIdle() {
        idleWork?.cancel()
        guard !paused, !hidden else { return }
        let w = DispatchWorkItem { [weak self] in self?.idleTick() }
        idleWork = w
        DispatchQueue.main.asyncAfter(deadline: .now() + Double.random(in: 8...16), execute: w)
    }

    private func idleTick() {
        defer { scheduleIdle() }
        guard !dragging, !demo else { return }
        // Kullanıcı bilgisayardan kalkınca/dönünce yenilemeyi beklemeden uyur/uyanır.
        let changed = applyBase()
        syncAIWork()
        if changed { return }
        guard Date() >= busyUntil else { return }
        let r = Double.random(in: 0..<1)
        switch base {
        case .sit:
            switch r {
            // Kendiliğinden girilen dikey "göbek" pozu kaldırıldı (kullanıcı istemedi); yalnızca esner ve etrafına bakar.
            case ..<0.25: oneShot(.still("yawn", breathe: 0), seconds: 1.6)
            case ..<0.45: oneShot(.calm, seconds: 8)
            default: break
            }
        case .laptop:
            // Çalışırken arada mola verir: oturup etrafına bakar ve patisini yalar ya da esner. Arada ne yaptığını söyler.
            switch r {
            case ..<0.35: oneShot(.idle, seconds: Double.random(in: 10...16))
            case ..<0.50: oneShot(.still("yawn", breathe: 0), seconds: 1.5)
            case ..<0.58: if aiWasWorking { say(.work, noteText) }
            default: break
            }
        case .sleep:
            popZ()
        case .tired, .grumpy:
            // Huysuz/yorgunken de donup kalmaz: arada esner ya da etrafına bakar.
            switch r {
            case ..<0.3: oneShot(.still("yawn", breathe: 0), seconds: 1.6)
            case ..<0.5: oneShot(.calm, seconds: 6)
            default: break
            }
        }
    }
}
