//! Görev çubuğu limit şeridi (Mac: menü çubuğu çipi — Views.swift MenuBarLabel.chip, Tonti/StatusItem.swift).
//! Saatin hemen solunda, görev çubuğu penceresinin içine gömülü küçük katmanlı pencere: görev çubuğu otomatik
//! gizlenince ya da tam ekran uygulama/oyun açılınca onunla birlikte gizlenir. Sol kısım açılır pencereyi açar,
//! sağdaki kutu kediyi kutuya koyar/çıkarır.

use crate::img::{rgba, Img};
use crate::look::{self, Rgb};
use crate::text::{self, Weight};
use crate::{cat, data, l10n, settings, win};
use std::cell::RefCell;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

thread_local! {
    static STRIP: RefCell<Option<Strip>> = const { RefCell::new(None) };
}

struct Strip {
    hwnd: HWND,
    surf: win::Surface,
    box_w: i32,
    w: i32,
    h: i32,
    embedded: bool,
}

const CHIP_H: f64 = 18.0;
const BOX_W: f64 = 30.0;

fn taskbar() -> HWND { unsafe { FindWindowW(w!("Shell_TrayWnd"), None).unwrap_or_default() } }

fn tray_notify(tb: HWND) -> Option<RECT> {
    unsafe {
        let n = FindWindowExW(Some(tb), None, w!("TrayNotifyWnd"), None).ok()?;
        let mut r = RECT::default();
        GetWindowRect(n, &mut r).ok()?;
        (r.right > r.left).then_some(r)
    }
}

/// Açık sağlayıcılar (Mac'teki sırayla; ek araçlar yalnızca izi varsa).
fn enabled() -> Vec<&'static str> {
    let s = settings::get();
    data::snapshot().ids().into_iter().filter(|p| s.is_enabled(p)).collect()
}

