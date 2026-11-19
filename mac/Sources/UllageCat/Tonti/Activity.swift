import AppKit
import ApplicationServices
import CoreServices
import EventKit
import SQLite3
import IOKit.pwr_mgt

// MARK: - Katmanlar

/// Bilgisayar günlüğünün katmanları. Hepsi varsayılan açık; her biri ayarlardan kapatılır.
/// İzin gerektirenler, izin verilmeden sessizce bekler (izin penceresi kendiliğinden açılmaz).
enum LogLayer: String, CaseIterable, Identifiable {
    case apps = "logApps", titles = "logTitles", sites = "logSites", files = "logFiles", calendar = "logCalendar"
    var id: String { rawValue }
    var on: Bool { UserDefaults.standard.object(forKey: rawValue) as? Bool ?? true }
    func set(_ v: Bool) { UserDefaults.standard.set(v, forKey: rawValue) }

    enum Access { case ok, needed, notNeeded }
    /// İzin durumu; soru sormadan okunur.
    var access: Access {
        switch self {
        case .apps: return .notNeeded
        case .files: return .notNeeded
        case .titles: return AXIsProcessTrusted() ? .ok : .needed
        case .sites: return Browser.anyAllowed ? .ok : (AXIsProcessTrusted() ? .ok : .needed)
        case .calendar: return EKEventStore.authorizationStatus(for: .event) == .fullAccess ? .ok : .needed
        }
    }
}

// MARK: - Kayıtlar (aynı veritabanında)

struct AppSpan {
    var start: Date, end: Date
    var bundle: String, app: String
    var title: String?, domain: String?
    var category: String
    var duration: TimeInterval { end.timeIntervalSince(start) }
}

extension HistoryStore {
    func setupActivityTables() {
        lock.lock(); defer { lock.unlock() }
        sqlite3_exec(db, """
        CREATE TABLE IF NOT EXISTS activity(id INTEGER PRIMARY KEY, start REAL NOT NULL, end REAL NOT NULL,
          bundle TEXT, app TEXT, title TEXT, domain TEXT, category TEXT);
        CREATE INDEX IF NOT EXISTS idx_activity_start ON activity(start);
        CREATE TABLE IF NOT EXISTS file_activity(hour REAL NOT NULL, project TEXT NOT NULL, count INTEGER NOT NULL,
          PRIMARY KEY(hour, project));
        """, nil, nil, nil)
    }

    func insertSpan(_ s: AppSpan) -> Int64 {
        lock.lock(); defer { lock.unlock() }
        var st: OpaquePointer?
        defer { sqlite3_finalize(st) }
        sqlite3_prepare_v2(db, "INSERT INTO activity(start,end,bundle,app,title,domain,category) VALUES(?,?,?,?,?,?,?)", -1, &st, nil)
        sqlite3_bind_double(st, 1, s.start.timeIntervalSince1970)
        sqlite3_bind_double(st, 2, s.end.timeIntervalSince1970)
        sqlite3_bind_text(st, 3, s.bundle, -1, SQLITE_TRANSIENT)
        sqlite3_bind_text(st, 4, s.app, -1, SQLITE_TRANSIENT)
        if let t = s.title { sqlite3_bind_text(st, 5, t, -1, SQLITE_TRANSIENT) } else { sqlite3_bind_null(st, 5) }
        if let d = s.domain { sqlite3_bind_text(st, 6, d, -1, SQLITE_TRANSIENT) } else { sqlite3_bind_null(st, 6) }
        sqlite3_bind_text(st, 7, s.category, -1, SQLITE_TRANSIENT)
        sqlite3_step(st)
        return sqlite3_last_insert_rowid(db)
    }

    func setSpanEnd(_ id: Int64, _ end: Date) {
        lock.lock(); defer { lock.unlock() }
        var st: OpaquePointer?
        defer { sqlite3_finalize(st) }
        sqlite3_prepare_v2(db, "UPDATE activity SET end=? WHERE id=?", -1, &st, nil)
        sqlite3_bind_double(st, 1, end.timeIntervalSince1970)
        sqlite3_bind_int64(st, 2, id)
        sqlite3_step(st)
    }

