//! İlk açılış: kedinin adını soran pencere ve beş adımlık tanıtım turu (Mac: Tonti/CatNameUI.swift → CatNamePrompt,
//! Tonti/Tour.swift → Tour, TourCard). Arayüz sürecinde. Hiçbir şey okunmaz ya da gönderilmez; yalnızca iki küçük pencere
//! çizilir. Ad penceresi yalnızca ad hiç girilmemişse çıkar; tur yalnızca ad girildikten sonra, bir kez (Ayarlar → Hakkında'dan tekrar).

use crate::{l10n, settings};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const CARD_W: f64 = 344.0;
const CARD_H: f64 = 246.0;

static INDEX: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// Pencere içi öğenin konumunu bekleyen adım (dashboard'dan yanıt gelince kart yerleşir).
static WAITING: Mutex<Option<usize>> = Mutex::new(None);
/// Kartın şu anki içeriği (sayfa açılınca kendisi de ister).
static CARD: Mutex<Option<Value>> = Mutex::new(None);

#[derive(Clone, Copy)]
enum Target { Strip, Cat, Window(&'static str, &'static str) }
#[derive(Clone, Copy, PartialEq)]
enum Arrow { Up, Down, Left, Right, None }

struct Step { title: &'static str, body: &'static str, target: Target, art: &'static str, prefer: Arrow }

/// Mac'teki beş adım; "menü çubuğu" ve "Mac" geçen iki metin Windows'a uyarlandı.
fn steps() -> [Step; 5] {
    [
        Step { title: "Limitlerin burada",
               body: "Görev çubuğunda 5 saatlik ve haftalık kalan limitini görürsün. Soldaki kısma basınca ayrıntılar açılır; sağdaki kutuya basınca ben kutuya girerim.",
               target: Target::Strip, art: "type_0", prefer: Arrow::Up },
        Step { title: "Bu benim, Tonti",
               body: "Üstüme gelip imleci gezdirirsen mırlarım. Enseden tutup istediğin yere taşıyabilirsin. Bana tıklarsan uygulamayı açarım.",
               target: Target::Cat, art: "content", prefer: Arrow::Right },
        Step { title: "Çalışırken not alırım",
               body: "Claude ya da Codex ile çalışınca Günlük'e yazarım. Raporlar'da günlük, haftalık ve aylık özetin hazır olur; Kullanım'da token'ların. Sohbetlerini okumam.",
               target: Target::Window("sidebar.nav", "diary"), art: "idle_2", prefer: Arrow::Left },
        Step { title: "Her şey ayarlarda",
               body: "Limit göstergesini, balonları ve bilgisayar günlüğünün her katmanını tek tek açıp kapatabilirsin. İzin gereken şeyleri sen onaylarsın; kendiliğinden hiçbir izin istemem.",
               target: Target::Window("settings.tabs", "appearance"), art: "groomC", prefer: Arrow::Up },
        Step { title: "Hepsi bu bilgisayarda kalır",
               body: "Burada neyi not alıp neyi asla almadığımı görürsün. Sunucum ve hesabım yok; hiçbir şey gönderilmez. Bu turu Ayarlar → Hakkında'dan yeniden görebilirsin.",
               target: Target::Window("computer.privacy", "computer"), art: "sleep", prefer: Arrow::Up },
    ]
}

// MARK: Ad penceresi

/// Kullanıcı ad penceresini X ile kapattı: güvence onu yeniden açmaz.
pub static PROMPT_CLOSED: AtomicBool = AtomicBool::new(false);

/// `--ui prompt:name`: ad hiç girilmemişse küçük pencereyi açar.
pub fn show_name_prompt(app: &AppHandle) {
    // Geliştirme: MEOWLY_TEST_SKIP_PROMPT ilk açılışı atlar (güvencenin pencereyi açtığını denemek için).
    if std::env::var_os("MEOWLY_TEST_SKIP_PROMPT").is_none() { open_name_prompt(app); }
    // Güvence: WebView2 o anda başlayamazsa (ör. kendini güncellerken) pencere görünmeden kaybolabiliyor;
    // 4 sn sonra ad hâlâ girilmemiş ve pencere yoksa bir kez daha açılır.
    let a = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        if settings::get().cat_name.is_some() || PROMPT_CLOSED.load(Ordering::SeqCst) { return; }
        let ok = a.get_webview_window("prompt").map(|w| w.is_visible().unwrap_or(false)).unwrap_or(false);
        if !ok { crate::ui_app::err_log("ad penceresi görünmedi, yeniden açılıyor"); open_name_prompt(&a); }
    });
}

