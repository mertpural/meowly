//! Ana pencere (Mac: Dashboard.swift, Tonti/TontiUI.swift → TontiRoot). Kapanınca bellekten tamamen bırakılır.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Sayfa adları Mac ile aynı: diary, reports, usage, settings (ve ayar sekmeleri).
pub fn open(app: &AppHandle, page: &str) {
    let page = page.to_string();
    crate::ui_app::on_main(app, move |app2| {
        let app2 = app2.clone();
        if let Some(w) = app2.get_webview_window("dashboard") {
            let _ = app2.emit("meowly-page", page);
            let _ = w.unminimize();
            crate::popover::show_window(&w);
            front(&w);
            return;
        }
        let url = format!("dashboard.html#{page}");
        let dark = crate::settings::get().dark;
        let built = crate::ui_app::building(|| WebviewWindowBuilder::new(&app2, "dashboard", WebviewUrl::App(url.into()))
            .title("Meowly")
            .inner_size(1000.0, 666.0)
            .min_inner_size(820.0, 560.0)
            .theme(Some(if dark { tauri::Theme::Dark } else { tauri::Theme::Light }))
            .center()
            .focused(crate::popover::may_focus())
            .visible(crate::popover::may_focus())
            .build());
        match built { Ok(w) => { crate::popover::show_window(&w); front(&w) } Err(e) => crate::ui_app::err_log(format!("pano açılamadı: {e}")) }
    });
}

/// Pencereyi öne getirir (arka plandaki uygulamanın öne gelme kısıtını aşarak).
fn front(w: &tauri::WebviewWindow) {
    if let Ok(h) = w.hwnd() { crate::popover::force_foreground(windows::Win32::Foundation::HWND(h.0 as *mut _)); }
    if crate::popover::may_focus() { let _ = w.set_focus(); }
}