    func spans(from: Date, to: Date) -> [AppSpan] {
        lock.lock(); defer { lock.unlock() }
        var st: OpaquePointer?
        defer { sqlite3_finalize(st) }
        sqlite3_prepare_v2(db, "SELECT start,end,bundle,app,title,domain,category FROM activity WHERE end>? AND start<? ORDER BY start", -1, &st, nil)
        sqlite3_bind_double(st, 1, from.timeIntervalSince1970)
        sqlite3_bind_double(st, 2, to.timeIntervalSince1970)
        var out: [AppSpan] = []
        func str(_ i: Int32) -> String? { sqlite3_column_text(st, i).map { String(cString: $0) } }
        while sqlite3_step(st) == SQLITE_ROW {
            // Aralık dışına taşan kısım kırpılır.
            let a = max(from.timeIntervalSince1970, sqlite3_column_double(st, 0))
            let b = min(to.timeIntervalSince1970, sqlite3_column_double(st, 1))
            guard b > a else { continue }
            out.append(AppSpan(start: Date(timeIntervalSince1970: a), end: Date(timeIntervalSince1970: b),
                               bundle: str(2) ?? "", app: str(3) ?? "", title: str(4), domain: str(5), category: str(6) ?? "Diğer"))
        }
        return out
    }

    /// Uygulama başına toplam süre (uzun dönemli raporlar için; tek tek kayıtlar belleğe alınmaz).
    func appTotals(from: Date, to: Date) -> [(bundle: String, app: String, category: String, time: TimeInterval)] {
        lock.lock(); defer { lock.unlock() }
        var st: OpaquePointer?
        defer { sqlite3_finalize(st) }
        sqlite3_prepare_v2(db, """
        SELECT bundle, MAX(app), MAX(category), SUM(MIN(end, ?2) - MAX(start, ?1)) t
        FROM activity WHERE end > ?1 AND start < ?2 AND end - start >= 3 GROUP BY bundle ORDER BY t DESC
        """, -1, &st, nil)
        sqlite3_bind_double(st, 1, from.timeIntervalSince1970)
        sqlite3_bind_double(st, 2, to.timeIntervalSince1970)
        var out: [(String, String, String, TimeInterval)] = []
        func str(_ i: Int32) -> String { sqlite3_column_text(st, i).map { String(cString: $0) } ?? "" }
        while sqlite3_step(st) == SQLITE_ROW { out.append((str(0), str(1), str(2), sqlite3_column_double(st, 3))) }
        return out
    }

    func addFileCounts(_ counts: [String: Int], hour: Date) {
        guard !counts.isEmpty else { return }
        lock.lock(); defer { lock.unlock() }
        var st: OpaquePointer?
        defer { sqlite3_finalize(st) }
        sqlite3_prepare_v2(db, "INSERT INTO file_activity(hour,project,count) VALUES(?,?,?) ON CONFLICT(hour,project) DO UPDATE SET count=count+excluded.count", -1, &st, nil)
        for (p, c) in counts {
            sqlite3_reset(st)
            sqlite3_bind_double(st, 1, hour.timeIntervalSince1970)
            sqlite3_bind_text(st, 2, p, -1, SQLITE_TRANSIENT)
            sqlite3_bind_int64(st, 3, Int64(c))
            sqlite3_step(st)
        }
    }

    func fileActivity(from: Date, to: Date) -> [(project: String, count: Int)] {
        lock.lock(); defer { lock.unlock() }
        var st: OpaquePointer?
        defer { sqlite3_finalize(st) }
        sqlite3_prepare_v2(db, "SELECT project, SUM(count) c FROM file_activity WHERE hour>=? AND hour<? GROUP BY project ORDER BY c DESC", -1, &st, nil)
        sqlite3_bind_double(st, 1, from.timeIntervalSince1970)
        sqlite3_bind_double(st, 2, to.timeIntervalSince1970)
        var out: [(String, Int)] = []
        while sqlite3_step(st) == SQLITE_ROW { out.append((String(cString: sqlite3_column_text(st, 0)), Int(sqlite3_column_int64(st, 1)))) }
        return out
    }

    /// Commit'lerini okuduğumuz proje kökleri (son 90 gün): dosya izleyici bunları izler, tüm ev klasörünü değil.
    func recentRoots() -> [String] {
        lock.lock(); defer { lock.unlock() }
        var st: OpaquePointer?
        defer { sqlite3_finalize(st) }
        sqlite3_prepare_v2(db, "SELECT DISTINCT path FROM events WHERE path IS NOT NULL AND path<>'' AND ts>=?", -1, &st, nil)
        sqlite3_bind_double(st, 1, Date().addingTimeInterval(-90 * 86400).timeIntervalSince1970)
        var out: [String] = []
        while sqlite3_step(st) == SQLITE_ROW { out.append(String(cString: sqlite3_column_text(st, 0))) }
        return out
    }