fn open_name_prompt(app: &AppHandle) {
    crate::ui_app::on_main(app, |app| {
        if settings::get().cat_name.is_some() { return; }
        if let Some(w) = app.get_webview_window("prompt") { crate::popover::show_window(&w); if crate::popover::may_focus() { let _ = w.set_focus(); } return; }
        let dark = settings::get().dark;
        let built = crate::ui_app::building(|| WebviewWindowBuilder::new(app, "prompt", WebviewUrl::App("prompt.html".into()))
            .title("Meowly")
            .inner_size(380.0, 300.0)
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            .always_on_top(true)
            .theme(Some(if dark { tauri::Theme::Dark } else { tauri::Theme::Light }))
            .center()
            .focused(crate::popover::may_focus())
            .visible(crate::popover::may_focus())
            .build());
        if let Err(e) = &built { crate::ui_app::err_log(format!("pencere açılamadı: {e}")); }
        if let Ok(w) = built {
            crate::popover::show_window(&w);
            if let Ok(h) = w.hwnd() { crate::popover::force_foreground(windows::Win32::Foundation::HWND(h.0 as *mut _)); }
            if crate::popover::may_focus() { let _ = w.set_focus(); }
        }
    });
}

/// "TAMAM": adı kaydeder, pencereyi kapatır, tur hiç gösterilmediyse başlatır (Mac: 0,7 sn sonra).
#[tauri::command]
pub async fn name_done(app: AppHandle, name: String) {
    crate::settings_ui::save_cat_name(name);
    if let Some(w) = app.get_webview_window("prompt") { let _ = w.destroy(); }
    if !settings::get().tour_done {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(700));
            start(&app);
        });
    }
}

// MARK: Tur

/// Turu baştan başlatır (Ayarlar → Hakkında → "Turu göster" ya da ad girildikten sonra).
pub fn start(app: &AppHandle) {
    if ACTIVE.swap(true, Ordering::SeqCst) { return; }
    // Geliştirme: MEOWLY_TOUR_STEP=n belirli adımdan başlatır (Mac ile aynı).
    let first = std::env::var("MEOWLY_TOUR_STEP").ok().and_then(|v| v.parse().ok()).unwrap_or(0usize);
    INDEX.store(first.min(4), Ordering::SeqCst);
    show(app);
}

#[tauri::command]
pub async fn tour_start(app: AppHandle) { start(&app); }

#[tauri::command]
pub async fn tour_next(app: AppHandle) {
    let i = INDEX.fetch_add(1, Ordering::SeqCst) + 1;
    if i >= steps().len() { finish(&app); } else { show(&app); }
}

#[tauri::command]
pub async fn tour_skip(app: AppHandle) { finish(&app); }

/// Kart sayfası açılınca kendi içeriğini ister.
#[tauri::command]
pub fn tour_card() -> Value { CARD.lock().unwrap().clone().unwrap_or(Value::Null) }

fn finish(app: &AppHandle) {
    ACTIVE.store(false, Ordering::SeqCst);
    *WAITING.lock().unwrap() = None;
    settings::update(|s| s.tour_done = true);
    crate::ipc::post_main(crate::ipc::WM_IPC_SETTINGS, 0);
    let app = app.clone();
    crate::ui_app::on_main(&app.clone(), move |_| {
        if let Some(w) = app.get_webview_window("tour") { let _ = w.destroy(); }
    });
}