/// Çipi çizer: [logo] 5s 69% H 83% + bitişik kutu. Tüm ölçüler pt × ölçek.
fn render(s: f64) -> (Img, i32) {
    let st = settings::get();
    let dark = st.dark;
    let fill: Rgb = if dark { (15, 15, 17) } else { (255, 255, 255) };
    let number: Rgb = if dark { (255, 255, 255) } else { (22, 12, 12) };
    let tag: Rgb = if dark { (242, 158, 179) } else { (204, 89, 117) };
    let snap = data::snapshot();
    let en = l10n::english();

    // Önce öğeleri ölç: (genişlik pt, çizici)
    enum Item { Logo(&'static str), Tag(String), Glyph(Option<f64>, String), Gap(f64) }
    let mut items: Vec<Item> = Vec::new();
    for (i, p) in enabled().into_iter().enumerate() {
        if i > 0 { items.push(Item::Gap(12.0)); }
        let u = snap.usage(p);
        let l = u.and_then(look::shown_limits);
        // 5 saatlik/haftalık yoksa (Gemini günlük kota) en düşük diğer pencere tek göstergeyle, "G"/"D" etiketiyle (Mac: MenuBarLabel).
        let other = u.filter(|u| u.confidence != data::Confidence::None)
            .and_then(|u| u.other_rows().iter().map(|r| r.left).min_by(|a, b| a.partial_cmp(b).unwrap()));
        items.push(Item::Logo(p));
        let mut glyph = |left: Option<f64>, style: &str, t: &str| {
            items.push(Item::Gap(5.0));
            if st.show_tags { items.push(Item::Tag(t.into())); items.push(Item::Gap(3.0)); }
            items.push(Item::Glyph(left, style.into()));
        };
        let five = l.and_then(|l| l.five_left);
        let week = l.and_then(|l| l.week_left);
        if five.is_none() && week.is_none() && other.is_some() {
            glyph(other, &st.style_five, if en { "D" } else { "G" });
            continue;
        }
        if st.show_five { glyph(five, &st.style_five, if en { "5h" } else { "5s" }); }
        if st.show_week { glyph(week, &st.style_week, if en { "W" } else { "H" }); }
    }
    let pct_w = |left: Option<f64>| match left {
        Some(l) => text::width(&format!("{}%", (l * 100.0).round() as i64), 13.0, Weight::Bold) as f64,
        None => text::width("–", 11.0, Weight::Regular) as f64,
    };
    let glyph_w = |left: Option<f64>, style: &str| -> f64 {
        if left.is_none() { return pct_w(None); }
        match style {
            "bar" => 26.0,
            "barPercent" => 22.0 + 3.0 + pct_w(left),
            "ring" => 14.0,
            "dot" => 8.0,
            "battery" => 29.0,
            "segments" => 5.0 * 4.0 + 4.0 * 1.5,
            _ => pct_w(left),
        }
    };
    let width_of = |it: &Item| match it {
        Item::Logo(_) => 15.0,
        Item::Tag(t) => text::width(t, 9.0, Weight::Bold) as f64,
        Item::Glyph(l, st) => glyph_w(*l, st),
        Item::Gap(g) => *g,
    };
    let content_w: f64 = items.iter().map(&width_of).sum();
    let chip_w = if items.is_empty() { 0.0 } else { content_w + 20.0 };
    let total_w = chip_w + BOX_W;
    let (wpx, hpx) = ((total_w * s).round() as usize, (CHIP_H * s).round() as usize);
    let mut out = Img::new(wpx, hpx);
    // Zemin: 3 pt köşeli dikdörtgen.
    rounded(&mut out, 0.0, 0.0, wpx as f64, hpx as f64, 3.0 * s, fill);
    let mid = hpx as f64 / 2.0;
    let mut x = 10.0 * s;
    for it in &items {
        let wpt = width_of(it);
        match it {
            Item::Logo(p) => draw_logo(&mut out, p, x, mid, 15.0 * s),
            Item::Tag(t) => draw_text_mid(&mut out, t, x, mid, 9.0 * s, Weight::Bold, tag),
            Item::Glyph(left, style) => draw_glyph(&mut out, *left, style, x, mid, s, number),
            Item::Gap(_) => {}
        }
        x += wpt * s;
    }
    draw_box(&mut out, chip_w * s, s, cat::in_box(), dark, number);
    (out, (BOX_W * s).round() as i32)
}

fn rounded(out: &mut Img, x: f64, y: f64, w: f64, h: f64, r: f64, c: Rgb) {
    out.fill_aa(x + r, y, w - 2.0 * r, h, c, 1.0);
    out.fill_aa(x, y + r, r, h - 2.0 * r, c, 1.0);
    out.fill_aa(x + w - r, y + r, r, h - 2.0 * r, c, 1.0);
    for (cx, cy) in [(x + r, y + r), (x + w - r, y + r), (x + r, y + h - r), (x + w - r, y + h - r)] {
        let qx = if cx < x + w / 2.0 { x } else { cx };
        let qy = if cy < y + h / 2.0 { y } else { cy };
        let mut tmp = Img::new(out.w, out.h);
        tmp.circle(cx, cy, r, 0.0, 1.0, c, 1.0);
        for py in qy.floor() as i32..(qy + r).ceil() as i32 {
            for px in qx.floor() as i32..(qx + r).ceil() as i32 {
                if px >= 0 && py >= 0 && (px as usize) < out.w && (py as usize) < out.h {
                    let p = tmp.px[py as usize * out.w + px as usize];
                    out.over(px, py, p);
                }
            }
        }
    }
}

fn draw_text_mid(out: &mut Img, t: &str, x: f64, mid: f64, size: f64, wt: Weight, c: Rgb) {
    let (a, d, _) = text::line_metrics(size as f32, wt);
    let base = mid + (a - d) as f64 / 2.0;
    text::draw(out, t, x as f32, base as f32, size as f32, wt, c, 1.0);
}

fn draw_logo(out: &mut Img, p: &str, x: f64, mid: f64, size: f64) {
    // Ek araçların logosu yok: Mac'teki gibi harfli renkli rozet.
    if let Some((letter, c)) = match p { "opencode" => Some(("O", (77, 204, 140))), "kimi" => Some(("K", (133, 92, 235))), "cline" => Some(("L", (242, 191, 64))), _ => None } {
        rounded(out, x, mid - size / 2.0, size, size, size * 0.28, c);
        let tw = text::width(letter, (size * 0.62) as f32, Weight::Bold) as f64;
        draw_text_mid(out, letter, x + (size - tw) / 2.0, mid, size * 0.62, Weight::Bold, (255, 255, 255));
        return;
    }
    let bytes: &[u8] = match p {
        "claude" => include_bytes!("../../../Resources/Logos/claude.png"),
        "codex" => include_bytes!("../../../Resources/Logos/codex.png"),
        "gemini" => include_bytes!("../../../Resources/Logos/gemini.png"),
        _ => include_bytes!("../../../Resources/Logos/antigravity.png"),
    };
    let src = Img::decode_png(bytes);
    let sc = size / src.w.max(src.h) as f64;
    let mut img = src.resample(sc);
    if p == "codex" {
        // Tek renkli maske: alfa şekli verir, renk marka grisi.
        let c = (158, 161, 168);
        for px in img.px.iter_mut() { let a = px[3]; *px = rgba(c.0, c.1, c.2, a as f32 / 255.0); }
    }
    let y = (mid - img.h as f64 / 2.0).round() as i32;
    out.blit_rows(&img, 0, img.h, x.round() as i32, y, img.h, 1.0, &|_| 0);
}

fn draw_glyph(out: &mut Img, left: Option<f64>, style: &str, x: f64, mid: f64, s: f64, text_c: Rgb) {
    let Some(l) = left else {
        draw_text_mid(out, "–", x, mid, 11.0 * s, Weight::Regular, text_c);
        return;
    };
    let pct = format!("{}%", (l * 100.0).round() as i64);
    let sc = look::status_color(l);
    let gray: Rgb = (128, 128, 128);
    let mini_bar = |out: &mut Img, x: f64, w: f64| {
        let (h, y) = (4.0 * s, mid - 2.0 * s);
        rounded(out, x, y, w * s, h, 2.0 * s, blend(gray, 0.35, out_bg(out)));
        rounded(out, x, y, (w * l).max(2.0) * s, h, 2.0 * s, sc);
    };
    match style {
        "bar" => mini_bar(out, x, 26.0),
        "barPercent" => {
            mini_bar(out, x, 22.0);
            draw_text_mid(out, &pct, x + 25.0 * s, mid, 13.0 * s, Weight::Bold, text_c);
        }
        "ring" => {
            let (cx, r) = (x + 7.0 * s, (14.0 - 2.5) / 2.0 * s);
            out.circle(cx, mid, r, 2.5 * s, 1.0, gray, 0.35);
            out.circle(cx, mid, r, 2.5 * s, l.max(0.02), sc, 1.0);
        }
        "dot" => out.circle(x + 4.0 * s, mid, 4.0 * s, 0.0, 1.0, sc, 1.0),
        "battery" => {
            let (bw, bh) = (26.0 * s, 12.0 * s);
            let y = mid - bh / 2.0;
            // çerçeve (1 pt, mürekkep %55)
            out.fill_aa(x, y, bw, 1.0 * s, text_c, 0.55);
            out.fill_aa(x, y + bh - 1.0 * s, bw, 1.0 * s, text_c, 0.55);
            out.fill_aa(x, y, 1.0 * s, bh, text_c, 0.55);
            out.fill_aa(x + bw - 1.0 * s, y, 1.0 * s, bh, text_c, 0.55);
            out.fill_aa(x + 1.0 * s, y + 1.0 * s, (24.0 * l).max(2.0) * s, bh - 2.0 * s, sc, 0.85);
            let n = format!("{}", (l * 100.0).round() as i64);
            let tw = text::width(&n, 8.0 * s as f32, Weight::Bold) as f64;
            draw_text_mid(out, &n, x + (bw - tw) / 2.0, mid, 8.0 * s, Weight::Bold, text_c);
            out.fill_aa(x + bw + 1.0 * s, mid - 2.5 * s, 2.0 * s, 5.0 * s, text_c, 0.55);
        }
        "segments" => {
            let n = (l * 5.0).ceil();
            for i in 0..5 {
                let on = (i as f64) < n;
                let c = if on { sc } else { text_c };
                out.fill_aa(x + i as f64 * 5.5 * s, mid - 5.0 * s, 4.0 * s, 10.0 * s, c, if on { 1.0 } else { 0.2 });
            }
        }
        _ => draw_text_mid(out, &pct, x, mid, 13.0 * s, Weight::Bold, text_c),
    }
}

fn out_bg(out: &Img) -> Rgb { let p = out.px[0]; (p[2], p[1], p[0]) }
fn blend(c: Rgb, a: f64, bg: Rgb) -> Rgb {
    let m = |x: u8, y: u8| (x as f64 * a + y as f64 * (1.0 - a)).round() as u8;
    (m(c.0, bg.0), m(c.1, bg.1), m(c.2, bg.2))
}

/// Kutu simgesi (Mac: CatController.boxIcon). 30×18 pt; kedi içindeyse başı görünür.
fn draw_box(out: &mut Img, x0: f64, s: f64, with_cat: bool, dark: bool, ink: Rgb) {
    let fill: Rgb = if dark { (15, 15, 17) } else { (255, 255, 255) };
    let h = CHIP_H;
    // Mac koordinatları alttan yukarı: y_aşağı = h - y - yükseklik
    let r = |out: &mut Img, x: f64, y: f64, w: f64, hh: f64, c: Rgb| out.fill_aa(x0 + x * s, (h - y - hh) * s, w * s, hh * s, c, 1.0);
    let (bx, by, bw, bh) = (7.0, 2.0, 16.0, 8.0);
    if with_cat {
        let src = Img::decode_png(include_bytes!("../../../Resources/Cat/idle_0.png"));
        let crop_h = (src.h as f64 * 0.55) as usize;
        let mut head = Img::new(src.w, crop_h);
        head.px.copy_from_slice(&src.px[..src.w * crop_h]);
        let hh = 12.0;
        let ww = hh * head.w as f64 / head.h as f64;
        let img = head.resample(hh * s / head.h as f64);
        let dx = x0 + (BOX_W - ww) / 2.0 * s;
        let dy = (h - 5.0 - hh) * s;
        out.blit_rows(&img, 0, img.h, dx.round() as i32, dy.round() as i32, img.h, 1.0, &|_| 0);
    }
    r(out, bx, by, bw, bh, fill);
    r(out, bx, by, bw, 1.5, ink);
    r(out, bx, by, 1.5, bh, ink);
    r(out, bx + bw - 1.5, by, 1.5, bh, ink);
    r(out, bx - 1.0, by + bh - 1.5, bw + 2.0, 1.5, ink);
    r(out, bx + bw / 2.0 - 3.0, by + 3.5, 6.0, 1.5, ink);
    if !with_cat {
        r(out, bx - 3.0, by + bh - 1.0, 5.0, 1.5, ink);
        r(out, bx + bw - 2.0, by + bh - 1.0, 5.0, 1.5, ink);
    }
}

impl Strip {
    /// Görev çubuğundaki yerini bulur ve çizer.
    fn refresh(&mut self) {
        let s = win::dpi_scale(self.hwnd);
        let (img, box_w) = render(s);
        self.box_w = box_w;
        self.w = img.w as i32;
        self.h = img.h as i32;
        let tb = taskbar();
        let mut tr = RECT::default();
        unsafe { let _ = GetWindowRect(tb, &mut tr); }
        let notify = tray_notify(tb);
        let gap = (6.0 * s).round() as i32;
        // Ekran koordinatı: saatin/tepsinin solu, görev çubuğunda dikey ortalı.
        let (sx, sy) = match notify {
            Some(n) => (n.left - gap - self.w, tr.top + ((tr.bottom - tr.top) - self.h) / 2),
            None => (tr.right - 320 - self.w, tr.top + ((tr.bottom - tr.top) - self.h) / 2),
        };
        let (x, y) = if self.embedded { (sx - tr.left, sy - tr.top) } else { (sx, sy) };
        self.surf.present(self.hwnd, &img, x, y, 255);
        if !self.embedded {
            unsafe { let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); }
        }
    }
}

static TASKBAR_CREATED: std::sync::OnceLock<u32> = std::sync::OnceLock::new();

extern "system" fn strip_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if Some(&msg) == TASKBAR_CREATED.get() {
            // Explorer yeniden başladı: görev çubuğuna yeniden gömül.
            STRIP.with(|s| s.borrow_mut().take());
            start();
            return LRESULT(0);
        }
        match msg {
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
            WM_MOUSEMOVE => {
                // Şeridin üstüne gelindi: arayüz kapalıysa arkada hazırla (olay güdümlü, bir kez; ayrılınca yeniden).
                if !HOVER.with(|h| h.replace(true)) {
                    use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
                    let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                    let _ = TrackMouseEvent(&mut tme);
                    if !crate::ipc::ui_alive() { crate::ipc::spawn_ui("warm"); }
                }
                return LRESULT(0);
            }
            0x02A3 => { HOVER.with(|h| h.set(false)); return LRESULT(0); } // WM_MOUSELEAVE
            WM_LBUTTONUP => {
                let x = (lp.0 & 0xffff) as i16 as i32;
                let on_box = STRIP.with(|s| s.borrow().as_ref().map(|st| x >= st.w - st.box_w).unwrap_or(false));
                if on_box {
                    cat::toggle_box();
                    refresh();
                } else {
                    crate::ipc::spawn_ui("popover");
                }
                return LRESULT(0);
            }
            WM_RBUTTONUP => { crate::tray::show_menu(); return LRESULT(0); }
            WM_DPICHANGED | WM_DISPLAYCHANGE | WM_SETTINGCHANGE => { refresh(); }
            _ => {}
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

/// Şeridi oluşturur ve görev çubuğuna gömer. Gömülemezse en üstte duran pencere olarak kalır.
pub fn start() {
    let _ = TASKBAR_CREATED.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });
    win::register_class(w!("MeowlyStrip"), Some(strip_proc));
    let hwnd = win::create_layered(w!("MeowlyStrip"), false, None);
    let tb = taskbar();
    let mut embedded = false;
    if !tb.is_invalid() {
        unsafe {
            // Görev çubuğunun çocuğu yap: onunla birlikte gizlenir/gösterilir, hep üstünde kalır.
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            SetWindowLongW(hwnd, GWL_STYLE, ((style & !WS_POPUP.0) | WS_CHILD.0) as i32);
            embedded = SetParent(hwnd, Some(tb)).is_ok();
            if !embedded { SetWindowLongW(hwnd, GWL_STYLE, style as i32); }
        }
    }
    let mut st = Strip { hwnd, surf: win::Surface::new(), box_w: 0, w: 0, h: 0, embedded };
    st.refresh();
    unsafe { let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE); }
    // Görev çubuğunun boyutu/konumu değişince (tepsiye simge eklenince) yeniden yerleş.
    unsafe {
        use windows::Win32::UI::Accessibility::SetWinEventHook;
        let mut pid = 0u32;
        GetWindowThreadProcessId(tb, Some(&mut pid));
        SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, None, Some(location_changed), pid, 0, WINEVENT_OUTOFCONTEXT);
    }
    STRIP.with(|s| *s.borrow_mut() = Some(st));
}

