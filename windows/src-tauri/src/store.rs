//! Token geçmişi: %APPDATA%\Meowly\history.sqlite (Mac: HistoryStore.swift + Journal.swift oturumları).
//! Log dosyaları silinse bile geçmiş kalır.

use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Mutex, OnceLock};

pub struct Event {
    pub provider: &'static str,
    pub key: String,
    pub ts: f64,
    pub model: String,
    pub input: i64,
    pub output: i64,
    pub cache_write: i64,
    pub cache_read: i64,
    pub project: String,
    pub path: String,
}

fn db() -> &'static Mutex<Connection> {
    static D: OnceLock<Mutex<Connection>> = OnceLock::new();
    D.get_or_init(|| {
        let c = Connection::open(crate::settings::data_dir().join("history.sqlite")).expect("history.sqlite");
        let _ = c.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS events(
               provider TEXT NOT NULL, key TEXT NOT NULL, ts REAL NOT NULL, model TEXT,
               input INTEGER, output INTEGER, cache_write INTEGER, cache_read INTEGER,
               project TEXT, path TEXT, PRIMARY KEY(provider, key));
             CREATE INDEX IF NOT EXISTS idx_events_ts ON events(provider, ts);
             CREATE TABLE IF NOT EXISTS file_state(path TEXT PRIMARY KEY, offset INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS file_total(path TEXT PRIMARY KEY, total INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS activity(id INTEGER PRIMARY KEY, start REAL, end REAL, bundle TEXT, app TEXT, title TEXT, domain TEXT, category TEXT);
             CREATE TABLE IF NOT EXISTS file_activity(hour REAL, project TEXT, count INT, PRIMARY KEY(hour, project));
             CREATE INDEX IF NOT EXISTS idx_activity_start ON activity(start);
             CREATE TABLE IF NOT EXISTS kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        );
        Mutex::new(c)
    })
}

fn with<R>(f: impl FnOnce(&Connection) -> R) -> R { f(&db().lock().unwrap()) }

/// "Harcanan token": cache okumaları yalnızca "Cache dahil" açıksa.
fn token_expr() -> &'static str {
    if crate::settings::get().include_cache {
        "COALESCE(input,0)+COALESCE(output,0)+COALESCE(cache_write,0)+COALESCE(cache_read,0)"
    } else {
        "COALESCE(input,0)+COALESCE(output,0)+COALESCE(cache_write,0)"
    }
}

// MARK: Küçük kalıcı değerler (Mac'te UserDefaults)

pub fn kv_get(k: &str) -> Option<String> {
    with(|c| c.query_row("SELECT value FROM kv WHERE key=?", [k], |r| r.get(0)).optional().ok().flatten())
}
pub fn kv_set(k: &str, v: &str) {
    with(|c| { let _ = c.execute("INSERT OR REPLACE INTO kv(key,value) VALUES(?,?)", params![k, v]); });
}
pub fn kv_f64(k: &str) -> f64 { kv_get(k).and_then(|v| v.parse().ok()).unwrap_or(0.0) }

// MARK: Dosya okuma konumları

pub fn file_offset(p: &str) -> u64 {
    with(|c| c.query_row("SELECT offset FROM file_state WHERE path=?", [p], |r| r.get::<_, i64>(0)).optional().ok().flatten().unwrap_or(0) as u64)
}
pub fn set_file_offset(p: &str, o: u64) {
    with(|c| { let _ = c.execute("INSERT OR REPLACE INTO file_state(path,offset) VALUES(?,?)", params![p, o as i64]); });
}
pub fn file_total(p: &str) -> Option<i64> {
    with(|c| c.query_row("SELECT total FROM file_total WHERE path=?", [p], |r| r.get(0)).optional().ok().flatten())
}
pub fn set_file_total(p: &str, t: i64) {
    with(|c| { let _ = c.execute("INSERT OR REPLACE INTO file_total(path,total) VALUES(?,?)", params![p, t]); });
}