    func deleteComputerLog() {
        lock.lock(); defer { lock.unlock() }
        sqlite3_exec(db, "DELETE FROM activity; DELETE FROM file_activity;", nil, nil, nil)
    }
}

// MARK: - Kategoriler ve tarayıcılar

enum AppCategory {
    static let browsers: Set<String> = ["com.apple.Safari", "com.google.Chrome", "com.brave.Browser", "com.microsoft.edgemac",
                                        "company.thebrowser.Browser", "org.mozilla.firefox", "com.operasoftware.Opera", "com.vivaldi.Vivaldi"]
    private static let dev: Set<String> = ["com.apple.Terminal", "com.googlecode.iterm2", "dev.warp.Warp-Stable", "com.mitchellh.ghostty",
                                           "com.microsoft.VSCode", "com.todesktop.230313mzl4w4u92", "com.exafunction.windsurf", "com.anthropic.claudefordesktop"]
    private static let talk: Set<String> = ["com.tinyspeck.slackmacgap", "com.hnc.Discord", "us.zoom.xos", "com.microsoft.teams2",
                                            "com.apple.mail", "com.apple.MobileSMS", "net.whatsapp.WhatsApp", "ru.keepcoder.Telegram"]
    private static var cache: [String: String] = [:]

    static func of(_ bundle: String) -> String {
        if let c = cache[bundle] { return c }
        var c = "Diğer"
        if browsers.contains(bundle) { c = "Tarayıcı" }
        else if dev.contains(bundle) { c = "Geliştirme" }
        else if talk.contains(bundle) { c = "İletişim" }
        else if let url = NSWorkspace.shared.urlForApplication(withBundleIdentifier: bundle),
                let t = Bundle(url: url)?.infoDictionary?["LSApplicationCategoryType"] as? String {
            switch t.replacingOccurrences(of: "public.app-category.", with: "") {
            case "developer-tools": c = "Geliştirme"
            case "graphics-design", "photography", "video", "music": c = "Tasarım ve medya"
            case "social-networking", "business": c = "İletişim"
            case "productivity": c = "Verimlilik"
            case "education", "reference", "news", "books": c = "Okuma"
            case "entertainment", "games", "sports": c = "Eğlence"
            case "utilities": c = "Araçlar"
            default: c = "Diğer"
            }
        }
        cache[bundle] = c
        return c
    }

    /// Bu uygulamalarda pencere başlığı hiç saklanmaz.
    static func titleBlocked(_ bundle: String) -> Bool {
        let b = bundle.lowercased()
        return ["1password", "bitwarden", "keychainaccess", "lastpass", "dashlane", "keepass", "passwords"].contains { b.contains($0) }
    }
}

enum Browser {
    /// Çalışan tarayıcılardan herhangi biri için otomasyon izni verilmiş mi (soru sormadan).
    static var anyAllowed: Bool {
        NSWorkspace.shared.runningApplications.compactMap(\.bundleIdentifier).filter { AppCategory.browsers.contains($0) && $0 != "org.mozilla.firefox" }
            .contains { automation($0, ask: false) == noErr }
    }

    static func automation(_ bundle: String, ask: Bool) -> OSStatus {
        var target = AEAddressDesc()
        let data = bundle.data(using: .utf8)!
        _ = data.withUnsafeBytes { AECreateDesc(typeApplicationBundleID, $0.baseAddress, data.count, &target) }
        defer { AEDisposeDesc(&target) }
        return AEDeterminePermissionToAutomateTarget(&target, typeWildCard, typeWildCard, ask)
    }

    /// Açık sekmenin alan adı. Önce Erişilebilirlik ile (izin istemeden), olmazsa izin verilmişse AppleScript ile.
    /// Gizli/incognito pencereler atlanır. Sayfa başlığı ve tam adres saklanmaz.
    static func domain(bundle: String, window: AXUIElement?) -> String? {
        if let w = window, let url = axURL(w) { return host(url) }
        guard bundle != "org.mozilla.firefox", automation(bundle, ask: false) == noErr else { return nil }
        let src: String
        if bundle == "com.apple.Safari" {
            src = "tell application id \"com.apple.Safari\" to if (count of windows) > 0 then return URL of current tab of front window"
        } else {
            src = "tell application id \"\(bundle)\" to if (count of windows) > 0 then if mode of front window is not \"incognito\" then return URL of active tab of front window"
        }
        var err: NSDictionary?
        guard let r = NSAppleScript(source: "with timeout of 1 second\n\(src)\nend timeout")?.executeAndReturnError(&err).stringValue else { return nil }
        return host(r)
    }

