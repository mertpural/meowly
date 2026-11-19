//! Raporlar (Mac: Tonti/Reports.swift → ReportPeriod, ReportData, ReportsPage). Arayüz sürecinde çalışır.

use crate::cat::format_tokens;
use crate::diary::{commits, duration, nearby_repos, Commit};
use crate::store;
use crate::{l10n, look, sense, settings};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Yerel takvim tarihinin gece yarısı (unix sn).
pub fn local_midnight(y: i32, m: u32, d: u32) -> f64 {
    use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows::Win32::System::Time::{SystemTimeToFileTime, TzSpecificLocalTimeToSystemTime};
    // Ay/gün taşmalarını düzelt (ör. 13. ay → gelecek yılın ocağı, 31 Kasım → 1 Aralık). Windows geçersiz bir tarihi
    // (31 Kasım) çeviremez; düzeltilmezse 1601'e döner ve aylık rapor döngüsü hiç bitmiyordu.
    let (y, m, d) = normalize(y, m as i64, d as i64);
    let lt = SYSTEMTIME { wYear: y as u16, wMonth: m as u16, wDay: d as u16, ..Default::default() };
    let mut st = SYSTEMTIME::default();
    let mut ft = FILETIME::default();
    unsafe {
        let _ = TzSpecificLocalTimeToSystemTime(None, &lt, &mut st);
        let _ = SystemTimeToFileTime(&st, &mut ft);
    }
    let v = ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64;
    (v / 10_000_000) as f64 - 11_644_473_600.0
}

/// (yıl, ay, gün) üçlüsünü geçerli bir tarihe çevirir: ay 1–12 dışına, gün ayın sınırları dışına taşabilir.
pub fn normalize(y: i32, m: i64, d: i64) -> (i32, u32, u32) {
    let (mut y, mut m) = (y as i64, m);
    y += (m - 1).div_euclid(12);
    m = (m - 1).rem_euclid(12) + 1;
    civil_from_days(days_from_civil(y, m, 1) + d - 1)
}

/// 1970-01-01'den beri gün (proleptik Gregoryen; H. Hinnant'ın algoritması).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    ((yoe + era * 400 + if m <= 2 { 1 } else { 0 }) as i32, m as u32, d as u32)
}

/// `t` anını içeren dönem: (başlangıç, bitiş). Hafta pazartesi başlar.
pub fn period(kind: &str, t: f64) -> (f64, f64) {
    let (y, m, d, _, _, wd) = look::local(t);
    match kind {
        "day" => { let s = local_midnight(y, m, d); (s, local_midnight(y, m, d + 1).max(s + 82800.0)) }
        "week" => {
            let back = (wd + 6) % 7; // pazartesiye kaç gün
            let s = store::start_of_day(t - back as f64 * 86400.0);
            let (y2, m2, d2, ..) = look::local(s + 3600.0);
            (s, local_midnight(y2, m2, d2 + 7))
        }
        "month" => (local_midnight(y, m, 1), local_midnight(y, m + 1, 1)),
        _ => (local_midnight(y, 1, 1), local_midnight(y + 1, 1, 1)),
    }
}

fn shift(kind: &str, start: f64, n: i32) -> f64 {
    let (y, m, d, ..) = look::local(start + 3600.0);
    match kind {
        "day" => { let _ = (y, m, d); start + n as f64 * 86400.0 + 3600.0 }
        "week" => start + n as f64 * 7.0 * 86400.0 + 3600.0,
        "month" => local_midnight(y, (m as i32 + n) as u32, 1),
        _ => local_midnight(y + n, 1, 1),
    }
}

fn per_project(kind: &str) -> usize { match kind { "day" => 8, "week" => 5, "month" => 4, _ => 3 } }

