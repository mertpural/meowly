//! Açılır pencere (Mac: Views.swift → PopoverView). Arayüz sürecinde WebView2 penceresi; odağı kaybedince kapanır.
//! Durum hesabı (`compute_state`) ana süreçte yapılır ve `ui_state.json` ile arayüze verilir.

use crate::data::{self, ProviderUsage};
use crate::look;
use crate::{ipc, l10n, sense, settings, store};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub static APP: OnceLock<AppHandle> = OnceLock::new();
static CLOSED_AT: AtomicU64 = AtomicU64::new(0);
/// Pencere görünür olduğu an (ms). Oluşurken odak birkaç kez gidip gelir; bu arada kapanmasın.
static SHOWN_AT: AtomicU64 = AtomicU64::new(0);
/// Açılması istendi (yeni pencere ya da gizliden yeniden açılış). Gizliyken gelen veri güncellemesi pencereyi göstermez.
static WANT_SHOW: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Son ölçülen sayfa yüksekliği (f64 bitleri): gizliden açılışta sayfayı beklemeden bu boyla gösterilir.
static LAST_H: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 { (sense::now_secs() * 1000.0) as u64 }

/// Açılır pencereyi gizler (kapatmaz: arayüz süreci açıkken yeniden açılış anında olsun).
fn hide(app: &AppHandle) {
    let Some(w) = app.get_webview_window("popover") else { return };
    if !w.is_visible().unwrap_or(false) { return; }
    let _ = w.hide();
    SHOWN_AT.store(0, Ordering::Relaxed);
    CLOSED_AT.store(now_ms(), Ordering::Relaxed);
    ipc::post_main(ipc::WM_IPC_POPOVER, 0);
    crate::ui_app::schedule_idle_exit(app);
}

/// Şerit tıklanınca: açıksa kapat, kapalıysa aç.
pub fn toggle() {
    let Some(app) = APP.get().cloned() else { return };
    crate::ui_app::on_main(&app, move |app| {
        let app = app.clone();
        let existing = app.get_webview_window("popover");
        if let Some(w) = &existing {
            if w.is_visible().unwrap_or(false) { hide(&app); return; }
        }
        // Şeride tıklama önce pencerenin odağını alıp gizler; hemen ardından yeniden açılmasın.
        if now_ms() - CLOSED_AT.load(Ordering::Relaxed) < 300 { return; }
        if let Some(w) = existing {
            // Gizli pencere: son boyla hemen göster (gizli WebView çizim yapmadığı için sayfayı beklemek pencereyi hiç
            // açmıyordu), sonra sayfa güncel durumu çizer ve gerekirse boyu düzeltir.
            use tauri::Emitter;
            let h = f64::from_bits(LAST_H.load(Ordering::Relaxed));
            place_and_show(&app, &w, if h > 0.0 { h } else { 300.0 }, true);
            let _ = app.emit("meowly-reshow", ());
            return;
        }
        WANT_SHOW.store(true, Ordering::Relaxed);
        create(&app);
    });
}

/// İmleç şeridin üstüne gelince: pencereyi gizli olarak hazırla, tıklanınca anında açılsın.
pub fn warm() {
    let Some(app) = APP.get().cloned() else { return };
    crate::ui_app::on_main(&app, move |app| {
        let app = app.clone();
        if app.get_webview_window("popover").is_some() { return; }
        create(&app);
        crate::ui_app::schedule_idle_exit(&app);
    });
}

