//! İki süreç arası iletişim.
//! Ana süreç (kedi, şerit, tepsi, veri) hafif kalır; arayüz (açılır pencere, pano) `meowly.exe --ui …` ile ayrı
//! süreçte açılır ve son pencere kapanınca kendini kapatır: WebView2'nin belleği tamamen geri verilir.
//! - Ana → arayüz: `ui_state.json` (yalnızca arayüz süreci açıkken yazılır; arayüz dosyayı izler).
//! - Arayüz → ana: ana sürecin gizli "MeowlyMain" penceresine mesaj.

use crate::settings;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_IPC_REFRESH: u32 = WM_APP + 20;
pub const WM_IPC_TOGGLE_BOX: u32 = WM_APP + 21;
pub const WM_IPC_QUIT: u32 = WM_APP + 22;
pub const WM_IPC_SETTINGS: u32 = WM_APP + 23;
pub const WM_IPC_POPOVER: u32 = WM_APP + 24; // wparam 1 = açıldı, 0 = kapandı
pub const WM_IPC_HELLO: u32 = WM_APP + 25;
pub const WM_IPC_NOTIFY_TEST: u32 = WM_APP + 26; // geliştirme: örnek bildirimler

/// Arayüz süreci açıkken var olan adlandırılmış nesne (ana süreç durum dosyasını yalnızca o zaman yazar).
const UI_MUTEX: windows::core::PCWSTR = w!("Local\\MeowlyUiAlive");

pub fn state_file() -> std::path::PathBuf { settings::data_dir().join("ui_state.json") }

/// Arayüz tarafı: ana sürece mesaj gönder.
/// Ana süreç (MeowlyMain penceresi) çalışıyor mu.
pub fn main_alive() -> bool { unsafe { FindWindowW(w!("MeowlyMain"), None).is_ok() } }

pub fn post_main(msg: u32, wp: usize) {
    unsafe {
        if let Ok(h) = FindWindowW(w!("MeowlyMain"), None) {
            let _ = PostMessageW(Some(h), msg, WPARAM(wp), LPARAM(0));
        }
    }
}

/// Arayüz süreci açık mı (ana taraf).
pub fn ui_alive() -> bool {
    use windows::Win32::System::Threading::{OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE};
    unsafe {
        match OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, false, UI_MUTEX) {
            Ok(h) => { let _ = CloseHandle(h); true }
            Err(_) => false,
        }
    }
}

/// Arayüz tarafı: süreç boyunca açık kalan işaret.
pub fn mark_ui_alive() {
    use windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        if let Ok(h) = CreateMutexW(None, false, UI_MUTEX) { std::mem::forget(h); }
    }
}

/// Ana taraf: arayüz açıksa güncel durumu yazar (değişmediyse yazmaz).
pub fn write_ui_state() {
    if !ui_alive() { return; }
    let v = crate::popover::compute_state();
    let Ok(t) = serde_json::to_string(&v) else { return };
    let f = state_file();
    if std::fs::read_to_string(&f).map(|o| o == t).unwrap_or(false) { return; }
    // Ayarlardaki şerit önizlemesi (değişmediyse yazılmaz).
    let png = crate::strip::preview_png();
    let pf = settings::data_dir().join("strip.png");
    if std::fs::read(&pf).map(|o| o != png).unwrap_or(true) { let _ = std::fs::write(&pf, &png); }
    let tmp = f.with_extension("json.tmp");
    if std::fs::write(&tmp, t).is_ok() { let _ = std::fs::rename(&tmp, &f); }
}

/// Ana taraf: arayüz sürecini başlatır ya da açık olana komut iletir (tek örnek eklentisi argümanları aktarır).
/// Arayüzün öne gelebilmesi için ön plan izni verilir (tıklamayı ana süreç aldı).
pub fn spawn_ui(cmd: &str) {
    use std::os::windows::process::CommandExt;
    kill_ui_if_hung();
    unsafe { let _ = AllowSetForegroundWindow(ASFW_ANY); }
    let Ok(exe) = std::env::current_exe() else { return };
    let _ = std::process::Command::new(exe).args(["--ui", cmd]).creation_flags(0x0800_0000).spawn();
}

/// Arayüz süreci kilitlendiyse (Windows'a göre 5 sn'dir yanıt vermiyorsa) kapatır. Yoksa yeni komutlar ona iletilmeye
/// çalışılır ve hiçbir şey açılmaz (şeride tıklama boşa gider). Kapatılınca bir sonraki komut temiz bir süreç başlatır.
pub fn kill_ui_if_hung() {
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE};
    if !ui_alive() { return; }
    unsafe {
        // Tek örnek eklentisinin gizli penceresi: yeni komutlar ona gönderilir.
        let Ok(h) = FindWindowW(w!("app.meowly.windows-sic"), None) else { return };
        if !IsHungAppWindow(h).as_bool() { return; }
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        if pid == 0 || pid == std::process::id() { return; }
        crate::cat::dev_log("arayüz süreci yanıt vermiyordu; kapatıldı");
        if let Ok(p) = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, false, pid) {
            let _ = TerminateProcess(p, 1);
            let _ = WaitForSingleObject(p, 2000);
            let _ = CloseHandle(p);
        }
    }
}

/// Ana sürecin gizli penceresi (hiç gösterilmez; tepsi menüsünün sahibi olabilsin diye mesaj penceresi değil).
pub fn create_main_window(proc_: WNDPROC) -> HWND {
    crate::win::register_class(w!("MeowlyMain"), proc_);
    unsafe {
        let hinst = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap();
        CreateWindowExW(WS_EX_TOOLWINDOW, w!("MeowlyMain"), w!("MeowlyMain"), WS_POPUP, 0, 0, 0, 0,
                        None, None, Some(hinst.into()), None).unwrap()
    }
}
