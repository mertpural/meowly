import Foundation
import SQLite3

// MARK: - Proje kökü

/// Çalışma klasörünü projenin git kök klasörüne bağlar: `ai_limit/macos` → `ai_limit`.
/// Git yoksa klasörün kendisi. Ev klasörü proje sayılmaz. Sonuç önbellekte tutulur, git çalıştırılmaz (yalnızca `.git` aranır).
enum RepoRoot {
    private static var cache: [String: String?] = [:]
    private static let lock = NSLock()

    static func resolve(_ cwd: String) -> String? {
        lock.lock(); defer { lock.unlock() }
        if let c = cache[cwd] { return c }
        let r = find(cwd)
        cache[cwd] = r
        return r
    }

    private static func find(_ cwd: String) -> String? {
        let home = NSHomeDirectory()
        guard !cwd.isEmpty, cwd != home, cwd != "/" else { return nil }
        // Claude Code çalışma ağaçları: …/proje/.claude/worktrees/ad → proje
        if let r = cwd.range(of: "/.claude/worktrees/") { return find(String(cwd[..<r.lowerBound])) }
        let fm = FileManager.default
        var u = URL(fileURLWithPath: cwd)
        while u.path != "/" && u.path != home {
            let g = u.appendingPathComponent(".git")
            var isDir: ObjCBool = false
            if fm.fileExists(atPath: g.path, isDirectory: &isDir) {
                // git worktree: .git bir dosyadır, ana deponun yolunu gösterir.
                if !isDir.boolValue, let s = try? String(contentsOf: g, encoding: .utf8),
                   let r = s.range(of: "gitdir: ") {
                    let gd = s[r.upperBound...].trimmingCharacters(in: .whitespacesAndNewlines)
                    if let w = gd.range(of: "/.git/worktrees/") { return String(gd[..<w.lowerBound]) }
                }
                return u.path
            }
            u = u.deletingLastPathComponent()
        }
        return cwd
    }
}

// MARK: - Oturumlar

/// Bir çalışma oturumu: aynı projede, aralarında 20 dakikadan uzun boşluk olmayan olaylar.
/// Kedinin balonları, Günlük ve Raporlar hep bunu kullanır; süreler her yerde aynı çıkar.
struct WorkSession: Identifiable, Hashable {
    var id: String { "\(project)|\(start.timeIntervalSince1970)" }
    var start: Date
    var end: Date
    var project: String          // boş = proje bilinmiyor
    var path: String?
    var providers: Set<String>
    var tokens: Int
    /// Ekranda gösterilen süre: ilk olaydan son olaya, en az 1 dakika.
    var duration: TimeInterval { max(60, end.timeIntervalSince(start)) }
}

extension HistoryStore {
    static let sessionGap: TimeInterval = 20 * 60

    func sessions(from: Date, to: Date = Date.distantFuture) -> [WorkSession] {
        let cache = Prefs.includeCache ? "+COALESCE(cache_read,0)" : ""
        var rows: [(ts: Double, provider: String, project: String, path: String?, tokens: Int)] = []
        lock.lock()
        var st: OpaquePointer?
        sqlite3_prepare_v2(db, """
        SELECT ts, provider, COALESCE(project,''), path, COALESCE(input,0)+COALESCE(output,0)+COALESCE(cache_write,0)\(cache)
        FROM events WHERE ts>=? AND ts<? ORDER BY ts
        """, -1, &st, nil)
        sqlite3_bind_double(st, 1, from.timeIntervalSince1970)
        sqlite3_bind_double(st, 2, min(to.timeIntervalSince1970, 1e11))
        while sqlite3_step(st) == SQLITE_ROW {
            let path = sqlite3_column_text(st, 3).map { String(cString: $0) }
            rows.append((sqlite3_column_double(st, 0), String(cString: sqlite3_column_text(st, 1)),
                         String(cString: sqlite3_column_text(st, 2)), (path?.isEmpty ?? true) ? nil : path,
                         Int(sqlite3_column_int64(st, 4))))
        }
        sqlite3_finalize(st)
        lock.unlock()

        // Her proje için açık bir oturum tutulur; iki projede paralel çalışmak oturumları bölmez.
        var open: [String: WorkSession] = [:]
        var done: [WorkSession] = []
        var lastProject = ""
        for r in rows {
            // Projesiz olay (ör. Antigravity) en son çalışılan projeye yazılır.
            let key = r.project.isEmpty ? lastProject : r.project
            let t = Date(timeIntervalSince1970: r.ts)
            if var s = open[key], t.timeIntervalSince(s.end) <= Self.sessionGap {
                s.end = t; s.tokens += r.tokens; s.providers.insert(r.provider)
                if s.path == nil { s.path = r.path }
                open[key] = s
            } else {
                if let s = open[key] { done.append(s) }
                open[key] = WorkSession(start: t, end: t, project: key, path: r.path, providers: [r.provider], tokens: r.tokens)
            }
            if !r.project.isEmpty { lastProject = r.project }
        }
        done.append(contentsOf: open.values)
        return done.sorted { $0.start < $1.start }
    }

