//! Arayüz süreci: `meowly.exe --ui <komut>`. Açılır pencere ve pano burada (Tauri + WebView2).
//! Tek örnek: açıkken yeni komutlar ona iletilir. Son pencere kapandıktan 30 sn sonra süreç kendini kapatır,
//! böylece WebView2'nin belleği tamamen geri verilir.

use crate::{dashboard, ipc, popover};
use tauri::Manager;

fn handle(app: &tauri::AppHandle, cmd: &str) {
    match cmd {
        "popover" => popover::toggle(),
        "warm" => popover::warm(),
        "prompt:name" => crate::tour::show_name_prompt(app),
        "tour" => crate::tour::start(app),
        "prompt:star" => crate::tour::show_star_prompt(app),
        // Geliştirme: turu tıklamadan ilerletir ("İLERİ" ile aynı kod).
        "tour:next" => { tauri::async_runtime::spawn(crate::tour::tour_next(app.clone())); }
        // Geliştirme: ad penceresindeki TAMAM ile aynı kod (ad değişmez).
        "prompt:done" => { tauri::async_runtime::spawn(crate::tour::name_done(app.clone(), crate::l10n::cat_name())); }
        "quit" => app.exit(0),
        c if c.starts_with("dashboard:") => dashboard::open(app, &c["dashboard:".len()..]),
        // Geliştirme (yalnızca MEOWLY_DEV ayarlıysa): "evalf:<pencere>:<js dosyası>" sayfada JS çalıştırır;
        // sonuç `dev_log` ile %APPDATA%\Meowly\dev.log'a yazılır. Gerçek ekrana tıklama göndermeden düğmeleri dener.
        c if c.starts_with("evalf:") && std::env::var_os("MEOWLY_DEV").is_some() => {
            let rest = &c["evalf:".len()..];
            if let Some((label, path)) = rest.split_once(':') {
                if let (Some(w), Ok(js)) = (app.get_webview_window(label), std::fs::read_to_string(path)) { let _ = w.eval(&js); }
                else { dev_log(format!("evalf: pencere ya da dosya yok: {label}")); }
            }
        }
        _ => {}
    }
}

/// Bir WebView penceresi oluşturulurken WebView2 mesaj döngüsünü kendi içinde çalıştırır; o sırada gelen başka bir
/// ana iş parçacığı işi (ikinci bir pencere oluşturmak) süreci kilitliyordu (üzerine gelme + hemen tıklama).
/// Bu yüzden oluşturma sürerken gelen işler, oluşturma bitince çalışsın diye ertelenir.
static BUILDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Ana iş parçacığında çalıştırır; bir pencere oluşturuluyorsa bitmesini bekleyip öyle çalıştırır.
pub fn on_main(app: &tauri::AppHandle, f: impl FnOnce(&tauri::AppHandle) + Send + 'static) {
    let a = app.clone();
    let r = app.run_on_main_thread(move || {
        if BUILDING.load(std::sync::atomic::Ordering::SeqCst) {
            let a2 = a.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(120));
                on_main(&a2, f);
            });
            return;
        }
        f(&a);
    });
    if let Err(e) = r { err_log(format!("ana iş parçacığına iş gönderilemedi: {e}")); }
}

/// Pencere oluşturma: bu sürerken gelen işler `on_main` ile ertelenir.
pub fn building<T>(f: impl FnOnce() -> T) -> T {
    BUILDING.store(true, std::sync::atomic::Ordering::SeqCst);
    let r = f();
    BUILDING.store(false, std::sync::atomic::Ordering::SeqCst);
    r
}

/// Görünür pencere kalmadıysa 30 sn sonra süreci kapat (bu arada yeni bir şey açılırsa iptal).
pub fn schedule_idle_exit(app: &tauri::AppHandle) {
    static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let g = GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let a2 = app.clone();
        let _ = app.run_on_main_thread(move || {
            if GEN.load(std::sync::atomic::Ordering::Relaxed) != g { return; }
            let any_visible = a2.webview_windows().values().any(|w| w.is_visible().unwrap_or(false));
            if !any_visible { a2.exit(0); }
        });
    });
}

