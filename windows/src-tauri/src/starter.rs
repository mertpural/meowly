//! Claude pencere başlatıcı (Mac: Automation.swift → WindowStarter, Models.swift → scheduleStarter). Ana süreçte.
//! 5 saatlik pencere, hesabın pencere dışındaki ilk isteğiyle başlar. Pencere bittiğinde Haiku'ya çok kısa bir istek
//! gönderip yeni pencereyi erken başlatır. Ek limit vermez. Yalnızca kullanıcı açarsa çalışır.
//! Sürekli yoklamaz: sıfırlanma saati limit bilgisinden okunur, o ana tek zamanlayıcı kurulur, o anda tek istek gider.

use crate::data::{Confidence, ProviderUsage};
use crate::{claude, l10n, look, sense, settings, store};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub const K_LAST: &str = "starterLast";
const K_TRY: &str = "starterTry";
/// Arayüz süreci okusun diye: kurulu zamanlayıcının anı ve son denemenin durumu.
pub const K_FIRE: &str = "starterFireAt";
pub const K_STATUS: &str = "starterStatus";

static GEN: AtomicU64 = AtomicU64::new(0);
static FIRE_AT: Mutex<Option<f64>> = Mutex::new(None);
static RUNNING: Mutex<bool> = Mutex::new(false);

/// Zamanlayıcının çalışacağı an: sıfırlanmadan 15 sn sonra. Değer eski (tahmini) olsa da sıfırlanma saati değişmez.
fn next_fire(u: &ProviderUsage, now: f64) -> Option<f64> {
    if u.confidence == Confidence::None { return None; }
    let r = u.limits.as_ref()?.five_reset?;
    (r > now).then_some(r + 15.0)
}

/// Sıfırlanma çoktan geçmişse (uygulama geç açıldıysa): 15 dk'da en çok bir deneme, 4,5 saatte en çok bir başarı.
fn should_start(u: &ProviderUsage, now: f64) -> bool {
    if u.confidence == Confidence::None { return false; }
    let Some(l) = &u.limits else { return false };
    if l.five_left.unwrap_or(0.0) < 0.999 { return false; }
    if l.five_reset.map(|r| r > now).unwrap_or(false) { return false; }
    now - store::kv_f64(K_TRY) > 15.0 * 60.0 && now - store::kv_f64(K_LAST) > 4.5 * 3600.0
}

pub fn fire_at() -> Option<f64> { *FIRE_AT.lock().unwrap() }

fn set_fire(v: Option<f64>) {
    *FIRE_AT.lock().unwrap() = v;
    store::kv_set(K_FIRE, &v.map(|x| x.to_string()).unwrap_or_default());
}

/// Her veri okumasından sonra (veri iş parçacığında). Yalnızca saat değiştiyse zamanlayıcı yeniden kurulur.
pub fn schedule(claude: Option<&ProviderUsage>) {
    if !settings::get().starter_on {
        if fire_at().is_some() { GEN.fetch_add(1, Ordering::SeqCst); set_fire(None); }
        return;
    }
    let Some(u) = claude else { return };
    let now = sense::now_secs();
    // Geliştirme: MEOWLY_STARTER_IN=<sn> zamanlayıcıyı şimdiden o kadar sonraya kurar (bir kez).
    let dev_at = std::env::var("MEOWLY_STARTER_IN").ok().and_then(|v| v.parse::<f64>().ok()).map(|s| {
        static AT: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
        *AT.get_or_init(|| now + s)
    }).filter(|a| *a > now);
    if let Some(at) = dev_at.or_else(|| next_fire(u, now)) {
        if fire_at().map(|c| (c - at).abs() < 60.0).unwrap_or(false) { return; }
        let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
        set_fire(Some(at));
        std::thread::spawn(move || {
            // Duvar saatine göre bekler: bilgisayar uyurken geçen süre de sayılsın (yalnızca bekleyen zamanlayıcı varken).
            loop {
                if GEN.load(Ordering::SeqCst) != gen { return; }
                let left = at - sense::now_secs();
                if left <= 0.0 { break; }
                std::thread::sleep(std::time::Duration::from_secs_f64(left.min(60.0)));
            }
            if GEN.load(Ordering::SeqCst) != gen { return; }
            set_fire(None);
            start_window();
        });
    } else if fire_at().is_none() && should_start(u, now) {
        start_window();
    }
}

/// Claude'un yeni 5 saatlik penceresini hemen başlatır (arka planda).
pub fn start_window() {
    {
        let mut r = RUNNING.lock().unwrap();
        if *r { return; }
        *r = true;
    }
    store::kv_set(K_STATUS, &l10n::t("Başlatılıyor…"));
    std::thread::spawn(|| {
        let (ok, msg) = ping();
        let (_, _, _, h, mi, _) = look::local(sense::now_secs());
        store::kv_set(K_STATUS, &format!("{h:02}:{mi:02} · {msg}"));
        crate::cat::dev_log(&format!("pencere başlatıcı: ok={ok}"));
        *RUNNING.lock().unwrap() = false;
        if ok {
            std::thread::sleep(std::time::Duration::from_secs(15));
            crate::data::refresh_now_fast();
        }
    });
}

/// Haiku'ya tek satırlık istek gönderir, oturum kaydı bırakmaz.
pub fn ping() -> (bool, String) {
    if claude::cli_path().is_none() { return (false, l10n::t("claude komutu bulunamadı")); }
    store::kv_set(K_TRY, &sense::now_secs().to_string());
    // Geliştirme: MEOWLY_STARTER_DRY=1 hiçbir istek göndermeden başarılı sayar.
    if std::env::var_os("MEOWLY_STARTER_DRY").is_some() {
        store::kv_set(K_LAST, &sense::now_secs().to_string());
        return (true, l10n::t("Yeni pencere başlatıldı"));
    }
    match claude::run_cli(&["-p", "--model", "haiku", "--no-session-persistence", "--tools", "", "--", "Reply with only: ok"], 120) {
        Some((true, _)) => {
            store::kv_set(K_LAST, &sense::now_secs().to_string());
            (true, l10n::t("Yeni pencere başlatıldı"))
        }
        Some((false, out)) => (false, format!("Hata: {}", String::from_utf8_lossy(&out).trim().chars().take(120).collect::<String>())),
        None => (false, l10n::t("Zaman aşımı")),
    }
}

/// Ayarlar → "Claude pencere başlatıcı" açıklamasının sonuna eklenen durum (Mac: AutomationPage.starterNote). Arayüz sürecinde.
pub fn note() -> String {
    if !settings::get().starter_on { return String::new(); }
    let mut out = match store::kv_get(K_FIRE).and_then(|v| v.parse::<f64>().ok()) {
        Some(at) => l10n::f(" Sonraki otomatik mesaj: %@.", &[look::when_text(at)]),
        None => l10n::t(" Sıfırlanma saati bekleniyor."),
    };
    let last = store::kv_f64(K_LAST);
    out += &if last > 0.0 { l10n::f(" Son başarılı mesaj: %@.", &[look::when_text(last)]) } else { l10n::t(" Henüz mesaj gönderilmedi.") };
    if let Some(st) = store::kv_get(K_STATUS).filter(|s| !s.is_empty()) { out += &format!(" ({st})"); }
    out
}