/// Akış sırasında aynı mesaj birkaç kez yazılır; en büyük değerler tutulur.
pub fn upsert(events: &[Event]) {
    if events.is_empty() { return; }
    with(|c| {
        let _ = c.execute_batch("BEGIN");
        if let Ok(mut st) = c.prepare_cached(
            "INSERT INTO events(provider,key,ts,model,input,output,cache_write,cache_read,project,path)
             VALUES(?,?,?,?,?,?,?,?,?,?)
             ON CONFLICT(provider,key) DO UPDATE SET
               project=CASE WHEN excluded.project<>'' THEN excluded.project ELSE project END,
               path=CASE WHEN excluded.path<>'' THEN excluded.path ELSE path END,
               input=MAX(input,excluded.input), output=MAX(output,excluded.output),
               cache_write=MAX(cache_write,excluded.cache_write),
               cache_read=MAX(cache_read,excluded.cache_read)",
        ) {
            for e in events {
                let _ = st.execute(params![e.provider, e.key, e.ts, e.model, e.input, e.output, e.cache_write, e.cache_read, e.project, e.path]);
            }
        }
        let _ = c.execute_batch("COMMIT");
    });
}

/// Bir sağlayıcının harcanan token toplamı (ts ≥ since).
pub fn billable(provider: &str, since: f64) -> i64 {
    let q = format!("SELECT COALESCE(SUM({}),0) FROM events WHERE provider=? AND ts>=?", token_expr());
    with(|c| c.query_row(&q, params![provider, since], |r| r.get(0)).unwrap_or(0))
}

/// Tüm araçların token toplamı (ts ≥ from).
pub fn total_tokens(from: f64) -> i64 {
    let q = format!("SELECT COALESCE(SUM({}),0) FROM events WHERE ts>=?", token_expr());
    with(|c| c.query_row(&q, [from], |r| r.get(0)).unwrap_or(0))
}

/// Bir sağlayıcının en yeni olayının zamanı (yoksa 0).
pub fn latest_event(provider: &str) -> f64 {
    with(|c| c.query_row("SELECT MAX(ts) FROM events WHERE provider=?", [provider], |r| r.get::<_, Option<f64>>(0)).ok().flatten().unwrap_or(0.0))
}

/// En son kullanılan proje (klasör adı).
pub fn latest_project() -> Option<String> {
    with(|c| c.query_row("SELECT project FROM events WHERE project IS NOT NULL AND project<>'' ORDER BY ts DESC LIMIT 1", [], |r| r.get(0)).optional().ok().flatten())
}

/// Aralıktaki olay zamanları (artan).
pub fn activity_times(from: f64) -> Vec<f64> {
    with(|c| {
        let Ok(mut st) = c.prepare("SELECT ts FROM events WHERE ts>=? ORDER BY ts") else { return vec![] };
        st.query_map([from], |r| r.get(0)).map(|it| it.flatten().collect()).unwrap_or_default()
    })
}

// MARK: Oturumlar (Journal.swift)

pub const SESSION_GAP: f64 = 20.0 * 60.0;

#[derive(Clone, Debug)]
pub struct WorkSession {
    pub start: f64,
    pub end: f64,
    pub project: String,
    pub path: Option<String>,
    pub providers: Vec<String>,
    pub tokens: i64,
}

impl WorkSession {
    /// Ekranda gösterilen süre: ilk olaydan son olaya, en az 1 dakika.
    pub fn duration(&self) -> f64 { (self.end - self.start).max(60.0) }
}