/// Hata kaydı: %APPDATA%\Meowly\errors.log (en çok 64 KB; dolunca baştan başlar). Yalnızca hata metni, içerik yok.
pub fn err_log(s: impl std::fmt::Display) {
    use std::io::Write;
    let p = crate::settings::data_dir().join("errors.log");
    if std::fs::metadata(&p).map(|m| m.len() > 65536).unwrap_or(false) { let _ = std::fs::remove_file(&p); }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = writeln!(f, "{:.0} {}", crate::sense::now_secs(), s);
    }
}

/// Geliştirme: sayfadan gelen satırı dev.log'a ekler (MEOWLY_DEV yoksa hiçbir şey yapmaz).
#[tauri::command]
pub fn dev_log(s: String) {
    if std::env::var_os("MEOWLY_DEV").is_none() { return; }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(crate::settings::data_dir().join("dev.log")) {
        let _ = writeln!(f, "{:.3} {}", crate::sense::now_secs(), s);
    }
}

fn cmd_of(args: &[String]) -> Option<String> {
    args.iter().position(|a| a == "--ui").and_then(|i| args.get(i + 1)).cloned()
}

pub fn run(args: Vec<String>) {
    ipc::mark_ui_alive();
    // Arayüz sürecinde bir hata olursa ekranda görünmez; nedenini errors.log'a yaz.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| { err_log(format!("arayüz hatası: {info}")); prev(info); }));
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(c) = cmd_of(&argv) { handle(app, &c); }
        }))
        .invoke_handler(tauri::generate_handler![
            popover::get_state, popover::tr, popover::tr_all, popover::set_tab, popover::refresh_now, popover::quit,
            popover::open_dashboard, popover::popover_ready,
            crate::usage::usage_page, crate::usage::set_usage_pref, crate::usage::save_csv,
            crate::diary::diary_page, crate::diary::set_diary_mode,
            crate::reports::report_page, crate::reports::save_png,
            crate::settings_ui::settings_page, crate::settings_ui::set_setting, crate::settings_ui::save_cat_name, crate::settings_ui::gh_token,
            crate::settings_ui::open_link, crate::settings_ui::delete_computer_log, crate::settings_ui::sync_now,
            crate::settings_ui::update_check, crate::settings_ui::update_install,
            crate::tour::name_done, crate::tour::tour_start, crate::tour::tour_next, crate::tour::tour_skip, crate::tour::tour_card, crate::tour::tour_anchor, crate::tour::star_done, crate::settings_ui::gh_send, dev_log
        ])
        .setup(move |app| {
            let _ = popover::APP.set(app.handle().clone());
            ipc::post_main(ipc::WM_IPC_HELLO, 0);
            popover::watch_state(app.handle().clone());
            if let Some(c) = cmd_of(&args) { handle(app.handle(), &c); }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Meowly arayüzü başlatılamadı")
        .run(|app, e| match e {
            // Son pencere kapandı: hızlı yeniden açılış için 30 sn bekle, hâlâ görünür pencere yoksa çık.
            tauri::RunEvent::ExitRequested { api, code: None, .. } => { api.prevent_exit(); schedule_idle_exit(app); }
            tauri::RunEvent::WindowEvent { label, event, .. } => {
                if !matches!(event, tauri::WindowEvent::Moved(_) | tauri::WindowEvent::Resized(_)) { dev_log(format!("pencere {label}: {event:?}")); }
                if matches!(event, tauri::WindowEvent::CloseRequested { .. }) && label == "prompt" {
                    crate::tour::PROMPT_CLOSED.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                if matches!(event, tauri::WindowEvent::Destroyed) { schedule_idle_exit(app); }
            }
            _ => {}
        });
}
