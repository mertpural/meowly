import AppKit
import SwiftUI
import Combine

/// Menü çubuğundaki tek öğe: solda limit göstergesi, sağda Tonti'nin kutusu; arada boşluk yok.
/// Limit kısmına tıklayınca menü açılır, kutuya tıklayınca Tonti kutuya girer ya da masaüstüne çıkar.
@MainActor
final class MenuBarController: NSObject, NSPopoverDelegate {
    static let shared = MenuBarController()

    private var item: NSStatusItem?
    private weak var state: AppState?
    private var sub: AnyCancellable?
    private var popover: NSPopover?
    private var boxWidth: CGFloat = 30
    /// Son çizilen menü çubuğu resmi (ayarlardaki önizleme için).
    private(set) var image: NSImage?
    /// Menü çubuğundaki öğenin ekran dikdörtgeni (tur çerçevesi için).
    var buttonFrame: CGRect? { item?.button?.window?.frame }

    func install(_ s: AppState) {
        guard item == nil else { return }
        state = s
        CatController.registerDefaults()
        let it = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        it.autosaveName = "MeowlyItem"
        // macOS görünürlüğü sıraya göre adlandırılmış ayarlarda tutar (Item-0, Item-1). Uygulamadaki görünmez
        // SwiftUI öğesi ile açılış sırası değişince bu öğe onun "gizli" ayarını devralıp kayboluyordu: hep görünür olsun.
        it.isVisible = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in
            if self?.item?.isVisible == false { self?.item?.isVisible = true }
        }
        it.button?.target = self
        it.button?.action = #selector(clicked)
        it.button?.sendAction(on: [.leftMouseUp, .rightMouseUp])
        item = it
        // Durum değişince (yüzdeler, görünüm, ayarlar) resim yeniden çizilir; art arda değişimler birleştirilir.
        sub = s.objectWillChange
            .debounce(for: .milliseconds(250), scheduler: DispatchQueue.main)
            .sink { [weak self] _ in self?.refresh() }
        refresh()
        if ProcessInfo.processInfo.environment["MEOWLY_POPOVER"] != nil {   // geliştirme: bellek testleri için menüyü açık başlat
            DispatchQueue.main.asyncAfter(deadline: .now() + 3) { [weak self] in if let b = self?.item?.button { self?.togglePopover(b) } }
        }
    }

    private var pendingRefresh = false

    func refresh() {
        // Açılır menü açıkken resim değiştirilmez: menü her seferinde yeniden konumlanıp bellek biriktiriyordu.
        if popover?.isShown == true { pendingRefresh = true; return }
        guard let b = item?.button, let st = state else { return }
        let dark = st.tontiDark
        let limits = MenuBarLabel.render(MenuBarLabel.chip(state: st))
        let box = CatController.boxIcon(withCat: CatController.shared.isInBox, dark: dark, background: false)
        let lw = limits?.size.width ?? 0
        let h: CGFloat = 18
        boxWidth = box.size.width
        let size = NSSize(width: lw + boxWidth, height: h)
        let fill = dark ? NSColor(srgbRed: 0.06, green: 0.06, blue: 0.065, alpha: 1) : .white
        let img = NSImage(size: size, flipped: false) { _ in
            fill.setFill()
            NSBezierPath(roundedRect: CGRect(origin: .zero, size: size), xRadius: 3, yRadius: 3).fill()
            if let l = limits { l.draw(in: CGRect(x: 0, y: (h - l.size.height) / 2, width: l.size.width, height: l.size.height)) }
            box.draw(in: CGRect(x: lw, y: 0, width: box.size.width, height: h))
            return true
        }
        img.isTemplate = false
        image = img
        b.image = img
        b.toolTip = CatController.shared.isInBox ? L.t("Tonti'yi masaüstüne çıkar") : nil
    }

    @objc private func clicked(_ sender: NSStatusBarButton) {
        // Tıklamanın düğmedeki yeri: ekran konumundan hesaplanır (olayın penceresine güvenilmez).
        let mouse = NSEvent.mouseLocation
        let frame = sender.window.map { $0.convertToScreen(sender.convert(sender.bounds, to: nil)) } ?? .zero
        let p = CGPoint(x: mouse.x - frame.minX, y: mouse.y - frame.minY)
        // Resim düğmenin ortasında durur; kutu resmin sağ ucundadır.
        let imgW = sender.image?.size.width ?? sender.bounds.width
        let imgRight = (sender.bounds.width + imgW) / 2
        if p.x >= imgRight - boxWidth {
            CatController.shared.toggleBox()
        } else {
            togglePopover(sender)
        }
    }

    private func togglePopover(_ b: NSStatusBarButton) {
        if let p = popover, p.isShown { p.performClose(nil); return }
        guard let st = state else { return }
        PopoverView.inPopover = true
        let host = NSHostingController(rootView: PopoverView().environmentObject(st))
        host.sizingOptions = [.preferredContentSize]
        let p = NSPopover()
        p.behavior = .transient
        p.animates = false
        p.contentViewController = host
        p.delegate = self
        p.appearance = NSAppearance(named: st.tontiDark ? .darkAqua : .aqua)
        popover = p
        p.show(relativeTo: b.bounds, of: b, preferredEdge: .minY)
        p.contentViewController?.view.window?.makeKey()
    }

    func closePopover() { popover?.performClose(nil) }

    // Kapanınca içerik bırakılır: menü kapalıyken bellekte SwiftUI ağacı tutulmaz.
    nonisolated func popoverDidClose(_ notification: Notification) {
        MainActor.assumeIsolated {
            popover?.contentViewController = nil
            popover = nil
            if pendingRefresh { pendingRefresh = false; refresh() }
            DispatchQueue.main.asyncAfter(deadline: .now() + 2) { malloc_zone_pressure_relief(nil, 0) }
        }
    }
}