    private static func host(_ s: String) -> String? {
        guard let u = URL(string: s), let h = u.host, ["http", "https"].contains(u.scheme ?? "") else { return nil }
        return h.hasPrefix("www.") ? String(h.dropFirst(4)) : h
    }

    /// Pencerenin AXDocument değeri ya da web alanının AXURL'i (Safari ve Chromium çoğu zaman verir).
    private static func axURL(_ w: AXUIElement) -> String? {
        var v: CFTypeRef?
        if AXUIElementCopyAttributeValue(w, kAXDocumentAttribute as CFString, &v) == .success, let s = v as? String, s.hasPrefix("http") { return s }
        return nil
    }
}

// MARK: - Kayıt motoru

/// Ön plandaki uygulamayı ve (izin varsa) pencere başlığını, tarayıcı alan adını not alır.
/// Uygulama değişimini macOS haber verir; ayrıca 30 sn'de bir başlık ve boşta durumu bakılır. Ağır iş yok.
@MainActor
final class ComputerLog {
    static let shared = ComputerLog()

    private var current: (id: Int64, span: AppSpan)?
    private var timer: Timer?
    private var started = false
    private let ownBundle = Bundle.main.bundleIdentifier ?? ""
    /// Uygulama sayılmayan sistem süreçleri (giriş ekranı, izin ve bildirim pencereleri gibi).
    private let ignored: Set<String> = ["com.apple.loginwindow", "com.apple.ScreenSaver.Engine", "com.apple.dock",
                                        "com.apple.UserNotificationCenter", "com.apple.accessibility.universalAccessAuthWarn",
                                        "com.apple.SecurityAgent", "com.apple.coreautha", "com.apple.CoreLocationAgent",
                                        "com.apple.controlcenter", "com.apple.notificationcenterui", "com.apple.Spotlight"]
    private var files: FileWatcher?
    private var tickCount = 1
    private var watchedRoots: [String] = []

