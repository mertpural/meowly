//! Görsel dil ve ortak hesaplar (Mac: TontiUI.swift → TC, Models.swift → statusColor/formatRemaining, Theme.swift → Pace).

use crate::data::{Limits, ProviderUsage};
use crate::{l10n, sense, settings};

pub type Rgb = (u8, u8, u8);

fn c(l: (f64, f64, f64), d: (f64, f64, f64)) -> Rgb {
    let v = if settings::get().dark { d } else { l };
    ((v.0 * 255.0).round() as u8, (v.1 * 255.0).round() as u8, (v.2 * 255.0).round() as u8)
}

pub fn ink() -> Rgb { c((0.086, 0.047, 0.047), (0.94, 0.94, 0.93)) }
pub fn sub() -> Rgb { c((0.54, 0.50, 0.49), (0.62, 0.62, 0.65)) }
pub const PINK: Rgb = (232, 143, 163);
pub const AMBER: Rgb = (207, 140, 41);

/// Tonti dili: yeterli hak mürekkep renginde, azalınca kehribar, bitmeye yakın pembe.
pub fn status_color(left: f64) -> Rgb {
    if left > 0.4 { ink() } else if left > 0.15 { AMBER } else { PINK }
}

pub fn hex(c: Rgb) -> String { format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2) }

/// Sağlayıcının Tonti rengi (açık/koyu).
pub fn provider_color(p: &str) -> Rgb {
    match p {
        "claude" => c((0.85, 0.47, 0.31), (0.93, 0.55, 0.38)),
        "codex" => c((0.32, 0.30, 0.33), (0.72, 0.72, 0.72)),
        "gemini" => (120, 158, 214),
        "antigravity" => c((0.20, 0.29, 0.62), (0.42, 0.53, 0.88)),
        "opencode" => (120, 184, 143),
        "kimi" => (163, 135, 214),
        "cline" => (219, 194, 102),
        _ => sub(),
    }
}

pub fn provider_name(p: &str) -> &'static str {
    match p { "claude" => "Claude", "codex" => "Codex", "gemini" => "Gemini", "antigravity" => "Antigravity", "opencode" => "OpenCode", "kimi" => "Kimi", "cline" => "Cline", _ => "?" }
}

/// "2s 21dk" / "2g 18s" / "5dk".
pub fn format_remaining(at: f64) -> String {
    let s = (at - sense::now_secs()) as i64;
    if s <= 0 { return l10n::t("şimdi"); }
    let (d, h, m) = (s / 86400, (s % 86400) / 3600, (s % 3600) / 60);
    let u = if l10n::english() { ("d", "h", "m") } else { ("g", "s", "dk") };
    if d > 0 { return format!("{d}{} {h}{}", u.0, u.1); }
    if h > 0 { return format!("{h}{} {m}{}", u.1, u.2); }
    format!("{m}{}", u.2)
}

pub fn reset_text(reset: Option<f64>) -> String {
    match reset {
        None => String::new(),
        Some(r) if r > sense::now_secs() => l10n::f("Sıfırlanma %@", &[format_remaining(r)]),
        Some(_) => l10n::t("Pencere sıfırlandı"),
    }
}

/// Pencerede geçen süreye göre beklenen kalan oran (çubuktaki işaret).
pub fn expected_left(left: f64, reset: Option<f64>, window: f64) -> Option<f64> {
    let _ = left;
    let r = reset?;
    let now = sense::now_secs();
    if r <= now { return None; }
    let remaining = r - now;
    if remaining > window + 60.0 { return None; }
    let elapsed = (1.0 - remaining / window).clamp(0.0, 1.0);
    Some(1.0 - elapsed)
}

/// Şimdiki harcama hızı sürerse limitin biteceği an; pencere bitmeden dolmayacaksa None.
pub fn exhaustion(left: f64, reset: Option<f64>, window: f64) -> Option<f64> {
    let r = reset?;
    let now = sense::now_secs();
    if r <= now || left <= 0.001 { return None; }
    let remaining = r - now;
    if remaining > window + 60.0 { return None; }
    let elapsed = window - remaining;
    let used = 1.0 - left;
    if elapsed / window < 0.08 || used < 0.03 { return None; }
    let at = now + left / (used / elapsed);
    (at < r - 60.0).then_some(at)
}

/// Saat bugünse yalnızca "HH:mm", değilse gün kısaltmasıyla.
pub fn when_text(t: f64) -> String {
    let (y, mo, d, h, mi, wd) = local(t);
    let (y0, mo0, d0, ..) = local(sense::now_secs());
    if (y, mo, d) == (y0, mo0, d0) { return format!("{h:02}:{mi:02}"); }
    let days_tr = ["Paz", "Pzt", "Sal", "Çar", "Per", "Cum", "Cmt"];
    let days_en = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let n = if l10n::english() { days_en[wd as usize % 7] } else { days_tr[wd as usize % 7] };
    format!("{n} {h:02}:{mi:02}")
}

/// Yerel tarih-saat: (yıl, ay, gün, saat, dakika, haftanın günü 0=Pazar).
pub fn local(t: f64) -> (i32, u32, u32, u32, u32, u32) {
    use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
    let ft100 = ((t.max(0.0) as u64) + 11_644_473_600) * 10_000_000;
    let ft = FILETIME { dwLowDateTime: ft100 as u32, dwHighDateTime: (ft100 >> 32) as u32 };
    let (mut st, mut lt) = (SYSTEMTIME::default(), SYSTEMTIME::default());
    unsafe {
        let _ = FileTimeToSystemTime(&ft, &mut st);
        let _ = SystemTimeToTzSpecificLocalTime(None, &st, &mut lt);
    }
    (lt.wYear as i32, lt.wMonth as u32, lt.wDay as u32, lt.wHour as u32, lt.wMinute as u32, lt.wDayOfWeek as u32)
}

/// Kartın altındaki notlar: limitin ne zaman biteceği.
pub fn footnotes(p: &str, u: &ProviderUsage) -> Vec<String> {
    let mut out = Vec::new();
    let s = settings::get();
    if s.show_forecast && u.stale_since.is_none() {
        if let Some(l) = &u.limits {
            let mut parts = Vec::new();
            if let Some(at) = l.five_left.and_then(|f| exhaustion(f, l.five_reset, 5.0 * 3600.0)) {
                parts.push(l10n::f("5 saatlik %@", &[when_text(at)]));
            }
            if let Some(at) = l.week_left.and_then(|w| exhaustion(w, l.week_reset, 7.0 * 86400.0)) {
                parts.push(l10n::f("haftalık %@", &[when_text(at)]));
            }
            if !parts.is_empty() { out.push(l10n::t("Bu hızla biter: ") + &parts.join(", ")); }
        }
    }
    if p == "claude" && s.show_next_start && s.starter_on {
        if let Some(at) = crate::starter::fire_at() { out.push(l10n::f("Sonraki otomatik mesaj: %@", &[when_text(at)])); }
    }
    out
}

/// Limit yalnızca güveni "yok" değilse gösterilir.
pub fn shown_limits(u: &ProviderUsage) -> Option<&Limits> {
    if u.confidence == crate::data::Confidence::None { None } else { u.limits.as_ref() }
}