fn create(app: &AppHandle) {
        let Ok(w) = crate::ui_app::building(|| WebviewWindowBuilder::new(app, "popover", WebviewUrl::App("popover.html".into()))
            .title("Meowly")
            .inner_size(380.0, 300.0)
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .visible(false)
            .shadow(true)
            .build()).map_err(|e| crate::ui_app::err_log(format!("açılır pencere açılamadı: {e}"))) else { return };
        w.on_window_event(|e| match e {
            tauri::WindowEvent::Focused(false) => {
                if SHOWN_AT.load(Ordering::Relaxed) == 0 { return; }
                if let Some(app) = APP.get() { hide(app); }
            }
            tauri::WindowEvent::CloseRequested { api, .. } => {
                // Alt+F4 gibi kapatmalar da yalnızca gizler.
                api.prevent_close();
                if let Some(app) = APP.get() { hide(app); }
            }
            _ => {}
        });
}

/// Windows arka plandaki uygulamanın öne gelmesini kısıtlar: ön plandaki iş parçacığına kısa süre bağlanıp
/// pencereyi etkinleştir (açılır pencere odak alsın ki dışarı tıklanınca kapanabilsin).
/// Geliştirme: MEOWLY_TEST_NOFOCUS ayarlıysa pencereler öne gelip odak almaz (testler kullanıcının yazdığını çalmasın).
pub fn may_focus() -> bool { std::env::var_os("MEOWLY_TEST_NOFOCUS").is_none() }

/// Pencereyi gösterir; test kipinde (MEOWLY_TEST_NOFOCUS) etkinleştirmeden gösterir.
pub fn show_window(w: &tauri::WebviewWindow) {
    if may_focus() { let _ = w.show(); return; }
    if let Ok(h) = w.hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
        unsafe { let _ = ShowWindow(windows::Win32::Foundation::HWND(h.0 as *mut _), SW_SHOWNOACTIVATE); }
    }
}

pub fn force_foreground(h: windows::Win32::Foundation::HWND) {
    if !may_focus() { return; }
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow};
    unsafe {
        let fg = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg, None);
        let me = GetCurrentThreadId();
        let attached = fg_thread != 0 && fg_thread != me && AttachThreadInput(me, fg_thread, true).as_bool();
        let _ = BringWindowToTop(h);
        let _ = SetForegroundWindow(h);
        if attached { let _ = AttachThreadInput(me, fg_thread, false); }
    }
}

/// Sayfa kendi yüksekliğini ölçünce çağırır: boyutlanır, şeridin üstüne yerleşir; açılması istendiyse görünür.
#[tauri::command]
pub fn popover_ready(app: AppHandle, height: f64) {
    let Some(w) = app.get_webview_window("popover") else { return };
    LAST_H.store(height.to_bits(), Ordering::Relaxed);
    let visible = w.is_visible().unwrap_or(false);
    let want = WANT_SHOW.swap(false, Ordering::Relaxed);
    if !visible && !want { return; }
    place_and_show(&app, &w, height, !visible);
}

/// Boyutlandırır, şeridin üstüne yerleştirir, gösterir; `activate` ise öne getirip odaklar.
fn place_and_show(app: &AppHandle, w: &tauri::WebviewWindow, height: f64, activate: bool) {
    let scale = w.scale_factor().unwrap_or(1.0);
    let _ = w.set_size(tauri::LogicalSize::new(380.0, height));
    let (pw, ph) = ((380.0 * scale).round() as i32, (height * scale).round() as i32);
    let gap = (8.0 * scale).round() as i32;
    let (x, y) = match crate::strip::find_rect() {
        Some(r) => {
            let wa = crate::win::work_area_at((r.left + r.right) / 2, r.top);
            let x = ((r.left + r.right) / 2 - pw / 2).clamp(wa.left + gap, wa.right - pw - gap);
            // Görev çubuğu alttaysa üstüne, üstteyse altına.
            let y = if r.top >= wa.bottom - 2 { wa.bottom - ph - gap } else { wa.top + gap };
            (x, y)
        }
        None => {
            let wa = crate::win::primary_work_area();
            (wa.right - pw - gap, wa.bottom - ph - gap)
        }
    };
    let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
    let _ = w.show();
    if activate {
        ipc::post_main(ipc::WM_IPC_POPOVER, 1);
        if let Ok(h) = w.hwnd() { force_foreground(windows::Win32::Foundation::HWND(h.0 as *mut _)); }
        let _ = w.set_focus();
        // Görev çubuğu tıklamadan sonra odağı geri alır; kısa bir süre sonra bir kez daha öne getir.
        let app2 = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(150));
            let app3 = app2.clone();
            let _ = app2.run_on_main_thread(move || {
                if let Some(w) = app3.get_webview_window("popover") {
                    if let Ok(h) = w.hwnd() { force_foreground(windows::Win32::Foundation::HWND(h.0 as *mut _)); }
                    let _ = w.set_focus();
                    // Artık odak kaybı gerçektir: dışarı tıklanınca kapanır.
                    SHOWN_AT.store(now_ms(), Ordering::Relaxed);
                }
            });
        });
    }
}

