//! Kedinin duruşu için yerel sinyaller. Hepsi olay güdümlü ya da tek sistem çağrısı; sürekli yoklama yok.
//! - Kullanıcı boşta süresi: GetLastInputInfo (klavye + fare).
//! - Son tuş zamanı: Raw Input. Yalnızca "bir tuşa basıldı" anı tutulur; hangi tuş olduğu hiç okunmaz.
//! - Video/görüşme: sistemin "ekranı açık tut" isteği (ES_DISPLAY_REQUIRED).
//! - Yapay zekâ etkinliği: Claude/Codex kayıt klasörlerinde dosya değişimi (ReadDirectoryChangesW, notify).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows::Win32::UI::Input::{RegisterRawInputDevices, RAWINPUTDEVICE, RIDEV_INPUTSINK};

fn env_f64(k: &str) -> Option<f64> { std::env::var(k).ok().and_then(|v| v.parse().ok()) }

pub fn now_secs() -> f64 { SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0) }

/// Klavye/fareye en son dokunmadan beri geçen süre (sn).
pub fn user_idle() -> f64 {
    if let Some(v) = env_f64("MEOWLY_USER_IDLE") { return v; }
    let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    unsafe {
        if !GetLastInputInfo(&mut li).as_bool() { return 0.0; }
        // dwTime 32 bit tick; GetTickCount64'ün alt 32 bitiyle farkı alınır (49 günlük taşmaya dayanıklı).
        let now = GetTickCount64() as u32;
        now.wrapping_sub(li.dwTime) as f64 / 1000.0
    }
}

static LAST_KEY_TICK: AtomicU64 = AtomicU64::new(0);

/// Raw Input'a yalnızca klavye için kaydol (pencere arka plandayken de haber alır).
pub fn register_keyboard(hwnd: HWND) -> bool {
    let dev = RAWINPUTDEVICE { usUsagePage: 0x01, usUsage: 0x06, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd };
    unsafe { RegisterRawInputDevices(&[dev], std::mem::size_of::<RAWINPUTDEVICE>() as u32).is_ok() }
}

/// WM_INPUT geldiğinde çağrılır. İçerik okunmaz (GetRawInputData çağrılmaz); yalnızca an kaydedilir.
pub fn key_event() { LAST_KEY_TICK.store(unsafe { GetTickCount64() }, Ordering::Relaxed); }

/// Klavyeye en son basılmasından beri geçen süre (sn).
pub fn key_idle() -> f64 {
    if let Some(v) = env_f64("MEOWLY_KEY_IDLE") { return v; }
    let t = LAST_KEY_TICK.load(Ordering::Relaxed);
    if t == 0 { return f64::INFINITY; }
    (unsafe { GetTickCount64() } - t) as f64 / 1000.0
}

/// Video oynuyor ya da görüntülü görüşme varsa ekran açık tutulur: kullanıcı başındadır.
pub fn media_active() -> bool {
    if let Ok(v) = std::env::var("MEOWLY_MEDIA") { return v != "0"; } // geliştirme: 1 = var, 0 = yok
    display_required()
}

pub fn display_required() -> bool {
    use windows::Win32::System::Power::{CallNtPowerInformation, POWER_INFORMATION_LEVEL};
    let mut state: u32 = 0;
    let r = unsafe {
        CallNtPowerInformation(POWER_INFORMATION_LEVEL(16), None, 0, Some(&mut state as *mut u32 as *mut _), 4)
    };
    r.is_ok() && state & 0x2 != 0 // ES_DISPLAY_REQUIRED
}

// MARK: Yapay zekâ etkinliği

fn home() -> PathBuf { std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default() }

/// Claude/Codex kayıt klasörlerini izler; bir .jsonl/.zst değişince veri iş parçacığı uyandırılır.
/// Dosya içeriği burada okunmaz.
pub fn start_ai_watch() {
    std::thread::spawn(|| {
        use notify::{RecursiveMode, Watcher};
        let (tx, rx) = std::sync::mpsc::channel();
        let Ok(mut w) = notify::recommended_watcher(tx) else { return };
        for d in [home().join(".claude").join("projects"), home().join(".codex").join("sessions")] {
            if d.exists() { let _ = w.watch(&d, RecursiveMode::Recursive); }
        }
        for ev in rx {
            let Ok(ev) = ev else { continue };
            if !(ev.kind.is_modify() || ev.kind.is_create()) { continue; }
            if ev.paths.iter().any(|p: &PathBuf| p.extension().map(|e| e == "jsonl" || e == "zst").unwrap_or(false)) {
                crate::data::refresh_now();
            }
        }
    });
}