// Konsol penceresi açılmasın (yalnızca sürüm derlemesinde).
#![cfg_attr(all(not(debug_assertions), not(feature = "console")), windows_subsystem = "windows")]

mod activity;
mod cat;
mod claude;
mod codex;
mod computer;
mod dashboard;
mod data;
mod diary;
mod img;
mod ipc;
mod look;
mod popover;
mod reports;
mod strip;
mod store;
mod l10n;
mod notify;
mod update;
mod starter;
mod tour;
mod github;
mod gemini;
mod antigravity;
mod extras;
mod calendar;
mod sense;
mod settings;
mod settings_ui;
mod text;
mod tray;
mod ui_app;
mod usage;
mod win;

use std::sync::OnceLock;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// Ana sürecin gizli penceresi: arayüz sürecinden ve tepsi simgesinden gelen mesajlar.
extern "system" fn main_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
    let tc = *TASKBAR_CREATED.get_or_init(|| unsafe { RegisterWindowMessageW(windows::core::w!("TaskbarCreated")) });
    match msg {
        // Yalnızca kullanıcının elle yenilemesi bu mesajı gönderir: Claude limiti de hemen ölçülsün.
        ipc::WM_IPC_REFRESH => { claude::force_probe(); data::refresh_now_fast() }
        ipc::WM_IPC_TOGGLE_BOX => { cat::toggle_box(); strip::refresh(); }
        ipc::WM_IPC_QUIT => unsafe { PostQuitMessage(0) },
        ipc::WM_IPC_SETTINGS => { settings::reload(); strip::refresh(); data::refresh_now_fast(); activity::send(activity::Msg::Settings); }
        ipc::WM_IPC_POPOVER => strip::set_popover_open(wp.0 == 1),
        ipc::WM_IPC_HELLO => ipc::write_ui_state(),
        ipc::WM_IPC_NOTIFY_TEST => { std::thread::spawn(notify::demo); }
        tray::WM_TRAY => tray::on_message(lp),
        m if m == tc => tray::add(),
        _ => return unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
    LRESULT(0)
}

