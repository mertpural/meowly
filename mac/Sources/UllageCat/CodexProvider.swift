import Foundation

/// ~/.codex/sessions altındaki oturum kayıtlarından limit ve token okur. Codex kayıtları .jsonl veya
/// sıkıştırılmış .jsonl.zst olarak yazılır; ikincisi için sistemdeki zstd kullanılır.
enum CodexProvider {
    private static let mtimeKey = "codexMtimes"

    private struct Window { let used: Double; let resets: Double }

    private static let zstdPath: String? = ["/opt/homebrew/bin/zstd", "/usr/local/bin/zstd", "/usr/bin/zstd"]
        .first { FileManager.default.isExecutableFile(atPath: $0) }

    private static func sessionsRoot() -> URL {
        FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".codex/sessions")
    }

    private static func sessionFiles() -> [(url: URL, mod: Date)] {
        guard let en = FileManager.default.enumerator(at: sessionsRoot(), includingPropertiesForKeys: [.contentModificationDateKey]) else { return [] }
        var out: [(url: URL, mod: Date)] = []
        for case let url as URL in en {
            let name = url.lastPathComponent
            guard name.hasSuffix(".jsonl") || name.hasSuffix(".jsonl.zst") else { continue }
            let mod = (try? url.resourceValues(forKeys: [.contentModificationDateKey]))?.contentModificationDate ?? .distantPast
            out.append((url, mod))
        }
        return out
    }

    /// Oturum dosyasını satır satır okur; bütün dosya belleğe alınmaz. .zst dosyaları zstd ile akış hâlinde açılır.
    /// Dönen değer: son \n dahil okunan bayt sayısı; dosya açılamadıysa ya da zstd hata verdiyse nil.
    private static func eachLine(_ url: URL, _ body: (Data, Int) -> Void) -> Int? {
        guard url.pathExtension == "zst" else {
            guard let h = try? FileHandle(forReadingFrom: url) else { return nil }
            defer { try? h.close() }
            return LineStream.each(h, body)
        }
        guard let zstd = zstdPath else { return nil }
        let p = Process()
        p.executableURL = URL(fileURLWithPath: zstd)
        p.arguments = ["-dc", url.path]
        let out = Pipe()
        p.standardOutput = out
        p.standardError = FileHandle.nullDevice
        do { try p.run() } catch { return nil }
        let n = LineStream.each(out.fileHandleForReading, body)
        p.waitUntilExit()
        return p.terminationStatus == 0 ? n : nil
    }

    private typealias Limits = (primary: Window?, secondary: Window?, stamp: Date)
    private static let cacheLock = NSLock()
    nonisolated(unsafe) private static var limitCache: [String: (mod: Date, limits: Limits?)] = [:]

    /// Dosyadaki en yeni limit kaydı. Dosya değişmediyse önceki sonuç kullanılır: her yenilemede
    /// büyük bir arşivi yeniden açmaya gerek yok (bellek ve işlemci).
    private static func latestRateLimits(_ f: (url: URL, mod: Date)) -> Limits? {
        cacheLock.lock()
        let hit = limitCache[f.url.path]
        cacheLock.unlock()
        if let hit, hit.mod == f.mod { return hit.limits }
        let iso = ISO8601DateFormatter()
        iso.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        let marker = Data("rate_limits".utf8)
        var best: Limits?
        let ok = eachLine(f.url) { line, _ in
            guard line.range(of: marker) != nil,
                  let obj = try? JSONSerialization.jsonObject(with: line) as? [String: Any],
                  let payload = obj["payload"] as? [String: Any],
                  let rl = payload["rate_limits"] as? [String: Any],
                  let ts = (obj["timestamp"] as? String).flatMap(iso.date(from:))
            else { return }
            if best == nil || ts > best!.stamp {
                best = (window(rl["primary"]), window(rl["secondary"]), ts)
            }
        }
        guard ok != nil else { return nil }   // okunamadıysa önbelleğe yazılmaz, sonra yeniden denenir
        cacheLock.lock()
        limitCache[f.url.path] = (f.mod, best)
        cacheLock.unlock()
        return best
    }

    /// Bir oturum dosyasının `start` konumundan sonraki token olayları. Dosyaya yazmaz; ingest ve deneme kullanır.
    /// Dönen: olaylar, son \n'den sonraki konum, son toplam. Dosya okunamadıysa nil.
    static func collect(_ url: URL, start: Int, prevTotal: Int) -> (events: [HistoryStore.Event], end: Int, total: Int)? {
        let iso = ISO8601DateFormatter()
        iso.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        var model = "codex"
        var prev = prevTotal
        var firstLine = Data()
        var events: [HistoryStore.Event] = []
        guard let end = eachLine(url, { line, at in
            if at == 0 { firstLine = line }
            guard at + line.count > start else { return }
            // Başlangıç bir satırın ortasına düşerse (olmamalı) eskisi gibi o satırın kalanı okunur.
            let part = at >= start ? line : Data(line.suffix(from: line.startIndex + (start - at)))
            guard !part.isEmpty,
                  let obj = try? JSONSerialization.jsonObject(with: part) as? [String: Any],
                  let payload = obj["payload"] as? [String: Any] else { return }
            if obj["type"] as? String == "turn_context", let m = payload["model"] as? String { model = m; return }
            guard payload["type"] as? String == "token_count",
                  let info = payload["info"] as? [String: Any],
                  let last = info["last_token_usage"] as? [String: Any],
                  let total = (info["total_token_usage"] as? [String: Any])?["total_tokens"] as? Int,
                  let ts = (obj["timestamp"] as? String).flatMap(iso.date(from:)),
                  let ord = obj["ordinal"] as? Int else { return }
            // Aynı toplamı taşıyan tekrar olaylar çift sayılmasın.
            if total == prev { return }
            prev = total
            let input = last["input_tokens"] as? Int ?? 0
            let cached = last["cached_input_tokens"] as? Int ?? 0
            events.append(.init(
                provider: "codex", key: "\(url.lastPathComponent)|\(ord)", ts: ts.timeIntervalSince1970,
                model: model, input: max(0, input - cached),
                output: last["output_tokens"] as? Int ?? 0, cacheWrite: 0, cacheRead: cached
            ))
        }) else { return nil }
        guard end > start else { return ([], end, prevTotal) }
        // Oturumun çalışma klasörü dosyanın başındaki session_meta satırında.
        let cwd = ((try? JSONSerialization.jsonObject(with: firstLine) as? [String: Any])?["payload"] as? [String: Any])?["cwd"] as? String
        let root = cwd.flatMap(RepoRoot.resolve)
        let project = root.map { URL(fileURLWithPath: $0).lastPathComponent } ?? ""
        for i in events.indices { events[i].project = project; events[i].path = root ?? "" }
        return (events, end, prev)
    }

    static func read(now: Date = Date()) -> Usage {
        ingest()
        let files = sessionFiles().sorted { $0.mod > $1.mod }
        if zstdPath == nil && files.contains(where: { $0.url.pathExtension == "zst" }) {
            return Usage(notes: [L.t("Codex kayıtları sıkıştırılmış (.zst). Okumak için zstd gerekli: brew install zstd")])
        }
        // En yeni dosyadan başlayıp ilk limit kaydında dur: daha eski dosyalar daha eski değer taşır.
        var latest: (primary: Window?, secondary: Window?, stamp: Date)?
        for f in files {
            guard let r = latestRateLimits(f) else { continue }
            latest = r
            break
        }
        guard let r = latest else { return Usage(notes: [L.t("Codex kayıtlarında limit bilgisi bulunamadı.")]) }

        var u = Usage(
            fiveHourLeft: left(r.primary, now: now),
            weeklyLeft: left(r.secondary, now: now),
            fiveHourReset: r.primary.map { Date(timeIntervalSince1970: $0.resets) },
            weeklyReset: r.secondary.map { Date(timeIntervalSince1970: $0.resets) },
            todayTokens: HistoryStore.shared.sums(provider: "codex", since: Calendar.current.startOfDay(for: now)).billable,
            limitConfidence: .exact,
            tokenConfidence: .exact
        )
        let fmt = DateFormatter()
        fmt.dateStyle = .medium; fmt.timeStyle = .short
        u.notes.append(L.f("Limit ve token: Codex oturum kayıtlarından (son kayıt %@).", fmt.string(from: r.stamp)))
        if let p = r.primary, p.resets <= now.timeIntervalSince1970 {
            u.notes.append(L.t("Son kayıttan beri 5 saatlik pencere sıfırlandı; yeni kullanım olmadığı için %100 gösteriliyor."))
        }
        if let w = r.secondary, w.resets <= now.timeIntervalSince1970 {
            u.notes.append(L.t("Haftalık pencere sıfırlandı; yeni kullanım olmadığı için %100 gösteriliyor."))
        }
        return u
    }

    /// Bu dosyaların token kayıtlarını HistoryStore'a işler. Değişmemiş dosyalar atlanır, değişenler
    /// kaldığı yerden (bayt konumu) okunur.
    private static func ingest() {
        if !UserDefaults.standard.bool(forKey: "codexIngestV4") {
            HistoryStore.shared.reset(provider: "codex", pathContains: "/.codex/sessions/")
            UserDefaults.standard.removeObject(forKey: mtimeKey)
            UserDefaults.standard.set(true, forKey: "codexIngestV4")
        }
        let store = HistoryStore.shared
        var mtimes = UserDefaults.standard.dictionary(forKey: mtimeKey) as? [String: Double] ?? [:]
        var changed = false
        for f in sessionFiles() {
            let mod = f.mod.timeIntervalSince1970
            if mtimes[f.url.path] == mod { continue }
            let start = store.fileOffset(f.url.path)
            guard let r = collect(f.url, start: start, prevTotal: store.fileTotal(f.url.path) ?? 0) else { continue }
            if r.end > start {
                store.upsert(r.events)
                store.setFileTotal(f.url.path, r.total)
                store.setFileOffset(f.url.path, r.end)
            }
            mtimes[f.url.path] = mod
            changed = true
        }
        // Değişiklik yoksa ayar dosyasına yazılmaz (her yenilemede gereksiz disk yazımı olmasın).
        if changed { UserDefaults.standard.set(mtimes, forKey: mtimeKey) }
    }

    private static func window(_ any: Any?) -> Window? {
        guard let d = any as? [String: Any],
              let used = d["used_percent"] as? Double,
              let resets = d["resets_at"] as? Double else { return nil }
        return Window(used: used, resets: resets)
    }

    /// Pencere sıfırlandıysa (resets_at geçmişte) kullanım 0 sayılır.
    private static func left(_ w: Window?, now: Date) -> Double? {
        guard let w else { return nil }
        if w.resets <= now.timeIntervalSince1970 { return 1 }
        return max(0, min(1, 1 - w.used / 100))
    }
}