/// Rapor verisi. `t`: bakılan an; `shift_by`: ‹ › ile kaç dönem kaydırılacağı.
#[tauri::command]
pub async fn report_page(kind: String, t: f64, shift_by: i32) -> Value {
    let (mut start, mut end) = period(&kind, t);
    if shift_by != 0 {
        let moved = if kind == "day" { start + shift_by as f64 * 86400.0 + 3600.0 } else { shift(&kind, start, shift_by) };
        let (s2, e2) = period(&kind, moved);
        // Geleceğe gidilmez.
        if s2 <= sense::now_secs() { start = s2; end = e2; }
    }
    let ss = store::sessions(start, end);
    let active: f64 = ss.iter().map(|s| s.duration()).sum();
    let tokens: i64 = ss.iter().map(|s| s.tokens).sum();
    let mut by: BTreeMap<String, (f64, i64, Option<String>)> = BTreeMap::new();
    for s in ss.iter().filter(|s| !s.project.is_empty()) {
        let e = by.entry(s.project.clone()).or_default();
        e.0 += s.duration(); e.1 += s.tokens;
        if e.2.is_none() { e.2 = s.path.clone(); }
    }
    let mut projects: Vec<(String, f64, i64, Vec<Commit>)> = by.iter()
        .map(|(n, (tm, tk, p))| (n.clone(), *tm, *tk, p.as_ref().map(|p| commits(p, start, end)).unwrap_or_default()))
        .collect();
    // Oturumu olmayan ama commit atılmış depolar da çıkanlara girer (süresiz).
    let known: Vec<String> = by.values().filter_map(|v| v.2.clone()).map(|p| p.to_lowercase().replace('/', "\\")).collect();
    for p in nearby_repos() {
        if known.contains(&p.to_lowercase().replace('/', "\\")) { continue; }
        let c = commits(&p, start, end);
        if c.is_empty() { continue; }
        let name = Path::new(&p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match projects.iter_mut().find(|x| x.0 == name) {
            Some(x) => if x.3.is_empty() { x.3 = c; },
            None => projects.push((name, 0.0, 0, c)),
        }
    }
    projects.sort_by(|a, b| (b.3.len(), b.1).partial_cmp(&(a.3.len(), a.1)).unwrap());

    // Günlük çalışma süreleri: en yoğun gün ve grafik.
    let mut per_day: HashMap<i64, f64> = HashMap::new();
    for s in &ss { *per_day.entry(store::start_of_day(s.start) as i64).or_default() += s.duration(); }
    let busiest = per_day.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(d, t)| json!({ "day": d, "dur": duration(*t) }));
    let mut buckets = Vec::new();
    match kind.as_str() {
        "week" | "month" => {
            let mut d = start;
            // En çok 32 gün: tarih hesabı bir gün yanlış dönse bile döngü sonsuza gitmesin.
            while d < end - 3600.0 && buckets.len() < 32 {
                buckets.push(json!({ "t": d, "v": per_day.get(&(d as i64)).copied().unwrap_or(0.0) }));
                let (y, m, dd, ..) = look::local(d + 3600.0);
                let next = local_midnight(y, m, dd + 1);
                if next <= d { break; }
                d = next;
            }
        }
        "year" => {
            let (y, ..) = look::local(start + 3600.0);
            for mo in 1..=12u32 {
                let (a, b) = (local_midnight(y, mo, 1), local_midnight(y, mo + 1, 1));
                let v: f64 = per_day.iter().filter(|(k, _)| (**k as f64) >= a - 1.0 && (**k as f64) < b - 1.0).map(|(_, v)| v).sum();
                buckets.push(json!({ "t": a, "v": v }));
            }
        }
        _ => {}
    }
    let pp = per_project(&kind);
    let s = settings::get();
    json!({
        "dark": s.dark,
        "en": l10n::english(),
        "kind": kind,
        "start": start,
        "end": end,
        "canNext": period(&kind, end + 3600.0).0 <= sense::now_secs(),
        "projects": projects.iter().map(|(n, tm, tk, c)| json!({
            "name": n,
            "right": if *tm > 0.0 { duration(*tm) } else { l10n::f("%lld commit", &[c.len().to_string()]) },
            "commits": c.iter().rev().map(|c| c.subject.clone()).collect::<Vec<_>>(),
            "tokens": format!("{} token", format_tokens(*tk)),
        })).collect::<Vec<_>>(),
        "perProject": pp,
        "active": duration(active),
        "activeSecs": active,
        "commits": projects.iter().map(|p| p.3.len()).sum::<usize>(),
        "sessions": ss.len(),
        "projectCount": projects.len(),
        "tokens": format_tokens(tokens),
        "busiest": busiest,
        "buckets": buckets,
        "computer": crate::computer::summary(start, end.min(sense::now_secs())),
    })
}

/// Rapor resmini Windows'un kaydet penceresiyle kaydeder.
#[tauri::command]
pub async fn save_png(name: String, data: Vec<u8>) -> bool {
    match crate::win::save_dialog(&name, "PNG (*.png)\0*.png\0\0") {
        Some(p) => std::fs::write(p, data).is_ok(),
        None => false,
    }
}

#[cfg(test)]
mod date_tests {
    use super::*;
    #[test]
    fn normalize_overflow() {
        assert_eq!(normalize(2026, 11, 31), (2026, 12, 1));
        assert_eq!(normalize(2026, 12, 32), (2027, 1, 1));
        assert_eq!(normalize(2026, 13, 1), (2027, 1, 1));
        assert_eq!(normalize(2026, 0, 1), (2025, 12, 1));
        assert_eq!(normalize(2028, 2, 30), (2028, 3, 1));
        assert_eq!(normalize(2026, 3, 0), (2026, 2, 28));
        assert_eq!(normalize(2026, 11, 37), (2026, 12, 7));
    }
    #[test]
    fn every_day_of_two_years() {
        let mut t = local_midnight(2026, 1, 1) + 7200.0;
        for _ in 0..731 {
            let (ds, de) = period("day", t);
            assert!(de - ds >= 82800.0 && de - ds <= 90000.0);
            let (ws, we) = period("week", t);
            assert!(we - ws >= 6.9 * 86400.0 && we - ws <= 7.1 * 86400.0, "{t}");
            let (ms, me) = period("month", t);
            assert!(ms <= t && t < me);
            let sh = shift("week", ws, -1);
            assert!(period("week", sh).0 < ws);
            t += 86400.0;
        }
    }
    #[test]
    fn every_month_terminates() {
        for y in [2025, 2026, 2028] { for m in 1..=12u32 {
            let t = local_midnight(y, m, 15);
            let (s, e) = period("month", t);
            assert!(e > s && e - s < 32.0 * 86400.0, "{y}-{m}");
            let (ws, we) = period("week", t + 15.0 * 86400.0);
            assert!(we - ws > 6.0 * 86400.0 && we - ws < 8.0 * 86400.0);
            assert!(shift("month", s, 1) > s && shift("month", s, -1) < s);
        } }
    }
}