fn limits_json(u: &ProviderUsage) -> Value {
    // 5 saatlik/haftalık dışındaki pencereler (Gemini günlük kota) ve havuzlar (Antigravity model grupları). Mac: otherRows, extra.
    let extra_row = |r: &crate::data::Row, window: f64| json!({
        "title": l10n::t(&r.title),
        "label": l10n::t(&r.title.replace(" · günlük kota", "")).to_uppercase(),
        "left": r.left,
        "pct": format!("{}%", (r.left * 100.0).round() as i64),
        "color": look::hex(look::status_color(r.left)),
        "reset": look::reset_text(r.reset),
        "remaining": r.reset.map(|x| if x > sense::now_secs() { look::format_remaining(x) } else { l10n::t("sıfırlandı") }),
        "expected": look::expected_left(r.left, r.reset, window),
    });
    let shown = u.confidence != crate::data::Confidence::None;
    let other: Vec<Value> = if shown { u.other_rows().into_iter().map(|r| extra_row(r, 86400.0)).collect() } else { vec![] };
    let pools: Vec<Value> = if shown && (u.extra.len() > 2 || !other.is_empty()) {
        u.extra.iter().map(|r| extra_row(r, if r.title.contains("Haftalık") { 7.0 * 86400.0 } else if r.title.contains("5 saatlik") { 5.0 * 3600.0 } else { 86400.0 })).collect()
    } else { vec![] };
    let pools_kind = if other.is_empty() { "HAVUZLAR" } else { "KOTALAR" };
    let Some(l) = look::shown_limits(u) else {
        if other.is_empty() && pools.is_empty() { return json!(null); }
        return json!({ "five": null, "week": null, "other": other, "pools": pools, "poolsKind": l10n::t(pools_kind) });
    };
    let row = |left: Option<f64>, reset: Option<f64>, window: f64| match left {
        None => json!(null),
        Some(v) => json!({
            "left": v,
            "pct": format!("{}%", (v * 100.0).round() as i64),
            "color": look::hex(look::status_color(v)),
            "reset": look::reset_text(reset),
            "remaining": reset.map(|r| if r > sense::now_secs() { look::format_remaining(r) } else { l10n::t("sıfırlandı") }),
            "expected": look::expected_left(v, reset, window),
        }),
    };
    json!({ "five": row(l.five_left, l.five_reset, 5.0 * 3600.0), "week": row(l.week_left, l.week_reset, 7.0 * 86400.0), "other": other, "pools": pools, "poolsKind": l10n::t(pools_kind) })
}

/// Son 7 günün günlük token toplamları (eskiden yeniye).
fn daily(p: &str) -> Vec<i64> {
    let today = store::start_of_day(sense::now_secs());
    (0..7).rev().map(|i| {
        let a = today - i as f64 * 86400.0;
        store::billable(p, a) - store::billable(p, a + 86400.0)
    }).collect()
}

