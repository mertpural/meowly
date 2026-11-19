//! Tepside yedek simge (ana süreçte, yerel Win32): sol tık uygulamayı açar, sağ tık Aç / Ayarlar / kutu / Çık.

use crate::{cat, ipc, l10n, strip, win};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_TRAY: u32 = WM_APP + 30;

fn main_hwnd() -> HWND { unsafe { FindWindowW(w!("MeowlyMain"), None).unwrap_or_default() } }

fn icon() -> HICON {
    unsafe {
        let hinst = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap_or_default();
        for id in [32512usize, 1] {
            if let Ok(i) = LoadImageW(Some(hinst.into()), PCWSTR(id as *const u16), IMAGE_ICON,
                GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), LR_DEFAULTCOLOR) {
                return HICON(i.0);
            }
        }
        LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
    }
}

/// Simgeyi ekler (Explorer yeniden başlayınca da çağrılır).
pub fn add() {
    unsafe {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: main_hwnd(),
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY,
            hIcon: icon(),
            ..Default::default()
        };
        for (i, c) in "Meowly".encode_utf16().enumerate() { nid.szTip[i] = c; }
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

pub fn remove() {
    unsafe {
        let nid = NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: main_hwnd(), uID: 1, ..Default::default() };
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

fn box_label() -> String {
    if cat::in_box() { l10n::t("Tonti'yi masaüstüne çıkar") } else { l10n::t("Tonti'yi kutuya koy") }
}

pub fn action(id: &str) {
    match id {
        "open" => ipc::spawn_ui("dashboard:diary"),
        "settings" => ipc::spawn_ui("dashboard:appearance"),
        "box" => { cat::toggle_box(); strip::refresh(); }
        "quit" => unsafe { PostQuitMessage(0) },
        _ => {}
    }
}

/// Sağ tık menüsü (tepside ve şeritte aynı).
pub fn show_menu() {
    let items = [("open", l10n::t("Aç")), ("settings", l10n::t("Ayarlar")), ("box", box_label()), ("quit", l10n::t("Çık"))];
    unsafe {
        let Ok(m) = CreatePopupMenu() else { return };
        let wide: Vec<Vec<u16>> = items.iter().map(|(_, t)| t.encode_utf16().chain(std::iter::once(0)).collect()).collect();
        for (i, wd) in wide.iter().enumerate() {
            let _ = AppendMenuW(m, MF_STRING, i + 1, PCWSTR(wd.as_ptr()));
        }
        let (x, y) = win::cursor();
        let owner = main_hwnd();
        let _ = SetForegroundWindow(owner);
        let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, x, y, None, owner, None);
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(m);
        if cmd.0 > 0 { action(items[cmd.0 as usize - 1].0); }
    }
}

/// Simgeden gelen fare olayları.
pub fn on_message(lp: LPARAM) {
    match lp.0 as u32 {
        WM_LBUTTONUP => action("open"),
        WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(),
        // Bildirime tıklandı (NIN_BALLOONUSERCLICK).
        x if x == WM_USER + 5 => ipc::spawn_ui("dashboard:usage"),
        _ => {}
    }
}
