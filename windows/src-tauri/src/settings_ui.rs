//! Ayarlar sayfasının komutları (Mac: Dashboard.swift → AppearancePage, ProvidersPage, MenuBarPage, AutomationPage,
//! SyncPage, AboutPage; ComputerUI.swift → ComputerLogSettings; GitHubProfile.swift → GitHubProfileSettings).
//! Arayüz sürecinde çalışır; değişiklikten sonra ana sürece "ayar değişti" der.

use crate::{claude, ipc, l10n, settings, store};
use serde_json::{json, Value};
use std::path::PathBuf;

fn home() -> PathBuf { std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default() }

pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for ch in data.chunks(3) {
        let n = (ch[0] as u32) << 16 | (*ch.get(1).unwrap_or(&0) as u32) << 8 | *ch.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= ch.len() { out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char); } else { out.push('='); }
        }
    }
    out
}

fn png_url(bytes: &[u8]) -> String { format!("data:image/png;base64,{}", b64(bytes)) }

// MARK: Windows ile başlat (HKCU\...\Run)

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

pub fn login_item_enabled() -> bool {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    let (k, v) = (wide(RUN_KEY), wide("Meowly"));
    unsafe { RegGetValueW(HKEY_CURRENT_USER, windows::core::PCWSTR(k.as_ptr()), windows::core::PCWSTR(v.as_ptr()), RRF_RT_REG_SZ, None, None, None).is_ok() }
}

fn set_login_item(on: bool) {
    settings::update(|s| s.login_item_on = on);
    write_login_item(on);
}

fn write_login_item(on: bool) {
    use windows::Win32::System::Registry::{RegDeleteKeyValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
    let (k, v) = (wide(RUN_KEY), wide("Meowly"));
    unsafe {
        if on {
            let exe = std::env::current_exe().unwrap_or_default();
            let val = wide(&format!("\"{}\"", exe.to_string_lossy()));
            let bytes = std::slice::from_raw_parts(val.as_ptr() as *const u8, val.len() * 2);
            let _ = RegSetKeyValueW(HKEY_CURRENT_USER, windows::core::PCWSTR(k.as_ptr()), windows::core::PCWSTR(v.as_ptr()), REG_SZ.0, Some(bytes.as_ptr() as *const _), bytes.len() as u32);
        } else {
            let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, windows::core::PCWSTR(k.as_ptr()), windows::core::PCWSTR(v.as_ptr()));
        }
    }
}

/// Kaldırıcı çağırır (meowly.exe --uninstall-cleanup): Windows ile başlat kaydını ve bizim durum satırımızı siler.
/// Ayarlar dosyası kalır; yeniden kurulunca `self_heal` kullanıcının seçimlerini geri koyar.
pub fn uninstall_cleanup() {
    let exe = std::env::current_exe().unwrap_or_default();
    let ours = format!("\"{}\"", exe.to_string_lossy());
    if login_item_value().as_deref() == Some(ours.as_str()) { write_login_item(false); }
    claude::uninstall_status_line_of_this_exe();
}

/// Açılışta: kullanıcının açtığı ama (güncelleme/yeniden kurulumla) silinmiş kayıtları geri koyar.
pub fn self_heal() {
    let s = settings::get();
    if s.login_item_on && !login_item_enabled() { write_login_item(true); }
    if s.statusline_on && claude::status_line_state() == (false, false) { let _ = claude::install_status_line(); }
}

fn login_item_value() -> Option<String> {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
    let (k, v) = (wide(RUN_KEY), wide("Meowly"));
    let mut buf = vec![0u16; 1024];
    let mut n = (buf.len() * 2) as u32;
    unsafe {
        RegGetValueW(HKEY_CURRENT_USER, windows::core::PCWSTR(k.as_ptr()), windows::core::PCWSTR(v.as_ptr()), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr() as *mut _), Some(&mut n)).ok().ok()?;
    }
    Some(String::from_utf16_lossy(&buf[..(n as usize / 2).saturating_sub(1)]))
}

// MARK: GitHub anahtarı (Windows Kimlik Bilgisi Yöneticisi; Mac'te Anahtar Zinciri)

const CRED_TARGET: &str = "Meowly/github";