    func start() {
        guard !started else { return }
        started = true
        HistoryStore.shared.setupActivityTables()
        let nc = NSWorkspace.shared.notificationCenter
        nc.addObserver(forName: NSWorkspace.didActivateApplicationNotification, object: nil, queue: .main) { [weak self] n in
            let app = n.userInfo?[NSWorkspace.applicationUserInfoKey] as? NSRunningApplication
            MainActor.assumeIsolated { self?.switchTo(app) }
        }
        for name in [NSWorkspace.screensDidSleepNotification, NSWorkspace.willSleepNotification, NSWorkspace.sessionDidResignActiveNotification] {
            nc.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in MainActor.assumeIsolated { self?.close() } }
        }
        for name in [NSWorkspace.screensDidWakeNotification, NSWorkspace.didWakeNotification, NSWorkspace.sessionDidBecomeActiveNotification] {
            nc.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated { if self?.screenLocked == false { self?.switchTo(NSWorkspace.shared.frontmostApplication) } }
            }
        }
        NotificationCenter.default.addObserver(forName: NSApplication.willTerminateNotification, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.close() }
        }
        let t = Timer(timeInterval: 30, repeats: true) { [weak self] _ in MainActor.assumeIsolated { self?.tick() } }
        t.tolerance = 10
        RunLoop.main.add(t, forMode: .common)
        timer = t
        switchTo(NSWorkspace.shared.frontmostApplication)
        restartFiles()
    }

    /// Ayar değişince çağrılır.
    func settingsChanged() {
        if !LogLayer.apps.on { close() } else if current == nil { switchTo(NSWorkspace.shared.frontmostApplication) }
        restartFiles()
    }

    private func restartFiles() {
        files?.stop(); files = nil
        guard LogLayer.files.on else { return }
        let roots = HistoryStore.shared.recentRoots()
        watchedRoots = roots.sorted()
        guard !roots.isEmpty else { return }
        files = FileWatcher(roots: roots)
    }

    private func refreshRootsIfChanged() {
        guard LogLayer.files.on, HistoryStore.shared.recentRoots().sorted() != watchedRoots else { return }
        restartFiles()
    }

    /// Kullanıcı en son ne zaman fare/klavye kullandı (izin gerekmez).
    private var idleSeconds: TimeInterval {
        CGEventSource.secondsSinceLastEventType(.combinedSessionState, eventType: CGEventType(rawValue: ~0)!)
    }

    /// Ekran kilitli mi (kilitliyken kayıt tutulmaz).
    private var screenLocked: Bool {
        (CGSessionCopyCurrentDictionary() as? [String: Any])?["CGSSessionScreenIsLocked"] as? Bool ?? false
    }

    /// Video oynatma ya da görüntülü görüşme ekranın uyumasını engelliyorsa kullanıcı izliyordur: boşta sayılmaz.
    private var mediaActive: Bool {
        var dict: Unmanaged<CFDictionary>?
        guard IOPMCopyAssertionsStatus(&dict) == kIOReturnSuccess, let d = dict?.takeRetainedValue() as? [String: Int] else { return false }
        return (d["PreventUserIdleDisplaySleep"] ?? 0) > 0
    }

    private func switchTo(_ app: NSRunningApplication?) {
        close()
        guard LogLayer.apps.on, let app, let bundle = app.bundleIdentifier, bundle != ownBundle, !ignored.contains(bundle) else { return }
        let (title, domain) = details(app, bundle: bundle)
        let s = AppSpan(start: Date(), end: Date(), bundle: bundle, app: app.localizedName ?? bundle,
                        title: title, domain: domain, category: AppCategory.of(bundle))
        current = (HistoryStore.shared.insertSpan(s), s)
    }

    private func close(at end: Date = Date()) {
        guard let c = current else { return }
        current = nil
        if end.timeIntervalSince(c.span.start) < 3 {
            HistoryStore.shared.setSpanEnd(c.id, c.span.start)   // çok kısa: 0 sürelik kalır, gösterilmez
        } else {
            HistoryStore.shared.setSpanEnd(c.id, end)
        }
    }

    private func tick() {
        files?.flush()
        guard LogLayer.apps.on else { return }
        if screenLocked { close(); return }
        if tickCount % 120 == 0 { refreshRootsIfChanged() }   // yaklaşık saatte bir yeni projeler izlenir
        if tickCount % 10 == 0 { malloc_zone_pressure_relief(nil, 0) }   // ~5 dakikada bir boş belleği sisteme geri ver
        tickCount += 1
        let idle = idleSeconds
        if idle > 5 * 60 && !mediaActive {
            // Uzun süre fare/klavye yok: boşta sayılır, kayıt son hareket anında kapanır.
            close(at: Date().addingTimeInterval(-idle))
            return
        }
        guard let app = NSWorkspace.shared.frontmostApplication else { return }
        guard var c = current else { switchTo(app); return }
        // Pencere ya da site değiştiyse yeni kayıt; değilse bitiş zamanı güncellenir (çökmede en çok 30 sn kaybolur).
        let (title, domain) = details(app, bundle: c.span.bundle)
        if title != c.span.title || domain != c.span.domain {
            switchTo(app)
        } else {
            c.span.end = Date()
            current = c
            HistoryStore.shared.setSpanEnd(c.id, c.span.end)
        }
    }

    /// Pencere başlığı (izin varsa) ve tarayıcıda alan adı. Tarayıcılarda başlık saklanmaz.
    private func details(_ app: NSRunningApplication, bundle: String) -> (String?, String?) {
        let isBrowser = AppCategory.browsers.contains(bundle)
        guard (LogLayer.titles.on && !isBrowser) || (LogLayer.sites.on && isBrowser) else { return (nil, nil) }
        var window: AXUIElement?
        if AXIsProcessTrusted() {
            let ax = AXUIElementCreateApplication(app.processIdentifier)
            AXUIElementSetMessagingTimeout(ax, 0.25)   // donmuş bir uygulama Tonti'yi dondurmasın
            var v: CFTypeRef?
            if AXUIElementCopyAttributeValue(ax, kAXFocusedWindowAttribute as CFString, &v) == .success, let w = v {
                window = (w as! AXUIElement)
            }
        }
        if isBrowser { return (nil, LogLayer.sites.on ? Browser.domain(bundle: bundle, window: window) : nil) }
        guard let w = window, !AppCategory.titleBlocked(bundle) else { return (nil, nil) }
        var t: CFTypeRef?
        guard AXUIElementCopyAttributeValue(w, kAXTitleAttribute as CFString, &t) == .success, let s = t as? String, !s.isEmpty else { return (nil, nil) }
        return (String(s.prefix(160)), nil)
    }
}