/// Sürekli açık kalan hafif süreç: kedi, şerit, tepsi, veri. Tauri/WebView burada hiç başlatılmaz.
fn run_main() {
    use windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        // Tek örnek: Meowly zaten açıksa ikincisi hemen çıkar.
        let _m = CreateMutexW(None, true, windows::core::w!("Local\\MeowlyMainAlive"));
        if GetLastError() == ERROR_ALREADY_EXISTS { return; }
        let _ = windows::Win32::UI::HiDpi::SetProcessDpiAwarenessContext(windows::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        ipc::create_main_window(Some(main_proc));
        // Geliştirme: bellek ölçümü için parçalar ayrı ayrı kapatılabilir.
        if std::env::var_os("MEOWLY_NOWATCH").is_none() { sense::start_ai_watch(); }
        if std::env::var_os("MEOWLY_NOCAT").is_none() { cat::start(); }
        std::thread::spawn(settings_ui::self_heal);
        // İlk açılış (Mac: CatPet açılışı): 1,5 sn sonra ad hiç girilmediyse adını sorar, ilk kullanım anını yazar;
        // güncelleyen mevcut kullanıcılara tur gösterilmez. 20 sn sonra vakti geldiyse bir kez yıldız rica eder.
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(1500));
            let s = settings::get();
            let now = sense::now_secs();
            if s.first_use_at <= 0.0 || (s.cat_name.is_some() && !s.tour_done) {
                settings::update(|s| {
                    if s.first_use_at <= 0.0 { s.first_use_at = now; }
                    if s.cat_name.is_some() { s.tour_done = true; }
                });
            }
            if s.cat_name.is_none() { ipc::spawn_ui("prompt:name"); }
            std::thread::sleep(std::time::Duration::from_secs(18));
            if tour::star_due(sense::now_secs()) { ipc::spawn_ui("prompt:star"); }
        });
        if std::env::var_os("MEOWLY_NOTRAY").is_none() { tray::add(); }
        if std::env::var_os("MEOWLY_NOSTRIP").is_none() { strip::start(); }
        update::check_pending();
        data::start(cat::notify);
        if std::env::var_os("MEOWLY_NOLOG").is_none() { activity::start(); }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        tray::remove();
        activity::send(activity::Msg::Stop);
        std::thread::sleep(std::time::Duration::from_millis(300));
        // Arayüz açıksa onu da kapat.
        if ipc::ui_alive() { ipc::spawn_ui("quit"); }
    }
}
fn main() {
    // Claude Code'un durum satırı komutu: stdin'i dosyaya yazar ve hemen çıkar (pencere, tepsi yok).
    // Kaldırıcı çağırır: çalışan uygulamayı kapatır, giriş kaydını ve durum satırımızı siler.
    if std::env::args().any(|a| a == "--uninstall-cleanup") {
        ipc::post_main(ipc::WM_IPC_QUIT, 0);
        for _ in 0..30 {
            if !ipc::main_alive() { break; }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        settings_ui::uninstall_cleanup();
        return;
    }
    // Geliştirme: Hakkında'daki düğmelerle aynı kod (denetle / indir-doğrula-kur); sonuç dev.log'a da yazılır.
    if std::env::args().any(|a| a == "--update-check") {
        let r = update::check_manual();
        cat::dev_log(&format!("güncelleme denetimi: {}", r["status"].as_str().unwrap_or("")));
        println!("{r}");
        return;
    }
    if std::env::args().any(|a| a == "--update-now") {
        let r = update::install();
        cat::dev_log(&format!("güncelleme kurulumu: {}", if r.is_empty() { "başladı" } else { r.as_str() }));
        println!("{r}");
        return;
    }
    // Geliştirme: pencere başlatıcının isteğini bir kez, uygulamanın koduyla gönderir; sonuç dev.log'a ve ayar notuna yazılır.
    if std::env::args().any(|a| a == "--starter-ping") {
        let t0 = std::time::Instant::now();
        let (ok, msg) = starter::ping();
        let (_, _, _, h, mi, _) = look::local(sense::now_secs());
        store::kv_set(starter::K_STATUS, &format!("{h:02}:{mi:02} · {msg}"));
        cat::dev_log(&format!("pencere başlatıcı (elle): ok={ok} süre={:.1}sn mesaj={msg}", t0.elapsed().as_secs_f64()));
        return;
    }
    // Geliştirme: GitHub kartını dosyaya yazar (gönderilmez). MEOWLY_GH_SPLIT=0/1, MEOWLY_GH_DARK=0/1.
    if let Some(i) = std::env::args().position(|a| a == "--gh-svg") {
        let s = settings::get();
        let f = |k: &str, d: bool| std::env::var(k).map(|v| v == "1").unwrap_or(d);
        let out = std::env::args().nth(i + 1).unwrap_or_else(|| "meowly.svg".into());
        let _ = std::fs::write(out, github::svg(f("MEOWLY_GH_SPLIT", s.gh_split), f("MEOWLY_GH_DARK", s.gh_dark), sense::now_secs()));
        return;
    }
    // Geliştirme: "Şimdi gönder" ile aynı kod (--gh-push) ya da zamanlanmış gönderim (--gh-push-auto); sonuç dev.log'a.
    if std::env::args().any(|a| a == "--gh-push" || a == "--gh-push-auto") {
        let r = github::push(std::env::args().any(|a| a == "--gh-push"));
        cat::dev_log(&format!("github: {r}"));
        return;
    }
    // Geliştirme: çalışan uygulamaya örnek bildirimleri gösterttirir.
    if std::env::args().any(|a| a == "--notify-test") {
        ipc::post_main(ipc::WM_IPC_NOTIFY_TEST, 0);
        return;
    }
    if std::env::args().any(|a| a == "--statusline") {
        claude::statusline_main();
        return;
    }
    // Geliştirme: verileri okuyup yalnızca sayıları yazar (anahtar, sohbet içeriği yok).
    if std::env::args().any(|a| a == "--dump") {
        claude::mark_launch();
        // Geliştirme: "Senkronize et" ile aynı yol (limit hemen ölçülür).
        if std::env::var_os("MEOWLY_FORCE_PROBE").is_some() { claude::force_probe(); }
        println!("durum satiri: {:?}", claude::read_status_line());
        let t0 = std::time::Instant::now();
        let c = claude::read();
        println!("claude: okuma {:.1} sn", t0.elapsed().as_secs_f64());
        println!("  bugun token: {} ({})", c.today_tokens, cat::format_tokens(c.today_tokens));
        println!("  guven: {:?}  eski: {:?}  oturum_suresi_doldu: {}", c.confidence, c.stale_since, c.auth_expired);
        if let Some(l) = &c.limits {
            println!("  kaynak: {:?}  5s kalan: {:?}  haftalik kalan: {:?}", l.source, l.five_left, l.week_left);
            println!("  5s sifirlanma: {:?}  haftalik: {:?}  alinma: {:.0} sn once", l.five_reset, l.week_reset, sense::now_secs() - l.fetched_at);
        }
        for n in &c.notes { println!("  not: {n}"); }
        let t1 = std::time::Instant::now();
        let x = codex::read();
        println!("codex: okuma {:.1} sn  bugun token: {}", t1.elapsed().as_secs_f64(), x.today_tokens);
        if let Some(l) = &x.limits { println!("  5s kalan: {:?}  haftalik kalan: {:?}", l.five_left, l.week_left); }
        for n in &x.notes { println!("  not: {n}"); }
        let day = store::start_of_day(sense::now_secs());
        println!("bugun toplam (tum araclar): {}", cat::format_tokens(store::total_tokens(day)));
        println!("son proje: {:?}", store::latest_project());
        for s in store::sessions(day, 1e11) {
            println!("  oturum {} {:.0} dk {} token", s.project, s.duration() / 60.0, cat::format_tokens(s.tokens));
        }
        println!("claude komutu: {:?}", claude::cli_path().map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())));
        return;
    }
    // Geliştirme: Kullanım sayfasının ve açılır pencerenin verisini dosyaya yazar (tarayıcıda önizleme için).
    if let Some(i) = std::env::args().position(|a| a == "--dump-ui") {
        let out = std::env::args().nth(i + 1).unwrap_or_else(|| "ui-dump.json".into());
        let usage = tauri::async_runtime::block_on(usage::usage_page("week".into(), -1));
        let diary = tauri::async_runtime::block_on(diary::diary_page(sense::now_secs()));
        let report = tauri::async_runtime::block_on(reports::report_page(std::env::var("MEOWLY_RKIND").unwrap_or("week".into()), sense::now_secs() - std::env::var("MEOWLY_RBACK").ok().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0), 0));
        let settings = tauri::async_runtime::block_on(settings_ui::settings_page());
        claude::mark_launch();
        data::read_all();
        let v = serde_json::json!({ "usage": usage, "diary": diary, "report": report, "settings": settings, "state": popover::compute_state(), "tr": l10n::table() });
        let _ = std::fs::write(out, serde_json::to_string(&v).unwrap_or_default());
        return;
    }
    if let Some(dir) = std::env::var_os("MEOWLY_GALLERY") {
        cat::gallery(std::path::Path::new(&dir));
        return;
    }
    if let Some(p) = std::env::var_os("MEOWLY_STRIPSHOT") {
        claude::mark_launch();
        data::read_all();
        strip::snapshot(std::path::Path::new(&p), std::env::var("MEOWLY_PT_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(2.0));
        return;
    }
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--ui") {
        ui_app::run(args);
        return;
    }
    run_main();
}