/// Kayıtlı mı? Windows bu kontrolü içeriği okumadan yapmaya izin vermez: okunan değer hiç kullanılmadan hemen bırakılır.
pub fn gh_has_token() -> bool {
    use windows::Win32::Security::Credentials::{CredFree, CredReadW, CRED_TYPE_GENERIC, CREDENTIALW};
    let t = wide(CRED_TARGET);
    let mut p: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        let ok = CredReadW(windows::core::PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None, &mut p).is_ok();
        if !p.is_null() { CredFree(p as *const _); }
        ok
    }
}

fn gh_save_token(token: &str) -> bool {
    use windows::Win32::Security::Credentials::{CredWriteW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW};
    let mut t = wide(CRED_TARGET);
    let mut user = wide("meowly");
    let mut blob = token.as_bytes().to_vec();
    let cred = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: windows::core::PWSTR(t.as_mut_ptr()),
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        UserName: windows::core::PWSTR(user.as_mut_ptr()),
        ..Default::default()
    };
    let ok = unsafe { CredWriteW(&cred, 0).is_ok() };
    blob.iter_mut().for_each(|b| *b = 0); // bellekte bırakma
    ok
}

fn gh_delete_token() {
    use windows::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};
    let t = wide(CRED_TARGET);
    unsafe { let _ = CredDeleteW(windows::core::PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None); }
}

/// Anahtarın kendisi: yalnızca gönderirken okunur; hiçbir yere yazılmaz, günlüğe girmez.
pub fn gh_read_token() -> Option<String> {
    use windows::Win32::Security::Credentials::{CredFree, CredReadW, CRED_TYPE_GENERIC, CREDENTIALW};
    let t = wide(CRED_TARGET);
    let mut p: *mut CREDENTIALW = std::ptr::null_mut();
    unsafe {
        CredReadW(windows::core::PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None, &mut p).ok()?;
        let c = &*p;
        let v = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize);
        let s = String::from_utf8(v.to_vec()).ok();
        CredFree(p as *const _);
        s.filter(|s| !s.is_empty())
    }
}

/// "Şimdi gönder" (Mac: GitHubModel.sendNow): değişiklik kontrolü yapılmadan gönderir.
#[tauri::command]
pub async fn gh_send() -> String { crate::github::push(true) }

pub fn gh_markdown(user: &str) -> String {
    format!("[![Meowly](https://raw.githubusercontent.com/{user}/{user}/HEAD/meowly.svg)](https://meowly.vercel.app)")
}

// MARK: Güncelleme

#[tauri::command]
pub async fn update_check() -> Value { crate::update::check_manual() }

/// Başarılıysa boş döner; kurulum başlamıştır, arayüz süreci de kapanır.
#[tauri::command]
pub async fn update_install(app: tauri::AppHandle) -> String {
    let r = crate::update::install();
    if r.is_empty() {
        std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(800)); app.exit(0); });
    }
    r
}

// MARK: Sayfa verisi

