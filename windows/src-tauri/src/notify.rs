//! Bildirimler (Mac: Automation.swift → Notifier, WeeklySummary). Ana süreçte.
//! Windows'ta tepsi simgesinin bilgi balonu kullanılır; Windows 10/11 bunu normal bildirim olarak gösterir,
//! Odaklanma/Rahatsız Etme ayarlarına uyar. Ek kayıt, kısayol ya da kütüphane gerekmez.

use crate::data::{Confidence, ProviderUsage, Snapshot};
use crate::{cat::format_tokens, l10n, look, sense, settings, store};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::core::{w, PCWSTR};
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const SEEN_KEY: &str = "notifiedKeys";
const WEEK_KEY: &str = "weeklySummaryWeek";

fn copy(dst: &mut [u16], s: &str) {
    let n = dst.len() - 1;
    for (i, c) in s.encode_utf16().take(n).enumerate() { dst[i] = c; }
}

fn big_icon() -> HICON {
    unsafe {
        let hinst = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap_or_default();
        for id in [32512usize, 1] {
            if let Ok(i) = LoadImageW(Some(hinst.into()), PCWSTR(id as *const u16), IMAGE_ICON,
                GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON), LR_DEFAULTCOLOR) {
                return HICON(i.0);
            }
        }
        HICON::default()
    }
}

/// Bildirim gönderir (sesli). Tıklanınca Kullanım sayfası açılır (tray.rs → NIN_BALLOONUSERCLICK).
pub fn send(title: &str, body: &str) {
    crate::cat::dev_log("bildirim gönderildi");
    unsafe {
        let hwnd = FindWindowW(w!("MeowlyMain"), None).unwrap_or_default();
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_INFO,
            dwInfoFlags: NIIF_USER | NIIF_LARGE_ICON,
            hBalloonIcon: big_icon(),
            ..Default::default()
        };
        copy(&mut nid.szInfoTitle, title);
        copy(&mut nid.szInfo, body);
        let ok = Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool();
        crate::cat::dev_log(&format!("bildirim sonucu ok={ok} hwnd={:?}", hwnd.0));
    }
}

fn last() -> &'static Mutex<Option<Snapshot>> {
    static L: OnceLock<Mutex<Option<Snapshot>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(None))
}

/// Her veri okumasından sonra (veri iş parçacığında) çağrılır.
pub fn after_read(new: &Snapshot) {
    let s = settings::get();
    let old = last().lock().unwrap().replace(new.clone());
    if !s.notifications_on { return; }
    if let Some(old) = old { check(&old, new); }
    if s.weekly_summary { weekly_if_due(sense::now_secs()); }
}

/// Mac: Usage.rows — ek satırlar varsa onlar (günlük kota, havuzlar), yoksa 5 saatlik ve haftalık.
fn rows(u: &ProviderUsage) -> Vec<(String, f64, f64)> {
    if !u.extra.is_empty() { return u.extra.iter().map(|r| (r.title.clone(), r.left, r.reset.unwrap_or(0.0))).collect(); }
    let Some(l) = &u.limits else { return vec![] };
    let mut r = vec![];
    if let Some(v) = l.five_left { r.push(("5 saatlik".to_string(), v, l.five_reset.unwrap_or(0.0))); }
    if let Some(v) = l.week_left { r.push(("Haftalık".to_string(), v, l.week_reset.unwrap_or(0.0))); }
    r
}

/// Eşikler pencere başına bir kez bildirilir; limit yenilenince ayrıca haber verilir.
pub fn check(old: &Snapshot, new: &Snapshot) {
    let mut seen: Vec<String> = store::kv_get(SEEN_KEY).and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default();
    let before = seen.len();
    for id in new.ids() {
        let name = crate::look::provider_name(id);
        let Some(n) = new.usage(id) else { continue };
        let o = old.usage(id);
        if n.confidence == Confidence::None { continue; }
        let orows = o.map(rows).unwrap_or_default();
        for (title, left, reset) in rows(n) {
            let base = format!("{}|{}|{}", name.to_lowercase(), title, reset as i64);
            for (level, label) in [(0.05, "%5"), (0.15, "%15")] {
                if left > level { continue; }
                let key = format!("{base}|{level}");
                if seen.contains(&key) { break; }
                // Daha düşük eşik geçildiyse üsttekini de işaretle ki ikinci bildirim gelmesin.
                for l2 in [0.05, 0.15] { if l2 >= level { let k = format!("{base}|{l2}"); if !seen.contains(&k) { seen.push(k); } } }
                send(&l10n::f("%@: limit azaldı", &[name.into()]),
                     &l10n::f("%@ için kalan %@ altına düştü (%lld%%).", &[l10n::t(&title), label.into(), ((left * 100.0).round() as i64).to_string()]).replace("%%", "%"));
                break;
            }
            if let Some(&(_, ol, _)) = orows.iter().find(|r| r.0 == title) {
                if ol <= 0.2 && left >= 0.9 {
                    send(&l10n::f("%@: limit yenilendi", &[name.into()]),
                         &l10n::f("%@ penceresi sıfırlandı, %%%lld kaldı.", &[l10n::t(&title), ((left * 100.0).round() as i64).to_string()]).replace("%%", "%"));
                }
            }
        }
    }
    if seen.len() != before {
        let from = seen.len().saturating_sub(300);
        store::kv_set(SEEN_KEY, &serde_json::to_string(&seen[from..]).unwrap_or_default());
    }
}