extern "system" fn location_changed(_: windows::Win32::UI::Accessibility::HWINEVENTHOOK, _: u32, hwnd: HWND, id_object: i32, _: i32, _: u32, _: u32) {
    if id_object != OBJID_WINDOW.0 { return; }
    let cls = unsafe {
        let mut b = [0u16; 32];
        let n = GetClassNameW(hwnd, &mut b);
        String::from_utf16_lossy(&b[..n as usize])
    };
    if cls == "TrayNotifyWnd" || cls == "Shell_TrayWnd" { reposition(); }
}

/// Yalnızca yeri değiştiyse taşı (yeniden çizmeden).
fn reposition() {
    STRIP.with(|s| {
        let Ok(mut g) = s.try_borrow_mut() else { return };
        let Some(st) = g.as_mut() else { return };
        st.refresh();
    });
}

/// Arayüz sürecinden: açılır pencere açıldı/kapandı.
pub fn set_popover_open(open: bool) {
    POPOVER_OPEN.with(|p| p.set(open));
    if !open { popover_closed(); }
}

/// Veri ya da ayar değişince yeniden çizer. Açılır pencere açıkken çizilmez; kapanınca bekleyen çizim yapılır.
pub fn refresh() {
    if POPOVER_OPEN.with(|p| p.get()) { PENDING.with(|p| p.set(true)); return; }
    STRIP.with(|s| { if let Ok(mut g) = s.try_borrow_mut() { if let Some(st) = g.as_mut() { st.refresh(); } } });
}