fn show(app: &AppHandle) {
    let i = INDEX.load(Ordering::SeqCst);
    let all = steps();
    let Some(st) = all.get(i) else { finish(app); return };
    match st.target {
        Target::Window(id, page) => {
            // Önce ilgili sayfa açılır; yerleşim bitince pano öğenin ekran konumunu bildirir, kart onun yanına konur.
            *WAITING.lock().unwrap() = Some(i);
            crate::dashboard::open(app, page);
            let app = app.clone();
            std::thread::spawn(move || {
                // Yeni açılan pano dinlemeye birkaç yüz ms sonra başlar: yanıt gelene kadar birkaç kez sorulur.
                for _ in 0..12 {
                    std::thread::sleep(std::time::Duration::from_millis(300));
                    if *WAITING.lock().unwrap() != Some(i) { return; }
                    let _ = app.emit("meowly-tour-anchor", id);
                }
                // Öğe bulunamadı: kart ekranın ortasına, oksuz.
                if WAITING.lock().unwrap().take() == Some(i) { place(&app, i, None); }
            });
        }
        Target::Strip => place(app, i, crate::strip::find_rect().map(rect_of)),
        Target::Cat => {
            let r = unsafe { windows::Win32::UI::WindowsAndMessaging::FindWindowW(windows::core::w!("MeowlyCat"), None) }
                .ok().and_then(|h| crate::win::window_rect(h)).map(rect_of);
            place(app, i, r.or_else(|| crate::strip::find_rect().map(rect_of)));
        }
    }
}

/// Pano, istenen öğenin pencere içi konumunu (CSS px) bildirir.
#[tauri::command]
pub fn tour_anchor(app: AppHandle, x: f64, y: f64, w: f64, h: f64) {
    let Some(i) = WAITING.lock().unwrap().take() else { return };
    let Some(win) = app.get_webview_window("dashboard") else { place(&app, i, None); return };
    let s = win.scale_factor().unwrap_or(1.0);
    let Ok(p) = win.inner_position() else { place(&app, i, None); return };
    let r = (p.x as f64 + x * s, p.y as f64 + y * s, p.x as f64 + (x + w) * s, p.y as f64 + (y + h) * s);
    place(&app, i, Some(r));
}

fn rect_of(r: windows::Win32::Foundation::RECT) -> (f64, f64, f64, f64) { (r.left as f64, r.top as f64, r.right as f64, r.bottom as f64) }

/// Kartı hedefin yanına yerleştirir (fiziksel piksel). Tercih edilen tarafa sığmazsa karşı tarafa geçer (Mac ile aynı kural;
/// y ekseni Windows'ta aşağı doğru: "up" = ok yukarı bakar, kart hedefin altında).
fn place(app: &AppHandle, i: usize, target: Option<(f64, f64, f64, f64)>) {
    let all = steps();
    let st = &all[i];
    let (cx, cy) = target.map(|t| ((t.0 + t.2) / 2.0, (t.1 + t.3) / 2.0)).unwrap_or_else(|| {
        let wa = crate::win::primary_work_area();
        (((wa.left + wa.right) / 2) as f64, ((wa.top + wa.bottom) / 2) as f64)
    });
    let wa = crate::win::work_area_at(cx as i32, cy as i32);
    let scale = crate::win::scale_at(cx as i32, cy as i32);
    let (w, h) = (CARD_W * scale, CARD_H * scale);
    let (sl, st_, sr, sb) = (wa.left as f64, wa.top as f64, wa.right as f64, wa.bottom as f64);
    let clamp_x = |x: f64| x.max(sl + 8.0 * scale).min(sr - w - 8.0 * scale);
    let clamp_y = |y: f64| y.max(st_ + 8.0 * scale).min(sb - h - 8.0 * scale);
    let mut origin = ((sl + sr) / 2.0 - w / 2.0, (st_ + sb) / 2.0 - h / 2.0 - 40.0 * scale);
    let mut arrow = Arrow::None;
    let mut at = 0.0;
    if let Some(t) = target {
        let mut dir = st.prefer;
        if dir == Arrow::Right && t.0 - w < sl { dir = Arrow::Left; }
        if dir == Arrow::Left && t.2 + w > sr { dir = Arrow::Right; }
        if dir == Arrow::Up && t.3 + h > sb { dir = Arrow::Down; }
        match dir {
            Arrow::Up => { origin = (clamp_x(cx - w / 2.0), t.3 - 4.0 * scale); at = cx - origin.0; }
            Arrow::Down => { origin = (clamp_x(cx - w / 2.0), t.1 - h + 4.0 * scale); at = cx - origin.0; }
            Arrow::Right => { origin = (t.0 - w + 2.0 * scale, clamp_y(cy - h / 2.0)); at = cy - origin.1; }
            Arrow::Left => { origin = (t.2 - 2.0 * scale, clamp_y(cy - h / 2.0)); at = cy - origin.1; }
            Arrow::None => {}
        }
        arrow = dir;
    }
    let card = json!({
        "index": i, "count": all.len(),
        "title": l10n::t(st.title), "body": l10n::t(st.body), "art": st.art,
        "arrow": match arrow { Arrow::Up => "up", Arrow::Down => "down", Arrow::Left => "left", Arrow::Right => "right", Arrow::None => "none" },
        "at": at / scale,
        "skip": l10n::t("ATLA"), "next": l10n::t(if i + 1 < all.len() { "İLERİ →" } else { "BAŞLAYALIM" }),
        "dark": settings::get().dark,
    });
    *CARD.lock().unwrap() = Some(card.clone());
    let app2 = app.clone();
    crate::ui_app::on_main(app, move |_| {
        let w = match app2.get_webview_window("tour") {
            Some(w) => w,
            None => {
                let Ok(w) = crate::ui_app::building(|| WebviewWindowBuilder::new(&app2, "tour", WebviewUrl::App("tour.html".into()))
                    .title("Meowly")
                    .inner_size(CARD_W, CARD_H)
                    .decorations(false)
                    .transparent(true)
                    .shadow(false)
                    .resizable(false)
                    .skip_taskbar(true)
                    .always_on_top(true)
                    .focused(false)
                    .visible(false)
                    .build()).map_err(|e| crate::ui_app::err_log(format!("tur kartı açılamadı: {e}"))) else { return };
                w
            }
        };
        let _ = w.set_position(tauri::PhysicalPosition::new(origin.0.round() as i32, origin.1.round() as i32));
        let _ = w.set_size(tauri::PhysicalSize::new(w_px(scale), h_px(scale)));
        let _ = app2.emit("meowly-tour", card);
        crate::popover::show_window(&w);
    });
}