/// Arayüz: ana sürecin yazdığı durumu okur (arayüz yeni açıldıysa yazılana kadar kısa süre bekler).
#[tauri::command]
pub async fn get_state() -> Value {
    for i in 0..30 {
        if let Some(v) = std::fs::read_to_string(ipc::state_file()).ok().and_then(|t| serde_json::from_str(&t).ok()) { return v; }
        if i == 0 { ipc::post_main(ipc::WM_IPC_HELLO, 0); }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    json!({ "providers": [], "dark": settings::get().dark, "en": l10n::english(), "tab": "all" })
}

/// Ana süreç: açılır pencerenin ve panonun okuduğu durum.
pub fn compute_state() -> Value {
    let s = settings::get();
    let snap = data::snapshot();
    let mut providers = Vec::new();
    for p in snap.ids() {
        if !s.is_enabled(p) { continue; }
        let u = snap.usage(p).cloned().unwrap_or_default();
        providers.push(json!({
            "id": p,
            "name": look::provider_name(p),
            "color": look::hex(look::provider_color(p)),
            "today": crate::cat::format_tokens(u.today_tokens),
            // Gemini'de token yerelde tutulmuyor (Mac: todayTokens nil): token kartı gösterilmez.
            "hasTokens": p != "gemini",
            "limits": limits_json(&u),
            "authExpired": u.auth_expired,
            "stale": u.stale_since.map(|t| look::when_text(t)),
            "notes": u.notes,
            "footnotes": look::footnotes(p, &u),
            "daily": daily(p),
        }));
    }
    // Ayarlar → Sağlayıcılar: dört çekirdek sağlayıcının durumu (açık olsun olmasın).
    let conf = |u: Option<&ProviderUsage>| match u.map(|u| u.confidence) {
        Some(crate::data::Confidence::Exact) => "exact", Some(crate::data::Confidence::Estimate) => "estimate", _ => "none" };
    let mut all = Vec::new();
    for p in snap.ids() {
        let u = snap.usage(p);
        let mut notes: Vec<String> = Vec::new();
        let mut token = "none";
        match p {
            "claude" => {
                token = "exact";
                notes.push(l10n::t("Token: ~/.claude kayıtlarından (Claude Code ve Desktop), cache okumaları hariç."));
                if let Some(l) = u.and_then(|u| u.limits.as_ref()) {
                    notes.push(l10n::t(match l.source {
                        crate::data::Source::Cli => "Limit: Claude Code komutundan ölçüldü (tek satırlık küçük bir istek).",
                        crate::data::Source::StatusLine => "Limit: Claude Code'un durum satırından (yerel, ağ gerekmez).",
                        _ => "Limit: Anthropic kullanım uç noktasından (resmi olmayan, Claude Code oturumuyla).",
                    }));
                }
                if let Some(st) = u.and_then(|u| u.stale_since) {
                    notes.push(l10n::f("Anthropic hız sınırı uyguladığı için son alınan değer gösteriliyor (%@); güncel olmayabilir.", &[look::when_text(st)]));
                }
            }
            "codex" => {
                if let Some(l) = u.and_then(|u| u.limits.as_ref()) {
                    token = "exact";
                    let (y, m, d, h, mi, _) = look::local(l.fetched_at);
                    notes.push(l10n::f("Limit ve token: Codex oturum kayıtlarından (son kayıt %@).", &[format!("{d:02}.{m:02}.{y} {h:02}:{mi:02}")]));
                    let now = sense::now_secs();
                    if l.five_reset.map(|r| r <= now).unwrap_or(false) { notes.push(l10n::t("Son kayıttan beri 5 saatlik pencere sıfırlandı; yeni kullanım olmadığı için %100 gösteriliyor.")); }
                    if l.week_reset.map(|r| r <= now).unwrap_or(false) { notes.push(l10n::t("Haftalık pencere sıfırlandı; yeni kullanım olmadığı için %100 gösteriliyor.")); }
                }
            }
            // Gemini, Antigravity ve ek araçlar: kendi notlarını taşır (aşağıda eklenir). Gemini kapalıyken hiç okunmaz.
            "gemini" => if u.is_none() { notes.push(l10n::t("Gemini: Google Code Assist kota API'si (Gemini CLI oturumuyla)")); },
            "antigravity" => if u.map(|u| u.today_tokens > 0 || crate::store::latest_event("antigravity") > 0.0).unwrap_or(false) { token = "estimate"; },
            _ => token = "exact",
        }
        if let Some(u) = u { for n in &u.notes { notes.push(n.clone()); } }
        all.push(json!({ "id": p, "name": look::provider_name(p), "limit": conf(u), "token": token, "notes": notes, "on": s.is_enabled(p) }));
    }
    json!({
        "all": all,
        "dark": s.dark,
        "en": l10n::english(),
        "includeCache": s.include_cache,
        "updatedAt": (snap.updated_at > 0.0).then(|| look::when_text(snap.updated_at)),
        "tab": s.selected_tab,
        "providers": providers,
        "catName": l10n::cat_name(),
        "detected": detected_tools(),
    })
}

/// Bu bilgisayarda izi bulunan yapay zekâ araçları (kayıt klasörü var ya da daha önce olay kaydedilmiş).
/// Boşsa sayfalar "araç bulunamadı" uyarısı gösterir.
pub fn detected_tools() -> Vec<&'static str> {
    let home = std::path::PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default());
    let seen = |p: &str| crate::store::latest_event(p) > 0.0;
    let mut v = Vec::new();
    if home.join(".claude").join("projects").exists() || seen("claude") { v.push("claude"); }
    if home.join(".codex").join("sessions").exists() || seen("codex") { v.push("codex"); }
    if home.join(".gemini").join("oauth_creds.json").exists() { v.push("gemini"); }
    if home.join(".gemini").join("antigravity-ide").exists() || seen("antigravity") { v.push("antigravity"); }
    v.extend(crate::extras::present());
    v
}