fn weekday_name(wd: u32) -> String {
    const TR: [&str; 7] = ["Pazar", "Pazartesi", "Salı", "Çarşamba", "Perşembe", "Cuma", "Cumartesi"];
    const EN: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
    (if l10n::english() { EN } else { TR })[wd as usize % 7].to_string()
}

/// Pazartesi 09:00'dan sonra, her hafta bir kez: geçen Pazartesi-Pazar arası özeti.
pub fn weekly_if_due(now: f64) {
    let (_, _, _, h, _, wd) = look::local(now);
    if wd != 1 || h < 9 { return; }
    let this_mon = store::start_of_day(now);
    let stamp = (this_mon as i64).to_string();
    if store::kv_get(WEEK_KEY).as_deref() == Some(stamp.as_str()) { return; }
    store::kv_set(WEEK_KEY, &stamp);
    if let Some(text) = weekly_text(this_mon) { send(&l10n::t("Meowly haftalık özet"), &text); }
}

fn weekly_text(this_mon: f64) -> Option<String> {
    let last_mon = store::start_of_day(this_mon - 7.0 * 86400.0 + 3.0 * 3600.0);
    let (y, m, d, ..) = look::local(this_mon + 3600.0);
    let end_key = format!("{y:04}-{m:02}-{d:02}");
    let pts: Vec<_> = store::series(last_mon, false).into_iter().filter(|p| p.0 < end_key).collect();
    let total: i64 = pts.iter().map(|p| p.3).sum();
    if total <= 0 { return None; }
    let mut by_day: HashMap<String, i64> = HashMap::new();
    let mut by_prov: HashMap<String, i64> = HashMap::new();
    for p in &pts { *by_day.entry(p.0.clone()).or_default() += p.3; *by_prov.entry(p.1.clone()).or_default() += p.3; }
    let mut text = l10n::f("Geçen hafta %@ token.", &[format_tokens(total)]);
    if let Some((k, v)) = by_day.iter().max_by_key(|x| *x.1) {
        // Günün adı: kova anahtarındaki tarihin öğle saati.
        let (yy, mm, dd) = (k[0..4].parse().unwrap_or(1970), k[5..7].parse().unwrap_or(1), k[8..10].parse().unwrap_or(1));
        let wd = look::local(crate::reports::local_midnight(yy, mm, dd) + 12.0 * 3600.0).5;
        text += &l10n::f(" En yoğun gün %@ (%@).", &[weekday_name(wd), format_tokens(*v)]);
    }
    if let Some((p, _)) = by_prov.iter().max_by_key(|x| *x.1) {
        let name = match p.as_str() { "claude" => "Claude", "codex" => "Codex", o => o };
        text += &l10n::f(" En çok %@.", &[name.to_string()]);
    }
    Some(text)
}

/// Geliştirme: `meowly.exe --notify-test` (çalışan uygulama varken) üç örnek bildirimi sırayla gösterir.
pub fn demo() {
    let now = sense::now_secs();
    let this_mon = store::start_of_day(now) - ((look::local(now).5 + 6) % 7) as f64 * 86400.0;
    let samples = [
        (l10n::f("%@: limit azaldı", &["Claude".into()]), l10n::f("%@ için kalan %@ altına düştü (%lld%%).", &[l10n::t("5 saatlik"), "%15".into(), "12".into()]).replace("%%", "%")),
        (l10n::f("%@: limit yenilendi", &["Claude".into()]), l10n::f("%@ penceresi sıfırlandı, %%%lld kaldı.", &[l10n::t("5 saatlik"), "100".into()]).replace("%%", "%")),
        (l10n::t("Meowly haftalık özet"), weekly_text(this_mon).unwrap_or_else(|| l10n::f("Geçen hafta %@ token.", &["0".into()]))),
    ];
    for (t, b) in samples { send(&t, &b); std::thread::sleep(std::time::Duration::from_secs(6)); }
}