fn w_px(scale: f64) -> u32 { (CARD_W * scale).round() as u32 }
fn h_px(scale: f64) -> u32 { (CARD_H * scale).round() as u32 }

// MARK: Yıldız ricası (Mac: Tonti/StarPrompt.swift)

/// En az 7 gün kullanımdan sonra bir kez; "Sonra" denirse 14 gün sonra son bir kez; sonra hiç. Ad girilmeden sorulmaz.
pub fn star_due(now: f64) -> bool {
    if std::env::var_os("MEOWLY_STAR").is_some() { return true; } // geliştirme
    let s = settings::get();
    if s.cat_name.is_none() || s.first_use_at <= 0.0 { return false; }
    match s.star_state {
        0 => now - s.first_use_at >= 7.0 * 86400.0,
        1 => now - s.star_ask_at >= 14.0 * 86400.0,
        _ => false,
    }
}

/// `--ui prompt:star`
pub fn show_star_prompt(app: &AppHandle) {
    crate::ui_app::on_main(app, |app| {
        if let Some(w) = app.get_webview_window("star") { crate::popover::show_window(&w); if crate::popover::may_focus() { let _ = w.set_focus(); } return; }
        let dark = settings::get().dark;
        let built = crate::ui_app::building(|| WebviewWindowBuilder::new(app, "star", WebviewUrl::App("star.html".into()))
            .title("Meowly")
            .inner_size(400.0, 300.0)
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            .always_on_top(true)
            .theme(Some(if dark { tauri::Theme::Dark } else { tauri::Theme::Light }))
            .center()
            .focused(crate::popover::may_focus())
            .visible(crate::popover::may_focus())
            .build());
        if let Err(e) = &built { crate::ui_app::err_log(format!("pencere açılamadı: {e}")); }
        if let Ok(w) = built {
            crate::popover::show_window(&w);
            if let Ok(h) = w.hwnd() { crate::popover::force_foreground(windows::Win32::Foundation::HWND(h.0 as *mut _)); }
            if crate::popover::may_focus() { let _ = w.set_focus(); }
        }
    });
}

/// "⭐ YILDIZ VER" (depo sayfasını açar), "SONRA" (ilk seferde 14 gün sonra bir kez daha), "Bir daha sorma".
#[tauri::command]
pub async fn star_done(app: AppHandle, choice: String) {
    if choice == "star" { crate::settings_ui::open_link("repo".into()); }
    let now = crate::sense::now_secs();
    settings::update(|s| {
        s.star_state = if choice == "later" && s.star_state == 0 { 1 } else { 2 };
        s.star_ask_at = now;
    });
    crate::ipc::post_main(crate::ipc::WM_IPC_SETTINGS, 0);
    if let Some(w) = app.get_webview_window("star") { let _ = w.destroy(); }
}