/// Sayfaların kullandığı metinler (Türkçe anahtar → seçili dil, kedinin adı işlenmiş).
#[tauri::command]
pub fn tr_all() -> Value { l10n::table() }

#[tauri::command]
pub fn tr(keys: Vec<String>) -> Value {
    let m: serde_json::Map<String, Value> = keys.into_iter().map(|k| { let v = l10n::t(&k); (k, Value::String(v)) }).collect();
    Value::Object(m)
}

#[tauri::command]
pub fn set_tab(tab: String) {
    settings::update(|s| s.selected_tab = tab);
    ipc::post_main(ipc::WM_IPC_SETTINGS, 0);
}

#[tauri::command]
pub fn refresh_now() { ipc::post_main(ipc::WM_IPC_REFRESH, 0); }

#[tauri::command]
pub fn quit(app: AppHandle) {
    ipc::post_main(ipc::WM_IPC_QUIT, 0);
    app.exit(0);
}

#[tauri::command]
pub async fn open_dashboard(app: AppHandle, page: String) {
    if let Some(w) = app.get_webview_window("popover") { let _ = w.close(); }
    crate::dashboard::open(&app, &page);
}

/// Arayüz: ana süreç durumu yazınca açık sayfalara haber ver (dosya izlenir, yoklama yok).
pub fn watch_state(app: AppHandle) {
    std::thread::spawn(move || {
        use notify::{RecursiveMode, Watcher};
        use tauri::Emitter;
        let (tx, rx) = std::sync::mpsc::channel();
        let Ok(mut w) = notify::recommended_watcher(tx) else { return };
        let _ = w.watch(&settings::data_dir(), RecursiveMode::NonRecursive);
        for ev in rx {
            let Ok(ev) = ev else { continue };
            if ev.paths.iter().any(|p| p.file_name().map(|n| n == "ui_state.json").unwrap_or(false)) {
                let _ = app.emit("meowly-state", ());
            }
        }
    });
}