/// Aynı projede, aralarında 20 dakikadan uzun boşluk olmayan olaylar bir oturumdur.
/// Her proje için ayrı açık oturum; projesiz olay en son çalışılan projeye yazılır.
pub fn sessions(from: f64, to: f64) -> Vec<WorkSession> {
    let q = format!("SELECT ts, provider, COALESCE(project,''), path, {} FROM events WHERE ts>=? AND ts<? ORDER BY ts", token_expr());
    let rows: Vec<(f64, String, String, Option<String>, i64)> = with(|c| {
        let Ok(mut st) = c.prepare(&q) else { return vec![] };
        st.query_map(params![from, to.min(1e11)], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
            .map(|it| it.flatten().collect())
            .unwrap_or_default()
    });
    let mut open: std::collections::HashMap<String, WorkSession> = Default::default();
    let mut done = Vec::new();
    let mut last_project = String::new();
    for (ts, prov, project, path, tokens) in rows {
        let key = if project.is_empty() { last_project.clone() } else { project.clone() };
        let path = path.filter(|p| !p.is_empty());
        match open.get_mut(&key) {
            Some(s) if ts - s.end <= SESSION_GAP => {
                s.end = ts;
                s.tokens += tokens;
                if !s.providers.contains(&prov) { s.providers.push(prov); }
                if s.path.is_none() { s.path = path; }
            }
            _ => {
                if let Some(s) = open.remove(&key) { done.push(s); }
                open.insert(key.clone(), WorkSession { start: ts, end: ts, project: key, path, providers: vec![prov], tokens });
            }
        }
        if !project.is_empty() { last_project = project; }
    }
    done.extend(open.into_values());
    done.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    done
}

// MARK: Kullanım sayfası sorguları (HistoryStore.series, modelSums, projectSums, exportCSV)

#[derive(Clone, Copy, Default)]
pub struct Sums { pub input: i64, pub output: i64, pub cache_write: i64, pub cache_read: i64 }

impl Sums {
    pub fn billable(&self) -> i64 {
        self.input + self.output + self.cache_write + if crate::settings::get().include_cache { self.cache_read } else { 0 }
    }
}

/// (kova, sağlayıcı, model, token). Kova yerel saatle "%Y-%m-%d %H:00" ya da "%Y-%m-%d".
pub fn series(since: f64, hourly: bool) -> Vec<(String, String, String, i64)> {
    let fmt = if hourly { "%Y-%m-%d %H:00" } else { "%Y-%m-%d" };
    let q = format!("SELECT strftime('{fmt}', ts, 'unixepoch', 'localtime') b, provider, COALESCE(model,''), SUM({})
                     FROM events WHERE ts>=? GROUP BY b, provider, model ORDER BY b", token_expr());
    with(|c| {
        let Ok(mut st) = c.prepare(&q) else { return vec![] };
        st.query_map([since], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).map(|it| it.flatten().collect()).unwrap_or_default()
    })
}

/// Model başına dört token türü (maliyet için).
pub fn model_sums(since: f64) -> Vec<(String, Sums)> {
    with(|c| {
        let Ok(mut st) = c.prepare("SELECT COALESCE(model,''), SUM(input), SUM(output), SUM(cache_write), SUM(cache_read) FROM events WHERE ts>=? GROUP BY model") else { return vec![] };
        st.query_map([since], |r| Ok((r.get(0)?, Sums { input: r.get::<_, Option<i64>>(1)?.unwrap_or(0), output: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
            cache_write: r.get::<_, Option<i64>>(3)?.unwrap_or(0), cache_read: r.get::<_, Option<i64>>(4)?.unwrap_or(0) })))
            .map(|it| it.flatten().collect()).unwrap_or_default()
    })
}

/// Proje ve model başına toplamlar; adı olmayanlar atlanır.
pub fn project_sums(since: f64) -> Vec<(String, String, Sums)> {
    with(|c| {
        let Ok(mut st) = c.prepare("SELECT project, COALESCE(model,''), SUM(input), SUM(output), SUM(cache_write), SUM(cache_read)
                                    FROM events WHERE ts>=? AND project IS NOT NULL AND project<>'' GROUP BY project, model") else { return vec![] };
        st.query_map([since], |r| Ok((r.get(0)?, r.get(1)?, Sums { input: r.get::<_, Option<i64>>(2)?.unwrap_or(0), output: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
            cache_write: r.get::<_, Option<i64>>(4)?.unwrap_or(0), cache_read: r.get::<_, Option<i64>>(5)?.unwrap_or(0) })))
            .map(|it| it.flatten().collect()).unwrap_or_default()
    })
}

/// Günlük, sağlayıcı ve model başına toplamlar (CSV).
pub fn export_csv() -> String {
    let mut out = String::from("date,provider,model,input,output,cache_write,cache_read\n");
    with(|c| {
        let Ok(mut st) = c.prepare("SELECT strftime('%Y-%m-%d', ts, 'unixepoch', 'localtime') d, provider, COALESCE(model,''),
                                    SUM(input), SUM(output), SUM(cache_write), SUM(cache_read) FROM events GROUP BY d, provider, model ORDER BY d, provider, model") else { return };
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
            r.get::<_, Option<i64>>(3)?.unwrap_or(0), r.get::<_, Option<i64>>(4)?.unwrap_or(0), r.get::<_, Option<i64>>(5)?.unwrap_or(0), r.get::<_, Option<i64>>(6)?.unwrap_or(0))));
        if let Ok(rows) = rows {
            for (d, p, m, a, b, cw, cr) in rows.flatten() {
                let q = |s: String| if s.contains(',') { format!("\"{s}\"") } else { s };
                out.push_str(&format!("{},{},{},{a},{b},{cw},{cr}\n", q(d), q(p), q(m)));
            }
        }
    });
    out
}

// MARK: Bilgisayar günlüğü (Mac: HistoryStore activity uzantısı)

pub fn insert_span(start: f64, bundle: &str, app: &str, title: Option<&str>, domain: Option<&str>, category: &str) -> i64 {
    with(|c| {
        let _ = c.execute("INSERT INTO activity(start,end,bundle,app,title,domain,category) VALUES(?,?,?,?,?,?,?)",
            params![start, start, bundle, app, title, domain, category]);
        c.last_insert_rowid()
    })
}

pub fn set_span_end(id: i64, end: f64) {
    with(|c| { let _ = c.execute("UPDATE activity SET end=? WHERE id=?", params![end, id]); });
}

pub struct Span { pub start: f64, pub end: f64, pub bundle: String, pub app: String, pub title: Option<String>, pub domain: Option<String>, pub category: String }

/// Aralıktaki kayıtlar; aralık dışına taşan kısım kırpılır.
pub fn spans(from: f64, to: f64) -> Vec<Span> {
    with(|c| {
        let Ok(mut st) = c.prepare("SELECT start,end,bundle,app,title,domain,category FROM activity WHERE end>? AND start<? ORDER BY start") else { return vec![] };
        st.query_map(params![from, to], |r| Ok(Span {
            start: r.get::<_, f64>(0)?.max(from), end: r.get::<_, f64>(1)?.min(to),
            bundle: r.get::<_, Option<String>>(2)?.unwrap_or_default(), app: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
            title: r.get(4)?, domain: r.get(5)?, category: r.get::<_, Option<String>>(6)?.unwrap_or_else(|| "Diğer".into()),
        })).map(|it| it.flatten().filter(|s| s.end > s.start).collect()).unwrap_or_default()
    })
}

/// Uygulama başına toplam süre (raporlar için).
pub fn app_totals(from: f64, to: f64) -> Vec<(String, String, String, f64)> {
    with(|c| {
        let Ok(mut st) = c.prepare("SELECT bundle, MAX(app), MAX(category), SUM(MIN(end, ?2) - MAX(start, ?1)) t
            FROM activity WHERE end > ?1 AND start < ?2 AND end - start >= 3 GROUP BY bundle ORDER BY t DESC") else { return vec![] };
        st.query_map(params![from, to], |r| Ok((r.get::<_, Option<String>>(0)?.unwrap_or_default(), r.get::<_, Option<String>>(1)?.unwrap_or_default(),
            r.get::<_, Option<String>>(2)?.unwrap_or_default(), r.get::<_, Option<f64>>(3)?.unwrap_or(0.0))))
            .map(|it| it.flatten().collect()).unwrap_or_default()
    })
}

pub fn add_file_counts(counts: &std::collections::HashMap<String, i64>, hour: f64) {
    with(|c| {
        for (p, n) in counts {
            let _ = c.execute("INSERT INTO file_activity(hour,project,count) VALUES(?,?,?) ON CONFLICT(hour,project) DO UPDATE SET count=count+excluded.count", params![hour, p, n]);
        }
    });
}

pub fn file_activity(from: f64, to: f64) -> Vec<(String, i64)> {
    with(|c| {
        let Ok(mut st) = c.prepare("SELECT project, SUM(count) c FROM file_activity WHERE hour>=? AND hour<? GROUP BY project ORDER BY c DESC") else { return vec![] };
        st.query_map(params![from, to], |r| Ok((r.get(0)?, r.get(1)?))).map(|it| it.flatten().collect()).unwrap_or_default()
    })
}

/// Bilgisayar günlüğünü siler.
pub fn delete_computer_log() { with(|c| { let _ = c.execute_batch("DELETE FROM activity; DELETE FROM file_activity;"); }); }

/// Commit'leri okunan proje kökleri (ts ≥ since).
pub fn recent_roots(since: f64) -> Vec<String> {
    with(|c| {
        let Ok(mut st) = c.prepare("SELECT DISTINCT path FROM events WHERE path IS NOT NULL AND path<>'' AND ts>=?") else { return vec![] };
        st.query_map([since], |r| r.get(0)).map(|it| it.flatten().collect()).unwrap_or_default()
    })
}

/// Token kullanımının API fiyatıyla karşılığı (milyon token başına dolar; Mac: Pricing.swift).
pub fn cost(model: &str, s: &Sums) -> f64 {
    let m = model.to_lowercase();
    let r: (f64, f64, f64, f64) = if m.contains("opus") { (4.0, 20.0, 5.0, 0.2) }
        else if m.contains("sonnet") { (2.0, 10.0, 2.5, 0.1) }
        else if m.contains("haiku") { (0.1, 0.5, 0.125, 0.01) }
        else if m.contains("astra") { (10.0, 50.0, 12.5, 1.0) }
        else if m.contains("luna") { (0.2, 1.0, 0.2, 0.02) }
        else if m.contains("sol") || m.contains("terra") || m.starts_with("gpt") || m.contains("codex") { (2.0, 10.0, 2.0, 0.2) }
        else if m.contains("gemini") { (0.3, 2.5, 0.3, 0.03) }
        else { return 0.0 };
    (s.input as f64 * r.0 + s.output as f64 * r.1 + s.cache_write as f64 * r.2 + s.cache_read as f64 * r.3) / 1e6
}

/// Boş toplam Rust'ta -0.0 döner ("$-0.00" yazmasın diye sıfıra çekilir).
pub fn format_usd(v: f64) -> String { let v = v.max(0.0) + 0.0; if v >= 100.0 { format!("${v:.0}") } else { format!("${v:.2}") } }
// MARK: Zaman yardımcıları

/// Yerel günün başı (unix sn).
pub fn start_of_day(t: f64) -> f64 {
    use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
    let ft100 = ((t as u64) + 11_644_473_600) * 10_000_000;
    let ft = FILETIME { dwLowDateTime: ft100 as u32, dwHighDateTime: (ft100 >> 32) as u32 };
    let (mut st, mut lt) = (SYSTEMTIME::default(), SYSTEMTIME::default());
    unsafe {
        let _ = FileTimeToSystemTime(&ft, &mut st);
        let _ = SystemTimeToTzSpecificLocalTime(None, &st, &mut lt);
    }
    let since_midnight = lt.wHour as f64 * 3600.0 + lt.wMinute as f64 * 60.0 + lt.wSecond as f64 + (t.fract());
    t - since_midnight
}

/// ISO 8601 (ör. 2026-11-17T12:34:56.789Z ya da +03:00) → unix sn.
pub fn parse_iso(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    if b.len() < 19 { return None; }
    let n = |a: usize, l: usize| s.get(a..a + l)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, se) = (n(0, 4)?, n(5, 2)?, n(8, 2)?, n(11, 2)?, n(14, 2)?, n(17, 2)?);
    let mut i = 19;
    let mut frac = 0.0;
    if b.get(i) == Some(&b'.') {
        let st = i + 1;
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() { i += 1; }
        frac = format!("0.{}", &s[st..i]).parse().unwrap_or(0.0);
    }
    let mut off = 0i64;
    match b.get(i) {
        Some(b'+') | Some(b'-') => {
            let sign = if b[i] == b'+' { 1 } else { -1 };
            let oh = n(i + 1, 2).unwrap_or(0);
            let om = s.get(i + 4..i + 6).and_then(|x| x.parse::<i64>().ok()).or_else(|| n(i + 3, 2)).unwrap_or(0);
            off = sign * (oh * 3600 + om * 60);
        }
        _ => {}
    }
    // Gün sayısı (Howard Hinnant'ın civil_from_days tersi)
    let y2 = if mo <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some((days * 86400 + h * 3600 + mi * 60 + se - off) as f64 + frac)
}

#[cfg(test)]
mod tests {
    #[test]
    fn iso() {
        assert_eq!(super::parse_iso("1970-01-01T00:00:00Z"), Some(0.0));
        assert_eq!(super::parse_iso("2026-11-17T12:00:00.500Z"), Some(1794916800.5));
        assert_eq!(super::parse_iso("2026-11-17T15:00:00+03:00"), Some(1794916800.0));
    }
}