    /// Günlük toplam token (son `days` gün, eskiden yeniye).
    func dailyTokens(days: Int) -> [(day: Date, tokens: Int)] {
        let cal = Calendar.current
        let today = cal.startOfDay(for: Date())
        guard let first = cal.date(byAdding: .day, value: -(days - 1), to: today) else { return [] }
        let cache = Prefs.includeCache ? "+COALESCE(cache_read,0)" : ""
        var map: [String: Int] = [:]
        lock.lock()
        var st: OpaquePointer?
        sqlite3_prepare_v2(db, """
        SELECT strftime('%Y-%m-%d', ts, 'unixepoch', 'localtime') d, SUM(COALESCE(input,0)+COALESCE(output,0)+COALESCE(cache_write,0)\(cache))
        FROM events WHERE ts>=? GROUP BY d
        """, -1, &st, nil)
        sqlite3_bind_double(st, 1, first.timeIntervalSince1970)
        while sqlite3_step(st) == SQLITE_ROW { map[String(cString: sqlite3_column_text(st, 0))] = Int(sqlite3_column_int64(st, 1)) }
        sqlite3_finalize(st)
        lock.unlock()
        let f = DateFormatter(); f.dateFormat = "yyyy-MM-dd"
        return (0..<days).compactMap { i in
            guard let d = cal.date(byAdding: .day, value: i, to: first) else { return nil }
            return (d, map[f.string(from: d)] ?? 0)
        }
    }
}

// MARK: - Commit'ler

/// Proje klasöründeki commit başlıkları (yalnızca senin attıkların). Sohbetler okunmaz.
/// Yalnızca Günlük/Raporlar açılınca, arka planda çalışır; depo+gün başına önbellekte tutulur.
enum GitLog {
    struct Commit: Hashable { let date: Date; let subject: String }

    private static let lock = NSLock()
    private static var cache: [String: [Commit]] = [:]
    private static var emails: [String: String] = [:]

    /// Geliştirici araçları kurulu değilse /usr/bin/git kurulum penceresi açar; önce denetlenir.
    static let available: Bool = {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/xcode-select")
        p.arguments = ["-p"]
        p.standardOutput = Pipe(); p.standardError = Pipe()
        guard (try? p.run()) != nil else { return false }
        p.waitUntilExit()
        return p.terminationStatus == 0
    }()

    static var enabled: Bool { UserDefaults.standard.object(forKey: "journalCommits") as? Bool ?? true }

    private static func git(_ path: String, _ args: [String]) -> String? {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/git")
        p.arguments = ["-C", path] + args
        let out = Pipe()
        p.standardOutput = out; p.standardError = Pipe()
        guard (try? p.run()) != nil else { return nil }
        let data = out.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        return p.terminationStatus == 0 ? String(data: data, encoding: .utf8) : nil
    }