// MARK: - Dosya değişiklikleri

/// Proje köklerindeki değişiklikleri klasör düzeyinde, 30 sn gecikmeyle sayar; saat başına proje toplamı saklanır.
/// Dosya içeriği ve adı saklanmaz; yalnızca "şu projede şu kadar değişiklik".
final class FileWatcher {
    private var stream: FSEventStreamRef?
    private let roots: [String]
    private var pending: [String: Int] = [:]
    private let lock = NSLock()
    /// Geri çağrılar ve durdurma aynı sıra kuyruğunda: durdurulmuş izleyiciye geri çağrı düşmez.
    private let queue = DispatchQueue(label: "meowly.files", qos: .utility)
    private static let skip = ["/.git/", "/node_modules/", "/.build/", "/DerivedData/", "/build/", "/.next/", "/dist/", "/Pods/", "/.venv/"]

    init(roots: [String]) {
        self.roots = roots.sorted { $0.count > $1.count }
        var ctx = FSEventStreamContext(version: 0, info: Unmanaged.passUnretained(self).toOpaque(), retain: nil, release: nil, copyDescription: nil)
        let cb: FSEventStreamCallback = { _, info, n, paths, _, _ in
            guard let info else { return }
            let me = Unmanaged<FileWatcher>.fromOpaque(info).takeUnretainedValue()
            // UseCFTypes ile gelen yollar CFString dizisidir. (Bu bayrak olmadan `char **` gelir; onu NSArray sanmak
            // objc_msgSend ile çöküyordu: proje klasöründe dosya değişince uygulama kapanıyordu.)
            let arr = unsafeBitCast(paths, to: CFArray.self) as? [String] ?? []
            me.record(Array(arr.prefix(n)))
        }
        stream = FSEventStreamCreate(nil, cb, &ctx, roots as CFArray, FSEventStreamEventId(kFSEventStreamEventIdSinceNow), 30,
                                     FSEventStreamCreateFlags(kFSEventStreamCreateFlagIgnoreSelf | kFSEventStreamCreateFlagUseCFTypes))
        if let s = stream {
            FSEventStreamSetDispatchQueue(s, queue)
            FSEventStreamStart(s)
        }
    }

    private func record(_ paths: [String]) {
        lock.lock(); defer { lock.unlock() }
        for p in paths {
            let pp = p.hasSuffix("/") ? p : p + "/"
            if Self.skip.contains(where: { pp.contains($0) }) { continue }
            guard let root = roots.first(where: { pp.hasPrefix($0 + "/") }) else { continue }
            pending[URL(fileURLWithPath: root).lastPathComponent, default: 0] += 1
        }
    }

    func flush() {
        lock.lock(); let c = pending; pending = [:]; lock.unlock()
        let hour = Calendar.current.dateInterval(of: .hour, for: Date())?.start ?? Date()
        HistoryStore.shared.addFileCounts(c, hour: hour)
    }

    func stop() {
        flush()
        let s = stream
        stream = nil
        queue.sync { if let s { FSEventStreamStop(s); FSEventStreamInvalidate(s); FSEventStreamRelease(s) } }
    }
}

// MARK: - Takvim

enum CalendarLog {
    struct Meeting: Hashable { let title: String; let start: Date; let end: Date }
    private static let store = EKEventStore()

    /// Yalnızca izin verilmişse ve katman açıksa; gün boyu etkinlikler ve reddedilenler atlanır.
    static func meetings(from: Date, to: Date) -> [Meeting] {
        guard LogLayer.calendar.on, EKEventStore.authorizationStatus(for: .event) == .fullAccess else { return [] }
        let pred = store.predicateForEvents(withStart: from, end: to, calendars: nil)
        return store.events(matching: pred)
            .filter { !$0.isAllDay && $0.status != .canceled && ($0.attendees?.first(where: \.isCurrentUser)?.participantStatus ?? .accepted) != .declined }
            .map { Meeting(title: $0.title ?? "", start: max($0.startDate, from), end: min($0.endDate, to)) }
            .sorted { $0.start < $1.start }
    }

    static func request(_ done: @escaping @Sendable (Bool) -> Void) {
        store.requestFullAccessToEvents { ok, _ in done(ok) }
    }
}