#[tauri::command]
pub async fn settings_page() -> Value {
    let s = settings::get();
    let state: Value = std::fs::read_to_string(ipc::state_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    let limit_of = |id: &str| state["all"].as_array().and_then(|a| a.iter().find(|p| p["id"] == id)).map(|p| p["limit"].as_str().unwrap_or("none").to_string()).unwrap_or_else(|| "none".into());
    let (ours, other) = claude::status_line_state();
    let setup = vec![
        json!({ "ok": claude::cli_path().is_some() || home().join(".claude").exists(), "title": l10n::t("Claude Code kurulu"),
                "hint": l10n::t("Terminalde `claude` komutunu bir kez çalıştırıp giriş yap. Meowly limiti oradaki oturumdan okur.") }),
        json!({ "ok": limit_of("claude") == "exact", "title": l10n::t("Claude limiti okunuyor"),
                "hint": l10n::t("Claude Code'a giriş yapılı değilse ya da oturum bittiyse limit görünmez. Terminalde `claude` çalıştır, gerekirse /login yaz.") }),
        json!({ "ok": ours || limit_of("claude") == "exact", "title": l10n::t("Claude Code durum satırı bağlı"),
                "hint": l10n::t("İsteğe bağlı. Bildirim ve otomasyon sayfasından açarsan limit, ağa gitmeden Claude Code'dan okunur. Açtıktan sonra Claude Code'ta bir mesaj yazınca veri gelir.") }),
        json!({ "ok": home().join(".codex").join("sessions").exists(), "title": l10n::t("Codex kullanımı bulundu"),
                "hint": l10n::t("Codex'i hiç kullanmadıysan bu normal. Kullanınca kendiliğinden görünür.") }),
        json!({ "ok": limit_of("codex") == "exact", "title": l10n::t("Codex limiti okunuyor"),
                "hint": l10n::t("Codex ile bir mesaj yazınca limit değeri kayıtlara düşer.") }),
        json!({ "ok": home().join(".gemini").join("oauth_creds.json").exists(), "title": l10n::t("Gemini CLI oturumu var"),
                "hint": l10n::t("Gemini istemiyorsan boş bırak. Kullanacaksan terminalde `gemini` çalıştırıp giriş yap.") }),
    ];
    let styles = ["percent", "bar", "barPercent", "ring", "dot", "battery", "segments"];
    let previews: serde_json::Map<String, Value> = styles.iter().map(|st| (st.to_string(), Value::String(png_url(&crate::strip::glyph_preview_png(st, 2.0))))).collect();
    let strip = std::fs::read(settings::data_dir().join("strip.png")).ok().map(|b| png_url(&b));
    json!({
        "s": serde_json::to_value(&s).unwrap_or_default(),
        "dark": s.dark,
        "en": l10n::english(),
        "catName": l10n::cat_name(),
        "setup": setup,
        "providers": state["all"].clone(),
        "statusLineOther": other && !ours,
        "loginItem": login_item_enabled(),
        "winToastsOff": windows_toasts_off(),
        "ghToken": gh_has_token(),
        // Takvim deposu yalnızca katman açıksa sorulur (temiz bilgisayarda Windows isteği bekletebilir).
        "calendarAccess": s.calendar_on && crate::calendar::access_ok(),
        "ghStatus": s.gh_status.clone(),
        "ghMarkdown": gh_markdown(if s.gh_user.is_empty() { "kullanıcıadı" } else { &s.gh_user }),
        "previews": previews,
        "strip": strip,
        "updatedAt": state["updatedAt"].clone(),
        "version": env!("CARGO_PKG_VERSION"),
        "update": crate::update::page_state(),
        "starterNote": crate::starter::note(),
    })
}

/// Tek bir ayarı yazar; ana süreç yeniden okur (şerit, kedi, dil).
#[tauri::command]
pub fn set_setting(app: tauri::AppHandle, key: String, value: Value) -> Value {
    let b = value.as_bool().unwrap_or(false);
    let st = value.as_str().unwrap_or("").to_string();
    let mut msg = Value::Null;
    match key.as_str() {
        "statusline_on" => {
            // Yalnızca kullanıcı açarsa eklenir; önce yedek alınır, başka bir durum satırı varsa dokunulmaz.
            if b {
                match claude::install_status_line() {
                    Ok(()) => settings::update(|s| s.statusline_on = true),
                    Err(e) => { msg = Value::String(l10n::t(&e)); settings::update(|s| s.statusline_on = false); }
                }
            } else {
                claude::uninstall_status_line();
                settings::update(|s| s.statusline_on = false);
            }
        }
        "login_item" => set_login_item(b),
        _ => settings::update(|s| match key.as_str() {
            "dark" => s.dark = b,
            "include_cache" => s.include_cache = b,
            "language" => s.language = st.clone(),
            "journal_commits" => s.journal_commits = b,
            "cat_bubbles" => s.cat_bubbles = b,
            "bubble_work" => s.bubble_work = b,
            "bubble_break" => s.bubble_break = b,
            "bubble_daily" => s.bubble_daily = b,
            "bubble_record" => s.bubble_record = b,
            "bubble_chat" => s.bubble_chat = b,
            "show_tags" => s.show_tags = b,
            "show_five" => s.show_five = b,
            "show_week" => s.show_week = b,
            "style_five" => s.style_five = st.clone(),
            "style_week" => s.style_week = st.clone(),
            "notifications_on" => s.notifications_on = b,
            "show_forecast" => s.show_forecast = b,
            "show_next_start" => s.show_next_start = b,
            "weekly_summary" => s.weekly_summary = b,
            "cli_probe" => s.cli_probe = b,
            "refresh_login" => s.refresh_login = b,
            "starter_on" => s.starter_on = b,
            "sync_interval" => s.sync_interval = value.as_u64().unwrap_or(30),
            "log_apps" => s.log_apps = b,
            "log_titles" => s.log_titles = b,
            "log_sites" => s.log_sites = b,
            "log_files" => s.log_files = b,
            "log_calendar" => s.log_calendar = b,
            "gh_on" => s.gh_on = b,
            "calendar_on" => s.calendar_on = b,
            "log_nudge_hidden" => s.log_nudge_hidden = b,
            "gh_user" => s.gh_user = st.trim().to_string(),
            "gh_interval" => s.gh_interval = st.clone(),
            "gh_split" => { s.gh_split = b; s.gh_last_hash.clear(); }
            "gh_dark" => { s.gh_dark = b; s.gh_last_hash.clear(); }
            "update_check" => s.update_check = b,
            "provider" => {
                // value: { id, on }
                let id = value["id"].as_str().unwrap_or("").to_string();
                let on = value["on"].as_bool().unwrap_or(false);
                s.enabled.retain(|e| e != &id);
                if on { s.enabled.push(id); }
            }
            _ => {}
        }),
    }
    if key == "dark" {
        // Pencerenin başlık çubuğu da açık/koyu olsun.
        use tauri::Manager;
        for w in app.webview_windows().values() { let _ = w.set_theme(Some(if b { tauri::Theme::Dark } else { tauri::Theme::Light })); }
    }
    ipc::post_main(ipc::WM_IPC_SETTINGS, 0);
    msg
}

/// Kedinin adı: boşsa Tonti, en çok 20 karakter.
#[tauri::command]
pub fn save_cat_name(name: String) {
    let n: String = name.trim().chars().take(20).collect();
    settings::update(|s| s.cat_name = Some(if n.is_empty() { "Tonti".into() } else { n }));
    ipc::post_main(ipc::WM_IPC_SETTINGS, 0);
}

#[tauri::command]
pub fn gh_token(action: String, token: Option<String>) -> bool {
    match action.as_str() {
        "save" => token.map(|t| gh_save_token(t.trim())).unwrap_or(false),
        "remove" => { gh_delete_token(); true }
        _ => false,
    }
}

/// Windows bildirimleri kullanıcı ayarından tümüyle kapalı mı (Ayarlar > Sistem > Bildirimler). Yalnızca okunur.
fn windows_toasts_off() -> bool {
    use windows::Win32::System::Registry::*;
    let mut v: u32 = 1;
    let mut n = 4u32;
    unsafe {
        let _ = RegGetValueW(HKEY_CURRENT_USER, windows::core::w!("Software\\Microsoft\\Windows\\CurrentVersion\\PushNotifications"),
            windows::core::w!("ToastEnabled"), RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut _), Some(&mut n));
    }
    v == 0
}

/// Yalnızca uygulamanın kendi bağlantıları açılır (Mac: NSWorkspace.open).
#[tauri::command]
pub fn open_link(which: String) {
    let url = match which.as_str() {
        "repo" => "https://github.com/mertpural/meowly",
        "issues" => "https://github.com/mertpural/meowly/issues",
        "site" => "https://meowly.vercel.app",
        "notifications" => "ms-settings:notifications",
        "calprivacy" => "ms-settings:privacy-calendar",
        _ => return,
    };
    let w = wide(url);
    unsafe {
        windows::Win32::UI::Shell::ShellExecuteW(None, windows::core::w!("open"), windows::core::PCWSTR(w.as_ptr()), None, None,
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL);
    }
}

/// Bilgisayar günlüğünü siler (yapay zekâ oturumları ve commit'ler etkilenmez).
#[tauri::command]
pub fn delete_computer_log() { store::delete_computer_log(); }

#[tauri::command]
pub fn sync_now() { ipc::post_main(ipc::WM_IPC_REFRESH, 0); }