    /// Bir deponun bir günündeki commit'ler (eskiden yeniye).
    static func commits(path: String, day: Date) -> [Commit] {
        let start = Calendar.current.startOfDay(for: day)
        return commits(path: path, from: start, to: start.addingTimeInterval(86400))
    }

    /// Bir deponun bir aralıktaki commit'leri (eskiden yeniye). Süren dönemler (bugün, bu hafta) önbelleğe alınmaz.
    static func commits(path: String, from start: Date, to end: Date) -> [Commit] {
        guard enabled, available, FileManager.default.fileExists(atPath: path + "/.git") else { return [] }
        let key = "\(path)|\(Int(start.timeIntervalSince1970))|\(Int(end.timeIntervalSince1970))"
        let finished = end <= Date()
        lock.lock()
        if finished, let c = cache[key] { lock.unlock(); return c }
        var email = emails[path]
        lock.unlock()
        if email == nil {
            email = git(path, ["config", "user.email"])?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            lock.lock(); emails[path] = email; lock.unlock()
        }
        var args = ["log", "--all", "--no-merges", "--format=%ct%x09%s",
                    "--since=@\(Int(start.timeIntervalSince1970))", "--until=@\(Int(end.timeIntervalSince1970))"]
        if let e = email, !e.isEmpty { args.append("--author=\(e)") }
        let out = git(path, args) ?? ""
        let list: [Commit] = out.split(separator: "\n").compactMap { line in
            let parts = line.split(separator: "\t", maxSplits: 1)
            guard parts.count == 2, let t = Double(parts[0]) else { return nil }
            return Commit(date: Date(timeIntervalSince1970: t), subject: String(parts[1]))
        }.sorted { $0.date < $1.date }
        lock.lock(); cache[key] = list; lock.unlock()
        return list
    }

    /// Yapay zekâ oturumu bir klasörde açılıp commit başka depoya atılabilir (ör. oturum `ai_limit`'te, commit `ullage-cat`'te).
    /// Bunun için oturum klasörleri ve onların bir üst klasöründeki kardeş git depoları da taranır. Saatte bir yenilenir.
    private static var repoCache: (at: Date, list: [String])?
    static func nearbyRepos() -> [String] {
        lock.lock()
        if let c = repoCache, Date().timeIntervalSince(c.at) < 3600 { lock.unlock(); return c.list }
        lock.unlock()
        let fm = FileManager.default
        let home = NSHomeDirectory()
        var set = Set<String>()
        let roots = HistoryStore.shared.recentRoots()
        for r in roots where fm.fileExists(atPath: r + "/.git") { set.insert(r) }
        for parent in Set(roots.map { ($0 as NSString).deletingLastPathComponent }) where parent.count > home.count {
            for name in (try? fm.contentsOfDirectory(atPath: parent)) ?? [] where !name.hasPrefix(".") {
                let p = parent + "/" + name
                if fm.fileExists(atPath: p + "/.git") { set.insert(p) }
            }
        }
        let list = Array(Array(set).sorted().prefix(60))
        lock.lock(); repoCache = (Date(), list); lock.unlock()
        return list
    }

    /// Oturumu olmayan depolardaki commit'ler. `known`: zaten oturumla gösterilen depo yolları.
    static func otherCommits(from start: Date, to end: Date, excluding known: Set<String>) -> [(name: String, path: String, commits: [Commit])] {
        nearbyRepos().filter { !known.contains($0) }.compactMap { p in
            let c = commits(path: p, from: start, to: end)
            return c.isEmpty ? nil : ((p as NSString).lastPathComponent, p, c)
        }
    }

    /// Oturuma düşen commit'ler: oturum başından 10 dk önce ile bitişinden 30 dk sonrası arası.
    static func commits(for s: WorkSession, in dayCommits: [Commit]) -> [Commit] {
        let a = s.start.addingTimeInterval(-10 * 60), b = s.end.addingTimeInterval(30 * 60)
        return dayCommits.filter { $0.date >= a && $0.date <= b }
    }
}