thread_local! {
    static HOVER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static PENDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static POPOVER_OPEN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn popover_closed() {
    if PENDING.with(|p| p.replace(false)) { refresh(); }
}

/// Açılır pencerenin konumlanacağı şerit dikdörtgeni (ekran koordinatı). Arayüz sürecinden de çalışır.
pub fn find_rect() -> Option<RECT> {
    unsafe {
        let tb = taskbar();
        let h = FindWindowExW(Some(tb), None, w!("MeowlyStrip"), None).or_else(|_| FindWindowW(w!("MeowlyStrip"), None)).ok()?;
        let mut r = RECT::default();
        GetWindowRect(h, &mut r).ok()?;
        Some(r)
    }
}

/// Geliştirme: şeridi PNG'ye çizer.
pub fn snapshot(path: &std::path::Path, scale: f64) {
    render(scale).0.save_png(path);
}

/// Ayarlar → Görev çubuğu: tek göstergenin önizlemesi (%57), şeritteki gibi beyaz/siyah kutuda (Mac: StyleChoice).
pub fn glyph_preview_png(style: &str, s: f64) -> Vec<u8> {
    let dark = settings::get().dark;
    let fill: Rgb = if dark { (15, 15, 17) } else { (255, 255, 255) };
    let number: Rgb = if dark { (255, 255, 255) } else { (22, 12, 12) };
    let left = Some(0.57);
    let pct_w = text::width("57%", 13.0, Weight::Bold) as f64;
    let gw = match style {
        "bar" => 26.0, "barPercent" => 25.0 + pct_w, "ring" => 14.0, "dot" => 8.0,
        "battery" => 29.0, "segments" => 26.0, _ => pct_w,
    };
    let (w, h) = (((gw + 20.0) * s).round() as usize, (22.0 * s).round() as usize);
    let mut out = Img::new(w, h);
    rounded(&mut out, 0.0, 0.0, w as f64, h as f64, 3.0 * s, fill);
    draw_glyph(&mut out, left, style, 10.0 * s, h as f64 / 2.0, s, number);
    out.png_bytes()
}

/// Ayarlar → Görev çubuğu önizlemesi: şeridin şu anki resmi (ana süreçte, güncel veriyle).
pub fn preview_png() -> Vec<u8> { render(2.0).0.png_bytes() }