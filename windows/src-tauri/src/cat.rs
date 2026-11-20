//! Masaüstündeki kedi. Mac: Sources/UllageCat/Cat/CatPet.swift — süreler, sabitler ve kurallar birebir.
//! Pil için: sürekli döngü yok. Yalnızca görüntü gerçekten değişeceği zaman tek seferlik zamanlayıcı kurulur;
//! görüntü değişmediyse pencereye hiç basılmaz. Ekran kapalı/kilitli ya da enerji tasarrufundayken her şey durur.

use crate::img::{rgba, Img};
use crate::text::{self, Weight};
use crate::{l10n, sense, settings, store, win};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::WindowsAndMessaging::*;

// MARK: Kareler

fn sprite_bytes(name: &str) -> Option<&'static [u8]> {
    macro_rules! s { ($($n:literal),*) => { match name { $($n => Some(include_bytes!(concat!("../../../Resources/Cat/", $n, ".png")) as &[u8]),)* _ => None } } }
    s!("idle_0", "idle_1", "idle_2", "groomA", "groomB", "groomC", "groomD", "type_0", "type_1c", "yawn", "sleep",
       "content", "grumpy", "tired", "dangle", "sit", "blink", "laptop")
}

thread_local! {
    static SOURCES: RefCell<HashMap<&'static str, Rc<Img>>> = RefCell::new(HashMap::new());
    static CAT: RefCell<Option<Cat>> = const { RefCell::new(None) };
}

fn source(name: &'static str) -> Rc<Img> {
    SOURCES.with(|m| {
        m.borrow_mut().entry(name).or_insert_with(|| Rc::new(Img::decode_png(sprite_bytes(name).expect(name)))).clone()
    })
}

// MARK: Animasyonlar (CatAnim)

#[derive(Clone)]
pub struct Anim {
    frames: Vec<(&'static str, f64)>,
    looping: bool,
    breathe: f64,
    tilt: &'static [(&'static str, f64)],
}

impl Anim {
    fn total(&self) -> f64 { self.frames.iter().map(|f| f.1).sum() }

    /// Oturur, göz kırpar, yana bakar; sonra patisini yalar ve oturuşa döner.
    fn idle() -> Anim {
        Anim {
            frames: vec![
                ("idle_0", 2.6), ("idle_1", 0.16), ("idle_0", 1.2), ("idle_2", 1.4), ("idle_0", 0.8),
                ("groomA", 0.22), ("groomB", 0.25), ("groomC", 0.32), ("groomD", 0.32), ("groomC", 0.32), ("groomD", 0.32),
                ("groomB", 0.25), ("groomA", 0.22), ("idle_0", 2.2), ("idle_1", 0.16), ("idle_0", 1.6),
            ],
            looping: true,
            breathe: 2.2,
            tilt: &[("groomB", -1.5), ("groomC", -3.5), ("groomD", -2.5)],
        }
    }
    /// Yalnızca oturup göz kırpar, etrafına bakar.
    fn calm() -> Anim {
        Anim { frames: vec![("idle_0", 2.4), ("idle_1", 0.16), ("idle_0", 1.8), ("idle_2", 1.6), ("idle_0", 2.0)], looping: true, breathe: 2.2, tilt: &[] }
    }
    /// Yalnızca ön patiler klavyeye basar.
    fn laptop() -> Anim {
        let mut f = Vec::new();
        for _ in 0..8 { f.push(("type_0", 0.2)); f.push(("type_1c", 0.2)); }
        f.push(("type_0", 1.6));
        for _ in 0..5 { f.push(("type_0", 0.2)); f.push(("type_1c", 0.2)); }
        Anim { frames: f, looping: true, breathe: 2.4, tilt: &[] }
    }
    fn sleep() -> Anim { Anim::still("sleep", 3.2) }
    fn dangle() -> Anim { Anim { frames: vec![("dangle", 1.0)], looping: true, breathe: 0.0, tilt: &[] } }
    fn content() -> Anim { Anim::still("content", 1.4) }
    fn still(n: &'static str, breathe: f64) -> Anim { Anim { frames: vec![(n, 1.0)], looping: true, breathe, tilt: &[] } }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Base { Sit, Laptop, Grumpy, Tired, Sleep }

// MARK: Zamanlama yardımcıları

/// CAMediaTimingFunction kübik Bézier (0,0 → c1 → c2 → 1,1).
fn bezier(x: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let (cx, bx) = (3.0 * x1, 3.0 * (x2 - x1) - 3.0 * x1);
    let ax = 1.0 - cx - bx;
    let (cy, by) = (3.0 * y1, 3.0 * (y2 - y1) - 3.0 * y1);
    let ay = 1.0 - cy - by;
    let mut t = x;
    for _ in 0..8 {
        let fx = ((ax * t + bx) * t + cx) * t - x;
        let d = (3.0 * ax * t + 2.0 * bx) * t + cx;
        if d.abs() < 1e-6 { break; }
        t = (t - fx / d).clamp(0.0, 1.0);
    }
    ((ay * t + by) * t + cy) * t
}
fn ease_in_out(x: f64) -> f64 { bezier(x.clamp(0.0, 1.0), 0.42, 0.0, 0.58, 1.0) }
fn ease_out(x: f64) -> f64 { bezier(x.clamp(0.0, 1.0), 0.0, 0.0, 0.58, 1.0) }

/// Nefes: 1 → 1,022, ileri-geri (autoreverses), easeInEaseOut.
fn breathe_at(t: f64, period: f64) -> f64 {
    if period <= 0.0 { return 1.0; }
    let half = period / 2.0;
    let u = (t % period) / half;
    let u = if u > 1.0 { 2.0 - u } else { u };
    1.0 + 0.022 * ease_in_out(u)
}

/// Yere konma yayı (CASpringAnimation damping 14, stiffness 260, kütle 1): 0,94 → 1.
/// Oturan kedinin karesini çizer (pencere ve galeri aynı kodu kullanır).
/// Bölünmüş çizimde gövde alttan `body_dh` yüksekliğinde, baş `head_bottom`'a oturur ve gövdeye 3 piksel biner;
/// `head_scale` yere konarken başın da gövdeyle aynı oranda basılmasını sağlar (Mac: basma bütün kediye uygulanır).
fn compose(img: &Img, src_h: usize, split: bool, w: usize, h: usize, body_rows: usize, body_dh: usize, head_bottom: i32, head_scale: f64) -> Img {
    let mut out = Img::new(w, h);
    let none = |_: usize| 0;
    let x0 = ((w as i32 - img.w as i32) as f64 / 2.0).round() as i32;
    if split {
        let lap = ((3.0 * img.h as f64 / src_h as f64).round() as usize).max(1);
        let head_rows = (img.h - body_rows + lap).min(img.h);
        let head_dh = ((head_rows as f64 * head_scale).round() as usize).max(1);
        out.blit_rows(img, img.h - body_rows, body_rows, x0, h as i32 - body_dh as i32, body_dh, 1.0, &none);
        out.blit_rows(img, 0, head_rows, x0, head_bottom - head_dh as i32, head_dh, 1.0, &none);
    } else {
        out.blit_rows(img, 0, img.h, x0, h as i32 - body_dh as i32, body_dh, 1.0, &none);
    }
    out
}

fn land_at(t: f64) -> f64 {
    let (k, c): (f64, f64) = (260.0, 14.0);
    let z = c / 2.0;
    let wd = (k - z * z).sqrt();
    1.0 - 0.06 * (-z * t).exp() * ((wd * t).cos() + z / wd * (wd * t).sin())
}
const LAND_DUR: f64 = 1.0;

fn catmull(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    0.5 * (2.0 * p1 + (-p0 + p2) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t)
}

// MARK: Efektler (z, kalp)

enum FxKind { Z(usize), Heart }
struct Fx { kind: FxKind, start: Instant, x: f64, y: f64 }

const HEART: [&str; 6] = [".##.##.", "#######", "#######", ".#####.", "..###..", "...#..."];
const ZED: [&str; 5] = ["#####", "...#.", "..#..", ".#...", "#####"];

// MARK: Balon

struct Bubble { img: Img, x: i32, y: i32, alpha: f64, target: f64 }

const CAT_W: f64 = 145.0;
const CAT_H: f64 = 125.0;
const DRAG_W: f64 = 215.0;
const DRAG_H: f64 = 172.0;
const DANGLE_SCALE: f64 = 1.45;

const T_ANIM: usize = 1;
const T_IDLE: usize = 2;
const T_UPDATE: usize = 3;
const T_ONESHOT: usize = 4;
const T_BUBBLE_HIDE: usize = 5;
const T_DRAG: usize = 6;
const T_FADE: usize = 7;
const T_DEMO: usize = 8;
const T_DAILY: usize = 10;
const T_HOVER: usize = 11;

const INK: (u8, u8, u8) = (22, 12, 12);

#[derive(Clone, Copy, PartialEq)]
enum Topic { Work, Rest, Daily, Record, Chat }

pub struct Cat {
    hwnd: HWND,
    bubble_hwnd: HWND,
    surf: win::Surface,
    bsurf: win::Surface,
    scale: f64,
    scaled: HashMap<(&'static str, u32), Rc<Img>>,

    base: Base,
    anim: Anim,
    play_start: Instant,
    sprite_top: f64,
    food: f64,
    water: f64,
    prev_food: f64,
    first_update: bool,
    busy_until: Instant,
    paused: bool,
    screen_off: bool,
    locked: bool,
    saver: bool,
    hidden: bool,
    fullscreen: bool,
    pos: (i32, i32),

    dragging: bool,
    down_at: Option<(i32, i32)>,
    dragged: bool,
    mid_x: f64, mid_v: f64, tip_x: f64, tip_v: f64, vx_smooth: f64,
    cursor: (f64, f64),
    prev_cursor: (f64, f64),
    last_frame: Option<Instant>,
    land_start: Option<Instant>,

    last_hover: Option<(i32, i32)>,
    stroke: f64,
    tracking: bool,
    last_pet: Instant,

    fx: Vec<Fx>,
    bubble: Option<Bubble>,
    last_say: Option<Instant>,

    last_project: Option<String>,
    work_start: Option<f64>,
    break_mark: i64,
    ai_was_working: bool,
    report_click_until: Option<Instant>,
    demo: Option<usize>,
    last_sig: u64,
}

fn env(k: &str) -> Option<String> { std::env::var(k).ok() }

/// Kedi penceresi: veri iş parçacığı yeni veri olunca buraya WM_APP+1 gönderir.
static CAT_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
const WM_DATA: u32 = WM_APP + 1;

/// Veri iş parçacığından çağrılır: kediye "yeni veri var" der (ana iş parçacığında update çalışır).
pub fn notify() {
    let h = CAT_HWND.load(std::sync::atomic::Ordering::Relaxed);
    if h != 0 { unsafe { let _ = PostMessageW(Some(HWND(h as *mut _)), WM_DATA, WPARAM(0), LPARAM(0)); } }
}

/// Son Claude/Codex etkinliği (veritabanındaki en yeni olay, unix sn).
fn last_ai() -> f64 { store::latest_event("claude").max(store::latest_event("codex")) }

/// Şu anki çalışma oturumunun başlangıcı (Günlük ile aynı oturum hesabı).
fn session_start() -> Option<f64> {
    let now = sense::now_secs();
    store::sessions(now - 12.0 * 3600.0, 1e11).into_iter()
        .filter(|s| now - s.end <= store::SESSION_GAP)
        .max_by(|a, b| a.end.partial_cmp(&b.end).unwrap())
        .map(|s| s.start)
}

pub fn format_tokens(n: i64) -> String {
    match n {
        n if n >= 1_000_000_000 => format!("{:.1}B", n as f64 / 1e9),
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1e6),
        n if n >= 1_000 => format!("{}K", n / 1000),
        n => n.to_string(),
    }
}

/// Geliştirme günlüğü (MEOWLY_LOG=1): %APPDATA%\Meowly\dev.log. Yalnızca durum sayıları; içerik yok.
pub fn dev_log(s: &str) {
    if std::env::var_os("MEOWLY_LOG").is_none() { return; }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(settings::data_dir().join("dev.log")) {
        let _ = writeln!(f, "{:.1} {s}", sense::now_secs());
    }
}

impl Cat {
    fn new(hwnd: HWND, bubble_hwnd: HWND) -> Cat {
        let past = Instant::now() - Duration::from_secs(3600);
        Cat {
            hwnd, bubble_hwnd,
            surf: win::Surface::new(), bsurf: win::Surface::new(),
            scale: win::dpi_scale(hwnd),
            scaled: HashMap::new(),
            base: Base::Sit, anim: Anim::idle(), play_start: Instant::now(), sprite_top: 108.0,
            food: 1.0, water: 1.0, prev_food: 1.0, first_update: true,
            busy_until: past, paused: false, screen_off: false, locked: false, saver: false, hidden: false, fullscreen: false,
            pos: (0, 0),
            dragging: false, down_at: None, dragged: false,
            mid_x: 0.0, mid_v: 0.0, tip_x: 0.0, tip_v: 0.0, vx_smooth: 0.0,
            cursor: (0.0, 0.0), prev_cursor: (0.0, 0.0), last_frame: None, land_start: None,
            last_hover: None, stroke: 0.0, tracking: false, last_pet: past,
            fx: Vec::new(), bubble: None, last_say: None,
            last_project: None, work_start: None, break_mark: 0, ai_was_working: false, report_click_until: None,
            demo: None, last_sig: 0,
        }
    }

    fn px(&self, pt: f64) -> i32 { (pt * self.scale).round() as i32 }

    fn sprite(&mut self, name: &'static str, factor: f64) -> Rc<Img> {
        let key = (name, (factor * 10000.0).round() as u32);
        if let Some(i) = self.scaled.get(&key) { return i.clone(); }
        let i = Rc::new(source(name).resample(factor));
        self.scaled.insert(key, i.clone());
        i
    }

    fn timer(&self, id: usize, secs: f64) {
        unsafe { SetTimer(Some(self.hwnd), id, (secs * 1000.0).round().max(1.0) as u32, None); }
    }
    fn kill(&self, id: usize) { unsafe { let _ = KillTimer(Some(self.hwnd), id); } }

    // MARK: Konum

    fn size_px(&self) -> (i32, i32) {
        if self.dragging { (self.px(DRAG_W), self.px(DRAG_H)) } else { (self.px(CAT_W), self.px(CAT_H)) }
    }

    fn default_home(&self) -> (i32, i32) {
        let f = win::primary_work_area();
        (f.right - self.px(CAT_W) - self.px(40.0), f.bottom - self.px(4.0) - self.px(CAT_H))
    }

    /// Pencereyi görünür ekran alanının içinde tutar (yanlarda 30 pt taşabilir).
    fn clamped(&self, o: (i32, i32)) -> (i32, i32) {
        let (w, h) = (self.px(CAT_W), self.px(CAT_H));
        let f = win::work_area_at(o.0 + w / 2, o.1 + h / 2);
        let m = self.px(30.0);
        ((o.0).clamp(f.left - m, (f.right - w + m).max(f.left - m)), (o.1).clamp(f.top, (f.bottom - h).max(f.top)))
    }

    // MARK: Oynatma

    /// Kedi her zaman ekrandadır; yalnızca kullanıcı onu kutuya (evine) gönderince gizlenir.
    fn visible(&self) -> bool { !self.hidden }
    fn halted(&self) -> bool { self.paused || !self.visible() }

    fn play(&mut self, a: Anim) {
        if !self.dragging {
            let mx = a.frames.iter().map(|f| source(f.0).h).max().unwrap_or(216);
            self.sprite_top = mx as f64 / 2.0;
        }
        self.anim = a;
        self.play_start = Instant::now();
        self.last_sig = 0;
        self.render();
    }

    fn play_base(&mut self) {
        if self.dragging { return; }
        let a = match self.base {
            Base::Sit => Anim::idle(),
            Base::Laptop => Anim::laptop(),
            Base::Grumpy => Anim::still("grumpy", 2.4),
            Base::Tired => Anim::still("tired", 3.0),
            Base::Sleep => Anim::sleep(),
        };
        self.play(a);
    }

    /// Tek seferlik hareket; bitince temel animasyona döner.
    fn one_shot(&mut self, a: Anim, secs: f64) {
        if self.dragging { return; }
        self.busy_until = Instant::now() + Duration::from_secs_f64(secs);
        self.play(a);
        self.timer(T_ONESHOT, secs);
    }

    fn frame_at(&self, t: f64) -> (usize, f64) {
        let total = self.anim.total();
        let tt = if self.anim.looping { t % total } else { t.min(total - 1e-6) };
        let mut acc = 0.0;
        for (i, f) in self.anim.frames.iter().enumerate() {
            if tt < acc + f.1 { return (i, acc + f.1 - tt); }
            acc += f.1;
        }
        (self.anim.frames.len() - 1, f64::INFINITY)
    }

    fn tilt_at(&self, t: f64) -> f64 {
        if self.anim.tilt.is_empty() { return 0.0; }
        let total = self.anim.total();
        let u = (t % total) / total;
        let tilt = |n: &str| self.anim.tilt.iter().find(|x| x.0 == n).map(|x| x.1).unwrap_or(0.0);
        let mut times = Vec::new();
        let mut vals = Vec::new();
        let mut acc = 0.0;
        for f in &self.anim.frames { times.push(acc / total); vals.push(tilt(f.0)); acc += f.1; }
        times.push(1.0);
        vals.push(vals[0]);
        let n = vals.len();
        let k = (0..n - 1).find(|&i| u < times[i + 1]).unwrap_or(n - 2);
        let p = ((u - times[k]) / (times[k + 1] - times[k])).clamp(0.0, 1.0);
        let g = |i: isize| vals[i.clamp(0, n as isize - 1) as usize];
        catmull(g(k as isize - 1), g(k as isize), g(k as isize + 1), g(k as isize + 2), p)
    }

    // MARK: Çizim

    /// Kediyi çizer; görüntü değişmediyse pencereye basmaz. Sonraki değişim için zamanlayıcı kurar.
    fn render(&mut self) {
        if !self.visible() { return; }
        if self.dragging { self.render_drag(); return; }
        let now = Instant::now();
        let s = self.scale;
        let (w, h) = (self.px(CAT_W), self.px(CAT_H));
        let t = if self.paused { 0.0 } else { now.duration_since(self.play_start).as_secs_f64() };
        let (fi, left) = self.frame_at(t);
        let name = self.anim.frames[fi].0;
        let img = self.sprite(name, s / 2.0);
        let src_h = source(name).h;
        let live = !self.paused;
        let breathe = if live { breathe_at(t, self.anim.breathe) } else { 1.0 };
        let land = self.land_start.map(|l| now.duration_since(l).as_secs_f64()).filter(|&x| x < LAND_DUR && live);
        let body_scale = land.map(land_at).unwrap_or(breathe);
        let split = !self.anim.tilt.is_empty();
        let tilt = if live { self.tilt_at(t) } else { 0.0 };
        self.fx.retain(|f| now.duration_since(f.start).as_secs_f64() < match f.kind { FxKind::Z(_) => 2.6, FxKind::Heart => 1.15 });

        // Görüntü imzası: değişmediyse çizme.
        let body_rows_src = if split { (src_h as f64 * (1.0 - 0.52)).round() as usize } else { src_h };
        let body_rows = ((body_rows_src as f64 * img.h as f64 / src_h as f64).round() as usize).min(img.h);
        let body_dh = (body_rows as f64 * body_scale).round() as usize;
        // Baş gövdenin üstüne oturur: yere konarken (Mac'te basma bütün kediye uygulanır) baş da gövdeyle birlikte iner.
        let head_bottom_pt = (body_rows_src as f64 / 2.0) * body_scale - 1.5 + tilt;
        let head_bottom = h - self.px(head_bottom_pt);
        let mut sig: u64 = (fi as u64) << 48 | (body_dh as u64) << 32 | ((head_bottom as u32 as u64) << 8) | split as u64;
        for f in &self.fx { sig = sig.wrapping_mul(31).wrapping_add(self.fx_sig(f, now)); }
        if let Some(b) = &self.bubble { sig = sig.wrapping_mul(7).wrapping_add(b.x as u64 ^ (b.y as u64) << 20); }
        sig = sig.wrapping_add((self.pos.0 as u64) << 40 ^ (self.pos.1 as u64) << 16);

        if sig != self.last_sig {
            self.last_sig = sig;
            let head_scale = land.map(land_at).unwrap_or(1.0);
            let mut out = compose(&img, src_h, split, w as usize, h as usize, body_rows, body_dh, head_bottom, head_scale);
            for f in &self.fx { self.draw_fx(&mut out, f, now); }
            self.surf.present(self.hwnd, &out, self.pos.0, self.pos.1, 255);
        }

        // Sonraki değişim.
        if !live { self.kill(T_ANIM); return; }
        let mut next = if left.is_finite() { left } else { 3600.0 };
        if self.anim.breathe > 0.0 || split {
            // Nefes/baş eğimi: bir sonraki gerçek piksel değişimini önceden bul, yalnızca o an uyan.
            let parts = |tt: f64| {
                let b = breathe_at(tt, self.anim.breathe);
                let dh = (body_rows as f64 * b).round() as i64;
                let hb = if split { self.px((body_rows_src as f64 / 2.0) * b - 1.5 + self.tilt_at(tt)) as i64 } else { 0 };
                (dh, hb)
            };
            let cur = parts(t);
            let mut dt = 0.01;
            while dt < next.min(3.0) {
                if parts(t + dt) != cur { break; }
                dt += 0.01;
            }
            next = next.min(dt);
        }
        if land.is_some() || !self.fx.is_empty() { next = next.min(1.0 / 30.0); }
        self.timer(T_ANIM, next.max(0.01));
    }

    fn fx_sig(&self, f: &Fx, now: Instant) -> u64 {
        let t = now.duration_since(f.start).as_secs_f64();
        (t * 30.0) as u64 + 1
    }

    fn draw_fx(&self, out: &mut Img, f: &Fx, now: Instant) {
        let t = now.duration_since(f.start).as_secs_f64();
        let h = out.h as f64;
        let s = self.scale;
        match f.kind {
            FxKind::Z(i) => {
                let tt = t - i as f64 * 0.6;
                if !(0.0..=1.8).contains(&tt) { return; }
                let p = tt / 1.8;
                let op = if p < 0.2 { p / 0.2 } else if p < 0.7 { 1.0 } else { 1.0 - (p - 0.7) / 0.3 };
                let side = if i == 0 { 10.0 } else { 7.0 };
                let (cx, cy) = (f.x, f.y + 18.0 * p);
                draw_cells(out, &ZED, cx - side / 2.0, cy + side / 2.0, side / 5.0, s, h, (140, 153, 191), op as f32);
            }
            FxKind::Heart => {
                let p = ease_out(t / 1.1);
                if t > 1.1 { return; }
                let (cx, cy) = (f.x, f.y + 22.0 * p);
                draw_cells(out, &HEART, cx - 7.0, cy + 6.0, 2.0, s, h, (245, 115, 148), (1.0 - p) as f32);
            }
        }
    }

    /// Tutulurken: kare 2 piksel satırlık şeritlere bölünür, her şerit enseden uzaklığına göre yana kayar.
    fn render_drag(&mut self) {
        let s = self.scale;
        let (w, h) = (self.px(DRAG_W), self.px(DRAG_H));
        let src = source("dangle");
        let img = self.sprite("dangle", s / DANGLE_SCALE);
        let count = (src.h / 2).max(8);
        let (m, t) = (self.mid_x, self.tip_x);
        let shifts: Vec<i32> = (0..img.h)
            .map(|r| {
                let src_row = ((r as f64 + 0.5) * src.h as f64 / img.h as f64) as usize;
                let strip = (src_row * count / src.h).min(count - 1);
                let sc = (strip as f64 + 0.5) / count as f64;
                let off = m * sc.powf(1.6) + (t - m) * sc.powf(3.2);
                // Kayma kaynak pikseline yuvarlanır (Mac ile aynı), sonra ekrana.
                (((off * DANGLE_SCALE).round() / DANGLE_SCALE) * s).round() as i32
            })
            .collect();
        let mut sig: u64 = 0xD;
        for v in &shifts { sig = sig.wrapping_mul(131).wrapping_add(*v as u64); }
        sig = sig.wrapping_add((self.pos.0 as u64) << 40 ^ (self.pos.1 as u64) << 16);
        if sig == self.last_sig { return; }
        self.last_sig = sig;
        let mut out = Img::new(w as usize, h as usize);
        let x0 = ((w - img.w as i32) as f64 / 2.0).round() as i32;
        out.blit_rows(&img, 0, img.h, x0, 0, img.h, 1.0, &|r| shifts[r]);
        self.surf.present(self.hwnd, &out, self.pos.0, self.pos.1, 255);
    }

    // MARK: Duruş

    fn ai_working(&self) -> bool {
        let ai = env("MEOWLY_AI_IDLE").and_then(|v| v.parse().ok()).unwrap_or_else(|| sense::now_secs() - last_ai());
        ai < 120.0 && sense::user_idle() < 10.0 * 60.0
    }

    /// Temel duruş (öncelik sırasıyla; Mac 1.0.4):
    /// 1. yapay zekâ çalışıyor → laptopta yazar (limit tamamen bittiyse yorgun)
    /// 2. kullanıcı 3 dk klavye/fareye dokunmadı ve video/görüşme yok → uyur
    /// 3. kullanıcı herhangi bir uygulamada yazıyor (son 90 sn'de tuş) → laptopta yazar
    /// 4. limit bitti → yorgun; limit %15 altı → huysuz
    /// 5. aksi halde → oturur, yalanır, etrafa bakar
    fn compute_base(&self) -> Base {
        if let Some(b) = env("ULLAGECAT_BASE") {
            match b.as_str() { "sit" => return Base::Sit, "laptop" => return Base::Laptop, "sleep" => return Base::Sleep,
                               "tired" => return Base::Tired, "grumpy" => return Base::Grumpy, _ => {} }
        }
        let empty = self.food <= 0.001 || self.water <= 0.001;
        if self.ai_working() { return if empty { Base::Tired } else { Base::Laptop }; }
        if sense::user_idle() > 3.0 * 60.0 && !sense::media_active() { return Base::Sleep; }
        if sense::key_idle() < 90.0 { return Base::Laptop; }
        if empty { return Base::Tired; }
        if self.food < 0.15 || self.water < 0.15 { return Base::Grumpy; }
        Base::Sit
    }

    /// Duruşu günceller; uykuya dalarken ve uyanırken önce esner. Değiştiyse true.
    fn apply_base(&mut self) -> bool {
        let nb = self.compute_base();
        if nb == self.base { return false; }
        let was_sleeping = self.base == Base::Sleep;
        self.base = nb;
        if self.dragging { return true; }
        if was_sleeping != (nb == Base::Sleep) { self.one_shot(Anim::still("yawn", 0.0), 1.6); }
        else if Instant::now() >= self.busy_until { self.play_base(); }
        true
    }

    /// Yenileme (yaklaşık 30 sn; Mac'te kullanım verisi gelince).
    fn update(&mut self) {
        // Claude limiti varsa o, yoksa Codex (Mac: usage[.claude] ?? usage[.codex]).
        let snap = crate::data::snapshot();
        let u = snap.claude.as_ref().filter(|u| u.confidence != crate::data::Confidence::None)
            .or(snap.codex.as_ref()).and_then(|u| u.limits.clone());
        let env_food = env("MEOWLY_FOOD").and_then(|v| v.parse::<f64>().ok());
        self.food = env_food.or(u.as_ref().and_then(|l| l.five_left)).unwrap_or(1.0);
        self.water = env_food.or(u.as_ref().and_then(|l| l.week_left)).unwrap_or(1.0);
        let refilled = !self.first_update && self.prev_food <= 0.2 && self.food >= 0.9;
        self.prev_food = self.food;
        self.apply_base();
        if refilled { self.one_shot(Anim::content(), 2.5); self.pop_heart(); }
        self.sync_ai_work();
        self.work_tick();
        dev_log(&format!(
            "update base={:?} paused={} (ekran_kapali={} kilit={} tasarruf={}) gizli={} tamekran={} kullanici_bosta={:.0}s tus_bosta={:.0}s video={} ai_once={:.0}s",
            self.base, self.paused, self.screen_off, self.locked, self.saver, self.hidden, self.fullscreen,
            sense::user_idle(), sense::key_idle(), sense::media_active(), sense::now_secs() - last_ai()
        ));
        if self.first_update { self.timer(T_DAILY, 1.5); }
        if u.is_some() { self.first_update = false; }
    }

    // MARK: Boşta hareketleri

    fn schedule_idle(&self) {
        self.kill(T_IDLE);
        if self.halted() { return; }
        self.timer(T_IDLE, 8.0 + fastrand::f64() * 8.0);
    }

    fn idle_tick(&mut self) {
        self.check_fullscreen();
        if self.dragging || self.demo.is_some() { self.schedule_idle(); return; }
        // Kullanıcı bilgisayardan kalkınca/dönünce yenilemeyi beklemeden uyur/uyanır.
        let changed = self.apply_base();
        self.sync_ai_work();
        if changed || Instant::now() < self.busy_until { self.schedule_idle(); return; }
        let r = fastrand::f64();
        match self.base {
            Base::Sit => {
                // Kendiliğinden girilen dikey "göbek" pozu kaldırıldı (Mac 1.0.5 sonrası): yalnızca esner ve etrafına bakar.
                if r < 0.25 { self.one_shot(Anim::still("yawn", 0.0), 1.6) }
                else if r < 0.45 { self.one_shot(Anim::calm(), 8.0) }
            }
            Base::Laptop => {
                if r < 0.35 { self.one_shot(Anim::idle(), 10.0 + fastrand::f64() * 6.0) }
                else if r < 0.50 { self.one_shot(Anim::still("yawn", 0.0), 1.5) }
                else if r < 0.58 && self.ai_was_working { let t = self.note_text(); self.say_topic(Topic::Work, &t, 5.0, false); }
            }
            Base::Sleep => self.pop_z(),
            Base::Tired | Base::Grumpy => {
                if r < 0.3 { self.one_shot(Anim::still("yawn", 0.0), 1.6) }
                else if r < 0.5 { self.one_shot(Anim::calm(), 6.0) }
            }
        }
        self.schedule_idle();
    }

    fn pop_z(&mut self) {
        let (cx, y) = (CAT_W / 2.0, self.sprite_top + 2.0);
        let now = Instant::now();
        self.fx.push(Fx { kind: FxKind::Z(0), start: now, x: cx + 22.0, y });
        self.fx.push(Fx { kind: FxKind::Z(1), start: now, x: cx + 30.0, y });
        self.last_sig = 0;
        self.render();
    }

    fn pop_heart(&mut self) {
        let top = (CAT_H - 8.0).min(self.sprite_top);
        self.fx.push(Fx { kind: FxKind::Heart, start: Instant::now(), x: CAT_W / 2.0 + (fastrand::f64() * 20.0 - 10.0), y: top - 10.0 });
        self.last_sig = 0;
        self.render();
    }

    /// Sevilince: uyuyorsa esner; değilse gözlerini kısar ve kalp yükselir.
    fn pet(&mut self) {
        if self.dragging || self.last_pet.elapsed().as_secs_f64() <= 2.5 { return; }
        self.last_pet = Instant::now();
        if self.base == Base::Sleep { self.one_shot(Anim::still("yawn", 0.0), 1.4); return; }
        self.one_shot(Anim::content(), 2.2);
        self.pop_heart();
        if fastrand::f64() < 0.35 { self.say_topic(Topic::Chat, &l10n::t("Mırr…"), 2.0, false); }
    }

    // MARK: Çalışma balonları (yalnızca yapay zekâ oturumu)

    fn duration(s: f64) -> String {
        let m = ((s / 60.0) as i64).max(1);
        let h = m / 60;
        if h == 0 { return l10n::f("%lld dk", &[m.to_string()]); }
        if m % 60 == 0 { l10n::f("%lld sa", &[h.to_string()]) } else { l10n::f("%lld sa %lld dk", &[h.to_string(), (m % 60).to_string()]) }
    }

    fn note_text(&self) -> String {
        let mut parts = Vec::new();
        if let Some(p) = store::latest_project() { parts.push(p); }
        if let Some(ws) = self.work_start {
            let d = sense::now_secs() - ws;
            if d >= 60.0 { parts.push(Self::duration(d)); }
        }
        let head = l10n::t("✎ Not alıyorum…");
        if parts.is_empty() { head } else { format!("{head}\n{}", parts.join(" · ")) }
    }

    fn sync_ai_work(&mut self) {
        let now = self.ai_working() && self.base != Base::Sleep;
        if now == self.ai_was_working { return; }
        self.ai_was_working = now;
        if now {
            let ws = session_start().unwrap_or_else(sense::now_secs);
            self.work_start = Some(ws);
            self.last_project = store::latest_project();
            self.break_mark = ((sense::now_secs() - ws) / (90.0 * 60.0)) as i64;
            let t = self.note_text();
            self.say_topic(Topic::Work, &t, 5.0, true);
        } else if let Some(ws) = self.work_start.take() {
            let end = store::activity_times(ws).last().copied().unwrap_or_else(sense::now_secs);
            if end - ws >= 5.0 * 60.0 {
                let p = self.last_project.clone().map(|p| p + " · ").unwrap_or_default();
                let t = format!("{}\n{}{}", l10n::t("Oturumu kaydettim"), p, Self::duration(end - ws));
                self.say_topic(Topic::Work, &t, 6.0, true);
            }
        }
    }

    fn work_tick(&mut self) {
        if !self.ai_was_working { return; }
        let Some(ws) = self.work_start else { return };
        if let Some(p) = store::latest_project() {
            if Some(&p) != self.last_project.as_ref() {
                self.last_project = Some(p.clone());
                let t = format!("{}\n{}", l10n::t("✎ Yeni sayfa açtım"), p);
                self.say_topic(Topic::Work, &t, 5.0, true);
                return;
            }
        }
        let mark = ((sense::now_secs() - ws) / (90.0 * 60.0)) as i64;
        if mark > self.break_mark {
            self.break_mark = mark;
            let t = format!("{}{}\n{}", Self::duration(sense::now_secs() - ws), l10n::t(" oldu"), l10n::t("Biraz esneyelim mi?"));
            self.say_topic(Topic::Rest, &t, 7.0, true);
        }
        self.record_note();
    }

    /// Günün token eşikleri: 250K … 100M; her eşik günde bir kez.
    fn record_note(&mut self) {
        let day = store::start_of_day(sense::now_secs());
        let day_key = day as i64;
        let steps: [u64; 9] = [250_000, 500_000, 1_000_000, 2_000_000, 5_000_000, 10_000_000, 20_000_000, 50_000_000, 100_000_000];
        let tokens = store::total_tokens(day).max(0) as u64;
        let reached = steps.iter().rev().find(|&&s| tokens >= s).copied().unwrap_or(0);
        let st = settings::get();
        let prev = if st.cat_record_day == Some(day_key) { st.cat_record } else { 0 };
        if reached <= prev { return; }
        settings::update(|s| { s.cat_record_day = Some(day_key); s.cat_record = reached; });
        let t = format!("{}\n{}", l10n::f("Bugün %@ token!", &[format_tokens(reached as i64)]), l10n::t("Deftere yazdım"));
        self.say_topic(Topic::Record, &t, 5.0, false);
    }

    /// Günün ilk açılışında dünün kısa özeti; 10 dk içinde tıklanırsa dünün raporu açılır.
    fn daily_note(&mut self) {
        let today = store::start_of_day(sense::now_secs());
        let key = today as i64;
        if settings::get().cat_daily_day == Some(key) { return; }
        settings::update(|s| s.cat_daily_day = Some(key));
        let ss = store::sessions(today - 86400.0, today);
        let active: f64 = ss.iter().map(|s| s.duration()).sum();
        let mut projects: Vec<&str> = ss.iter().map(|s| s.project.as_str()).filter(|p| !p.is_empty()).collect();
        projects.sort();
        projects.dedup();
        if active < 5.0 * 60.0 { return; }
        let line = Self::duration(active) + &if projects.is_empty() { String::new() } else { l10n::f(" · %lld proje", &[projects.len().to_string()]) };
        let t = format!("{}: {}\n{}", l10n::t("Dün"), line, l10n::t("Rapor hazır, bana tıkla"));
        self.say_topic(Topic::Daily, &t, 8.0, true);
        if Self::allowed(Topic::Daily) { self.report_click_until = Some(Instant::now() + Duration::from_secs(600)); }
    }

    fn allowed(t: Topic) -> bool {
        let s = settings::get();
        s.cat_bubbles && match t { Topic::Work => s.bubble_work, Topic::Rest => s.bubble_break, Topic::Daily => s.bubble_daily,
                                   Topic::Record => s.bubble_record, Topic::Chat => s.bubble_chat }
    }

    fn say_topic(&mut self, t: Topic, text: &str, secs: f64, force: bool) {
        if !Self::allowed(t) || self.demo.is_some() { return; }
        self.say(text, secs, force);
    }

    // MARK: Konuşma balonu

    /// Başının üstünde kısa bir balon. `force` olmadan en az 20 sn arayla.
    fn say(&mut self, text: &str, secs: f64, force: bool) {
        if self.halted() || self.dragging { return; }
        if !force && self.last_say.map(|l| l.elapsed().as_secs_f64() <= 20.0).unwrap_or(false) { return; }
        self.last_say = Some(Instant::now());
        let s = self.scale;
        let (bw, bh) = bubble_size(text);
        let cat_mid = self.pos.0 as f64 + self.px(CAT_W) as f64 / 2.0;
        let mut x = (cat_mid - bw * s / 2.0).round() as i32;
        let f = win::work_area_at(cat_mid as i32, self.pos.1);
        x = x.clamp(f.left + self.px(4.0), (f.right - (bw * s).round() as i32 - self.px(4.0)).max(f.left));
        let tail_x = (cat_mid - x as f64) / s;
        let img = bubble_image(text, bw, bh, tail_x, s);
        let bottom = self.pos.1 + self.px(CAT_H) - self.px(self.sprite_top + 2.0);
        let y = bottom - img.h as i32;
        let alpha = self.bubble.as_ref().map(|b| b.alpha).unwrap_or(0.0);
        self.bubble = Some(Bubble { img, x, y, alpha, target: 1.0 });
        unsafe { let _ = ShowWindow(self.bubble_hwnd, SW_SHOWNOACTIVATE); }
        self.fade_step();
        self.timer(T_BUBBLE_HIDE, secs);
    }

    fn hide_bubble(&mut self) {
        self.kill(T_BUBBLE_HIDE);
        if let Some(b) = &mut self.bubble { b.target = 0.0; self.fade_step(); }
    }

    fn hide_bubble_now(&mut self) {
        self.kill(T_BUBBLE_HIDE);
        self.kill(T_FADE);
        self.bubble = None;
        unsafe { let _ = ShowWindow(self.bubble_hwnd, SW_HIDE); }
    }

    /// Belirme 0,15 sn, kaybolma 0,2 sn.
    fn fade_step(&mut self) {
        let Some(b) = &mut self.bubble else { return };
        let step = if b.target > b.alpha { 1.0 / (0.15 * 60.0) } else { 1.0 / (0.2 * 60.0) };
        b.alpha = if b.target > b.alpha { (b.alpha + step).min(1.0) } else { (b.alpha - step).max(0.0) };
        let a = (b.alpha * 255.0).round() as u8;
        let (x, y) = (b.x, b.y);
        let done = (b.alpha - b.target).abs() < 1e-6;
        let gone = done && b.target == 0.0;
        let img = std::mem::replace(&mut b.img, Img::new(0, 0));
        self.bsurf.present(self.bubble_hwnd, &img, x, y, a);
        if let Some(b) = &mut self.bubble { b.img = img; }
        if gone { self.hide_bubble_now(); return; }
        if done { self.kill(T_FADE); } else { self.timer(T_FADE, 1.0 / 60.0); }
    }

    // MARK: Tutma

    fn drag_start(&mut self, p: (i32, i32)) {
        self.dragging = true;
        self.kill(T_IDLE);
        self.hide_bubble_now();
        self.mid_x = 0.0; self.mid_v = 0.0; self.tip_x = 0.0; self.tip_v = 0.0; self.vx_smooth = 0.0;
        self.last_frame = None;
        self.land_start = None;
        self.fx.clear();
        let c = (p.0 as f64 / self.scale, p.1 as f64 / self.scale);
        self.cursor = c; self.prev_cursor = c;
        self.anim = Anim::dangle();
        self.last_sig = 0;
        self.drag_to(p);
        self.timer(T_DRAG, 0.01);
    }

    fn drag_to(&mut self, p: (i32, i32)) {
        self.cursor = (p.0 as f64 / self.scale, p.1 as f64 / self.scale);
        // Ense (karenin üst ortası, üstten 9 pt) imlecin tam altında kalır.
        self.pos = (p.0 - self.px(DRAG_W / 2.0), p.1 - self.px(9.0));
        self.render_drag();
    }

    /// Ekran yenilemesiyle: imlecin yumuşatılmış hızına göre hedef, iki yay (gövde ortası ve ayak ucu).
    fn swing_frame(&mut self) {
        if !self.dragging { return; }
        let now = Instant::now();
        let dt = self.last_frame.map(|l| now.duration_since(l).as_secs_f64().clamp(1.0 / 240.0, 1.0 / 30.0)).unwrap_or(1.0 / 60.0);
        self.last_frame = Some(now);
        let vx = (self.cursor.0 - self.prev_cursor.0) / dt;
        self.prev_cursor = self.cursor;
        self.vx_smooth += (vx - self.vx_smooth) * (dt * 10.0).min(1.0);
        let target = (-self.vx_smooth * 0.03).clamp(-24.0, 24.0);
        self.mid_v += (200.0 * (target * 0.35 - self.mid_x) - 20.0 * self.mid_v) * dt;
        self.mid_x += self.mid_v * dt;
        self.tip_v += (55.0 * (target - self.tip_x) - 4.5 * self.tip_v) * dt;
        self.tip_x += self.tip_v * dt;
        self.render_drag();
    }

    fn drag_end(&mut self) {
        self.dragging = false;
        self.kill(T_DRAG);
        let left = self.pos.0 + (self.px(DRAG_W) - self.px(CAT_W)) / 2;
        let top = self.pos.1 + self.px(DRAG_H) - self.px(CAT_H);
        self.pos = self.clamped((left, top));
        let p = self.pos;
        settings::update(|s| s.cat_home = Some([p.0, p.1]));
        // Bırakınca hemen oturur; yere konar gibi kısa bir esneme (zıplama yok).
        self.land_start = Some(Instant::now());
        self.play_base();
        self.schedule_idle();
    }

    // MARK: Kutu, görünürlük, güç

    pub fn set_in_box(&mut self, inbox: bool) {
        settings::update(|s| s.cat_in_box = inbox);
        self.hidden = inbox;
        self.apply_visibility();
    }

    fn apply_visibility(&mut self) {
        if self.visible() {
            unsafe { let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE); }
            self.last_sig = 0;
            self.play_base();
            self.schedule_idle();
        } else {
            self.hide_bubble_now();
            self.kill(T_IDLE);
            self.kill(T_ANIM);
            unsafe { let _ = ShowWindow(self.hwnd, SW_HIDE); }
        }
    }

    fn set_paused(&mut self) {
        // Bilgisayar günlüğü: ekran kapalı ya da kilitliyken kayıt yok.
        crate::activity::send(if self.screen_off || self.locked { crate::activity::Msg::Pause } else { crate::activity::Msg::Resume });
        let p = self.screen_off || self.locked || self.saver;
        if p == self.paused { return; }
        self.paused = p;
        if p { self.hide_bubble_now(); self.kill(T_IDLE); self.fx.clear(); self.last_sig = 0; self.render(); }
        else { self.play_base(); self.schedule_idle(); }
    }

    /// Tam ekran uygulama/oyun/sunum durumu (yalnızca geliştirme günlüğü için; kediyi gizlemez).
    pub fn check_fullscreen(&mut self) {
        use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
        let st = unsafe { SHQueryUserNotificationState() };
        let fs = matches!(st, Ok(s) if s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE);
        self.fullscreen = fs;
    }

    /// Başka bir "her zaman üstte" pencere öne geçtiyse kediyi yeniden en üste al (yalnızca öndeki pencere değişince).
    /// Başka bir sanal masaüstüne geçildiyse kediyi oraya taşır: kedi hep ekranda kalır.
    fn keep_on_top(&self, fg: HWND) {
        if !self.visible() { return; }
        follow_desktop(self.hwnd, fg);
        unsafe {
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER);
            if IsWindowVisible(self.bubble_hwnd).as_bool() {
                let _ = SetWindowPos(self.bubble_hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER);
            }
        }
    }

    // MARK: Tanıtım (ULLAGECAT_DEMO=1): tüm balon türleri sırayla.

    fn demo_step(&mut self) {
        let Some(i) = self.demo else { return };
        let d = Self::duration;
        let steps: Vec<(Anim, String, bool)> = vec![
            (Anim::laptop(), format!("{}\nmeowly · {}", l10n::t("✎ Not alıyorum…"), d(45.0 * 60.0)), false),
            (Anim::laptop(), format!("{}\nai_limit", l10n::t("✎ Yeni sayfa açtım")), false),
            (Anim::laptop(), format!("{}{}\n{}", d(90.0 * 60.0), l10n::t(" oldu"), l10n::t("Biraz esneyelim mi?")), false),
            (Anim::laptop(), format!("{}\n{}", l10n::f("Bugün %@ token!", &["1M".into()]), l10n::t("Deftere yazdım")), false),
            (Anim::idle(), format!("{}\nmeowly · {}", l10n::t("Oturumu kaydettim"), d(80.0 * 60.0)), false),
            (Anim::content(), l10n::t("Mırr…"), true),
            (Anim::idle(), format!("{}: {}{}\n{}", l10n::t("Dün"), d(190.0 * 60.0), l10n::f(" · %lld proje", &["2".into()]), l10n::t("Rapor hazır, bana tıkla")), false),
        ];
        if i >= steps.len() {
            self.demo = None;
            self.busy_until = Instant::now();
            self.play_base();
            self.schedule_idle();
            return;
        }
        let (a, text, heart) = steps[i].clone();
        self.play(a);
        if heart { self.pop_heart(); }
        self.say(&text, 4.2, true);
        self.demo = Some(i + 1);
        self.timer(T_DEMO, 5.0);
    }

    // MARK: Mesajlar

    fn handle(&mut self, msg: u32, wp: WPARAM, lp: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_TIMER => {
                match wp.0 {
                    T_ANIM => { self.kill(T_ANIM); self.render(); }
                    T_IDLE => { self.kill(T_IDLE); self.idle_tick(); }
                    T_UPDATE => {}
                    T_ONESHOT => {
                        self.kill(T_ONESHOT);
                        if !self.dragging && Instant::now() >= self.busy_until { self.play_base(); }
                    }
                    T_BUBBLE_HIDE => self.hide_bubble(),
                    T_FADE => self.fade_step(),
                    T_DRAG => self.swing_frame(),
                    T_DEMO => { self.kill(T_DEMO); self.demo_step(); }
                    T_HOVER => self.hover_poll(),
                    T_DAILY => { self.kill(T_DAILY); self.daily_note(); }
                    _ => {}
                }
                Some(LRESULT(0))
            }
            WM_DATA => {
                self.update();
                crate::strip::refresh();
                crate::ipc::write_ui_state();
                Some(LRESULT(0))
            }
            WM_LBUTTONDOWN => {
                self.down_at = Some(win::cursor());
                self.dragged = false;
                unsafe { SetCapture(self.hwnd); }
                Some(LRESULT(0))
            }
            WM_MOUSEMOVE => {
                let p = win::cursor();
                if !self.tracking {
                    let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: self.hwnd, dwHoverTime: 0 };
                    unsafe { let _ = TrackMouseEvent(&mut tme); }
                    self.tracking = true;
                    self.kill(T_HOVER);
                }
                if let Some(d) = self.down_at {
                    let dist = (((p.0 - d.0) as f64).powi(2) + ((p.1 - d.1) as f64).powi(2)).sqrt() / self.scale;
                    if !self.dragged && dist > 3.0 { self.dragged = true; self.drag_start(p); }
                    if self.dragged { self.drag_to(p); }
                } else {
                    self.hover_move(p);
                }
                Some(LRESULT(0))
            }
            0x02A3 => {
                // WM_MOUSELEAVE: kedinin pikselinden çıktı. Mac'teki gibi okşama alanı tüm pencere (145×125):
                // imleç hâlâ pencere alanındaysa yalnızca o süre boyunca konumu izlenir, alandan çıkınca durur.
                self.tracking = false;
                if self.in_rect(win::cursor()) { self.timer(T_HOVER, 0.03); }
                else { self.last_hover = None; self.stroke = 0.0; }
                Some(LRESULT(0))
            }
            WM_LBUTTONUP => {
                unsafe { let _ = ReleaseCapture(); }
                if self.dragged { self.drag_end(); } else if self.down_at.is_some() { self.click(); }
                self.down_at = None;
                self.dragged = false;
                Some(LRESULT(0))
            }
            WM_CAPTURECHANGED => {
                if self.dragging { self.drag_end(); }
                self.down_at = None;
                self.dragged = false;
                Some(LRESULT(0))
            }
            WM_DPICHANGED => {
                self.scale = win::dpi_scale(self.hwnd);
                self.scaled.clear();
                self.pos = self.clamped(self.pos);
                self.last_sig = 0;
                self.render();
                Some(LRESULT(0))
            }
            WM_DISPLAYCHANGE => { self.pos = self.clamped(self.pos); self.last_sig = 0; self.render(); None }
            WM_WTSSESSION_CHANGE => {
                match wp.0 as u32 { 7 => self.locked = true, 8 => self.locked = false, _ => {} } // WTS_SESSION_LOCK / UNLOCK
                self.set_paused();
                Some(LRESULT(0))
            }
            WM_POWERBROADCAST if wp.0 == 0x8013 => {
                // PBT_POWERSETTINGCHANGE
                #[repr(C)]
                struct Pbs { guid: windows::core::GUID, len: u32, data: u32 }
                let p = unsafe { &*(lp.0 as *const Pbs) };
                dev_log(&format!("power {:?} len={} data={}", p.guid, p.len, p.data));
                if p.guid == GUID_CONSOLE_DISPLAY_STATE { self.screen_off = p.data == 0; }
                if p.guid == GUID_POWER_SAVING_STATUS { self.saver = p.data != 0; }
                self.set_paused();
                Some(LRESULT(1))
            }
            _ => None,
        }
    }

    /// Okşama: imleç kedinin alanında toplam ~160 pt gidip gelirse.
    fn hover_move(&mut self, p: (i32, i32)) {
        if let Some(l) = self.last_hover {
            self.stroke += (((p.0 - l.0) as f64).powi(2) + ((p.1 - l.1) as f64).powi(2)).sqrt() / self.scale;
        }
        self.last_hover = Some(p);
        if self.stroke > 160.0 { self.stroke = 0.0; self.pet(); }
    }

    fn in_rect(&self, p: (i32, i32)) -> bool {
        let (w, h) = (self.px(CAT_W), self.px(CAT_H));
        p.0 >= self.pos.0 && p.0 < self.pos.0 + w && p.1 >= self.pos.1 && p.1 < self.pos.1 + h
    }

    /// Yalnızca imleç kedinin pikselleri dışında ama pencere alanı içindeyken çalışır.
    fn hover_poll(&mut self) {
        let p = win::cursor();
        if self.tracking || self.dragging || !self.visible() { self.kill(T_HOVER); return; }
        if !self.in_rect(p) { self.kill(T_HOVER); self.last_hover = None; self.stroke = 0.0; return; }
        if self.last_hover != Some(p) { self.hover_move(p); }
    }

    fn click(&mut self) {
        self.hide_bubble();
        // Pano henüz yok (adım 1d). "Rapor hazır" balonundan sonra dünün raporu, değilse Günlük açılacak.
        // "Rapor hazır" balonundan sonraki 10 dk içinde tıklanırsa dünün raporu, değilse Günlük açılır.
        if self.report_click_until.take().map(|t| Instant::now() < t).unwrap_or(false) {
            crate::ipc::spawn_ui("dashboard:reports-yesterday");
        } else {
            crate::ipc::spawn_ui("dashboard:diary");
        }
    }
}

/// Pikselli desen (z, kalp): `left`, `top` pt cinsinden, pencere tabanından yukarı.
fn draw_cells(out: &mut Img, rows: &[&str], left: f64, top: f64, cell: f64, s: f64, h: f64, rgb: (u8, u8, u8), op: f32) {
    if op <= 0.0 { return; }
    let c = rgba(rgb.0, rgb.1, rgb.2, op);
    for (ry, r) in rows.iter().enumerate() {
        for (rx, ch) in r.chars().enumerate() {
            if ch != '#' { continue; }
            let x0 = ((left + rx as f64 * cell) * s).round() as i32;
            let x1 = ((left + (rx + 1) as f64 * cell) * s).round() as i32;
            let y0 = (h - (top - ry as f64 * cell) * s).round() as i32;
            let y1 = (h - (top - (ry + 1) as f64 * cell) * s).round() as i32;
            out.fill(x0, y0, (x1 - x0).max(1), (y1 - y0).max(1), c);
        }
    }
}

// MARK: Balon çizimi (BubbleView)

const PAD: f64 = 8.0;
const TAIL: f64 = 12.0;
const EDGE: f64 = 2.0;

fn bubble_lines(text: &str) -> Vec<(String, f32, Weight, f32)> {
    text.split('\n').enumerate()
        .map(|(i, l)| if i == 0 { (l.to_string(), 10.5, Weight::SemiBold, 1.0) } else { (l.to_string(), 10.0, Weight::Medium, 0.62) })
        .collect()
}

/// SF Mono ile aynı satır yüksekliği: yaklaşık 1,19 × punto.
fn line_h(size: f32) -> f64 { (size as f64 * 1.19).round() }

fn text_bounds(text: &str) -> (f64, f64) {
    let lines = bubble_lines(text);
    let w = lines.iter().map(|l| text::width(&l.0, l.1, l.2) as f64).fold(0.0, f64::max).min(180.0);
    let h: f64 = lines.iter().map(|l| line_h(l.1)).sum::<f64>() + (lines.len() as f64 - 1.0);
    (w, h)
}

/// Balonun boyutu (pt).
fn bubble_size(text: &str) -> (f64, f64) {
    let (tw, th) = text_bounds(text);
    let w = tw.ceil() + PAD * 2.0 + EDGE * 2.0 + 6.0;
    let h = th.ceil() + PAD * 1.4 + EDGE * 2.0;
    (((w / 2.0).ceil() * 2.0).max(44.0), (h / 2.0).ceil() * 2.0 + TAIL + 2.0)
}

fn bubble_image(text: &str, bw: f64, bh: f64, tail_x: f64, s: f64) -> Img {
    let (wpx, hpx) = ((bw * s).round() as usize, (bh * s).round() as usize);
    let mut out = Img::new(wpx, hpx);
    let hh = hpx as f64;
    // pt cinsinden, tabandan yukarı dikdörtgen → piksel.
    let rect = |out: &mut Img, x: f64, y: f64, w: f64, h: f64, c: [u8; 4]| {
        let x0 = (x * s).round() as i32;
        let x1 = ((x + w) * s).round() as i32;
        let y0 = (hh - (y + h) * s).round() as i32;
        let y1 = (hh - y * s).round() as i32;
        out.fill(x0, y0, x1 - x0, y1 - y0, c);
    };
    let notched = |out: &mut Img, x: f64, y: f64, w: f64, h: f64, e: f64, c: [u8; 4]| {
        rect(out, x + 2.0 * e, y, w - 4.0 * e, h, c);
        rect(out, x + e, y + e, w - 2.0 * e, h - 2.0 * e, c);
        rect(out, x, y + 2.0 * e, w, h - 4.0 * e, c);
    };
    let e = EDGE;
    let (bx, by, bwid, bhei) = (0.0, TAIL, bw - 2.0, bh - TAIL - 2.0);
    let mid = ((tail_x.clamp(12.0, bx + bwid - 12.0)) / 2.0).round() * 2.0;
    let ink = rgba(INK.0, INK.1, INK.2, 1.0);
    let white = rgba(255, 255, 255, 1.0);
    // Gölge
    notched(&mut out, bx + 2.0, by - 2.0, bwid, bhei, e, rgba(INK.0, INK.1, INK.2, 0.18));
    // Dış çizgi + kuyruk
    notched(&mut out, bx, by, bwid, bhei, e, ink);
    for (i, w) in [14.0, 10.0, 6.0].iter().enumerate() {
        rect(&mut out, mid - w / 2.0, TAIL - (i as f64 + 1.0) * 2.0 * e, *w, 2.0 * e, ink);
    }
    // İç
    notched(&mut out, bx + e, by + e, bwid - 2.0 * e, bhei - 2.0 * e, e, white);
    rect(&mut out, mid - 5.0, TAIL - 2.0 * e, 10.0, 3.0 * e, white);
    rect(&mut out, mid - 3.0, TAIL - 4.0 * e, 6.0, 2.0 * e, white);
    rect(&mut out, mid - 1.0, TAIL - 5.0 * e, 2.0, e, white);
    // Metin: iç alanda dikey ortalı, satırlar yatay ortalı.
    let (_, th) = text_bounds(text);
    let inner_x = bx + e + PAD;
    let inner_w = bwid - 2.0 * (e + PAD);
    let inner_mid = by + bhei / 2.0;
    let mut top = inner_mid + th / 2.0 + 1.0; // tabandan yukarı
    for (line, size, wt, a) in bubble_lines(text) {
        let lh = line_h(size);
        let (asc, desc, _) = text::line_metrics(size, wt);
        let ascent = asc as f64 * lh / (asc + desc) as f64;
        let lw = text::width(&line, size, wt) as f64;
        let x = inner_x + (inner_w - lw) / 2.0;
        let base = top - ascent;
        text::draw(&mut out, &line, (x * s) as f32, (hh - base * s) as f32, size * s as f32, wt, INK, a);
        top -= lh + 1.0;
    }
    out
}

// MARK: Pencere işlemleri

pub fn with<R>(f: impl FnOnce(&mut Cat) -> R) -> Option<R> {
    CAT.with(|c| c.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

const GUID_CONSOLE_DISPLAY_STATE: windows::core::GUID = windows::core::GUID::from_u128(0x6fe69556_704a_47a0_8f24_c28d936fda47);
const GUID_POWER_SAVING_STATUS: windows::core::GUID = windows::core::GUID::from_u128(0xe00958c0_c213_4ace_ac77_fecced2eeea5);

const MENU_SETTINGS: usize = 1;
const MENU_BOX: usize = 2;

extern "system" fn cat_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_INPUT => { sense::key_event(); return DefWindowProcW(hwnd, msg, wp, lp); }
            WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
            WM_RBUTTONUP => { show_menu(hwnd); return LRESULT(0); }
            _ => {}
        }
        match with(|c| c.handle(msg, wp, lp)).flatten() {
            Some(r) => r,
            None => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

extern "system" fn bubble_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_MOUSEACTIVATE { return LRESULT(MA_NOACTIVATE as isize); }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

fn to_wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

fn show_menu(hwnd: HWND) {
    unsafe {
        let Ok(m) = CreatePopupMenu() else { return };
        let a = to_wide(&l10n::t("Ayarlar"));
        let b = to_wide(&l10n::t("Tonti'yi kutuya koy"));
        let _ = AppendMenuW(m, MF_STRING, MENU_SETTINGS, windows::core::PCWSTR(a.as_ptr()));
        let _ = AppendMenuW(m, MF_STRING, MENU_BOX, windows::core::PCWSTR(b.as_ptr()));
        let (x, y) = win::cursor();
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, x, y, None, hwnd, None);
        let _ = DestroyMenu(m);
        match cmd.0 as usize {
            MENU_BOX => { with(|c| c.set_in_box(true)); crate::strip::refresh(); }
            MENU_SETTINGS => crate::ipc::spawn_ui("dashboard:appearance"),
            _ => {}
        }
    }
}

extern "system" fn foreground_changed(_: windows::Win32::UI::Accessibility::HWINEVENTHOOK, _: u32, hwnd: HWND, _: i32, _: i32, _: u32, _: u32) {
    with(|c| { c.check_fullscreen(); c.keep_on_top(hwnd); });
    crate::activity::send(crate::activity::Msg::Foreground(hwnd.0 as isize));
}

/// Kediyi başlatır (ana iş parçacığında, Tauri olay döngüsü mesajları dağıtır).
pub fn start() {
    win::register_class(w!("MeowlyCat"), Some(cat_proc));
    win::register_class(w!("MeowlyBubble"), Some(bubble_proc));
    let hwnd = win::create_layered(w!("MeowlyCat"), false, None);
    let bubble = win::create_layered(w!("MeowlyBubble"), true, Some(hwnd));
    let mut cat = Cat::new(hwnd, bubble);
    let st = settings::get();
    cat.pos = cat.clamped(st.cat_home.map(|p| (p[0], p[1])).unwrap_or_else(|| cat.default_home()));
    cat.hidden = st.cat_in_box;
    unsafe {
        use windows::Win32::System::RemoteDesktop::{WTSRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION};
        let _ = WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION);
        let _ = windows::Win32::System::Power::RegisterPowerSettingNotification(HANDLE(hwnd.0), &GUID_CONSOLE_DISPLAY_STATE, DEVICE_NOTIFY_WINDOW_HANDLE);
        let _ = windows::Win32::System::Power::RegisterPowerSettingNotification(HANDLE(hwnd.0), &GUID_POWER_SAVING_STATUS, DEVICE_NOTIFY_WINDOW_HANDLE);
        use windows::Win32::UI::Accessibility::SetWinEventHook;
        SetWinEventHook(EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND, None, Some(foreground_changed), 0, 0, WINEVENT_OUTOFCONTEXT);
    }
    sense::register_keyboard(hwnd);
    CAT.with(|c| *c.borrow_mut() = Some(cat));
    with(|c| {
        c.check_fullscreen();
        c.base = c.compute_base();
        c.update();
        CAT_HWND.store(c.hwnd.0 as isize, std::sync::atomic::Ordering::Relaxed);
        if c.visible() { unsafe { let _ = ShowWindow(c.hwnd, SW_SHOWNOACTIVATE); } }
        c.play_base();
        c.schedule_idle();
        if env("ULLAGECAT_DEMO").is_some() { c.demo = Some(0); c.busy_until = Instant::now() + Duration::from_secs(3600); c.timer(T_DEMO, 1.5); }
    });
}

/// Kedi başka bir sanal masaüstünde kaldıysa öndeki pencerenin masaüstüne taşınır (Windows'un belgelenmiş arayüzü).
fn follow_desktop(cat: HWND, fg: HWND) {
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager};
    thread_local! { static VDM: Option<IVirtualDesktopManager> = unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_ALL).ok()
    }; }
    if fg.0.is_null() { return; }
    VDM.with(|m| unsafe {
        let Some(m) = m else { return };
        if m.IsWindowOnCurrentVirtualDesktop(cat).map(|b| b.as_bool()).unwrap_or(true) { return; }
        if let Ok(id) = m.GetWindowDesktopId(fg) { let _ = m.MoveWindowToDesktop(cat, &id); }
    });
}

pub fn toggle_box() {
    with(|c| { let h = c.hidden; c.set_in_box(!h); });
}

pub fn in_box() -> bool { settings::get().cat_in_box }

// MARK: Geliştirme: tüm durumları PNG olarak çizer ve çıkar (MEOWLY_GALLERY=klasör).

pub fn gallery(dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    let s: f64 = std::env::var("MEOWLY_PT_SCALE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let still = |name: &'static str| {
        let img = source(name).resample(s / 2.0);
        let (w, h) = ((CAT_W * s).round() as usize, (CAT_H * s).round() as usize);
        let mut out = Img::new(w, h);
        let x0 = ((w as i32 - img.w as i32) as f64 / 2.0).round() as i32;
        out.blit_rows(&img, 0, img.h, x0, h as i32 - img.h as i32, img.h, 1.0, &|_| 0);
        out
    };
    for n in ["idle_0", "type_0", "type_1c", "sleep", "yawn", "grumpy", "tired", "content", "groomC"] {
        still(n).save_png(&dir.join(format!("{n}.png")));
    }
    // Yere konma (bırakınca): bölünmüş oturuş, basmanın en derin anları. "eski" = baş gövdeyle inmiyordu (boyunda boşluk).
    {
        let name = "idle_0";
        let img = source(name).resample(s / 2.0);
        let src_h = source(name).h;
        let (w, h) = ((CAT_W * s).round() as usize, (CAT_H * s).round() as usize);
        let body_rows_src = (src_h as f64 * (1.0 - 0.52)).round() as usize;
        let body_rows = ((body_rows_src as f64 * img.h as f64 / src_h as f64).round() as usize).min(img.h);
        let px = |pt: f64| (pt * s).round() as i32;
        let mut sheet = Img::new(w * 6, h * 2);
        for (i, t) in [0.0, 0.02, 0.05, 0.08, 0.15, 0.4].into_iter().enumerate() {
            let sc = land_at(t);
            let body_dh = (body_rows as f64 * sc).round() as usize;
            let old = compose(&img, src_h, true, w, h, body_rows, body_dh, h as i32 - px((body_rows_src as f64 / 2.0) - 1.5), 1.0);
            let new = compose(&img, src_h, true, w, h, body_rows, body_dh, h as i32 - px((body_rows_src as f64 / 2.0) * sc - 1.5), sc);
            for (row, frame) in [(0usize, &old), (1, &new)] {
                for y in 0..h { for x in 0..w { sheet.px[(row * h + y) * sheet.w + i * w + x] = frame.px[y * w + x]; } }
            }
        }
        sheet.save_png(&dir.join("land_eski_ust_yeni_alt.png"));
    }
    // Uyku + z (yükselişin ortası)
    let mut sl = still("sleep");
    let slh = sl.h as f64;
    let top = source("sleep").h as f64 / 2.0 + 2.0;
    draw_cells(&mut sl, &ZED, CAT_W / 2.0 + 22.0 - 5.0, top + 9.0 + 5.0, 2.0, s, slh, (140, 153, 191), 1.0);
    draw_cells(&mut sl, &ZED, CAT_W / 2.0 + 30.0 - 3.5, top + 3.0 + 3.5, 1.4, s, slh, (140, 153, 191), 1.0);
    sl.save_png(&dir.join("sleep_z.png"));
    // Bükülme: imleç sağa hızla giderken (gövde sola, ayaklar daha çok sola)
    for (tag, m, t) in [("drag_bend", -8.4f64, -24.0f64), ("drag_straight", 0.0, 0.0), ("drag_swing", 6.0, 14.0)] {
        let src = source("dangle");
        let img = src.resample(s / DANGLE_SCALE);
        let count = (src.h / 2).max(8);
        let (w, h) = ((DRAG_W * s).round() as usize, (DRAG_H * s).round() as usize);
        let mut out = Img::new(w, h);
        let x0 = ((w as i32 - img.w as i32) as f64 / 2.0).round() as i32;
        let sh: Vec<i32> = (0..img.h).map(|r| {
            let sr = ((r as f64 + 0.5) * src.h as f64 / img.h as f64) as usize;
            let st = (sr * count / src.h).min(count - 1);
            let sc = (st as f64 + 0.5) / count as f64;
            let off = m * sc.powf(1.6) + (t - m) * sc.powf(3.2);
            (((off * DANGLE_SCALE).round() / DANGLE_SCALE) * s).round() as i32
        }).collect();
        out.blit_rows(&img, 0, img.h, x0, 0, img.h, 1.0, &|r| sh[r]);
        out.save_png(&dir.join(format!("{tag}.png")));
    }
    // Balonlar
    let texts = [
        format!("{}\nmeowly · 45 dk", l10n::t("✎ Not alıyorum…")),
        l10n::t("Mırr…"),
        format!("1 sa 30 dk{}\n{}", l10n::t(" oldu"), l10n::t("Biraz esneyelim mi?")),
    ];
    for (i, t) in texts.iter().enumerate() {
        let (bw, bh) = bubble_size(t);
        bubble_image(t, bw, bh, bw / 2.0, s).save_png(&dir.join(format!("bubble_{i}.png")));
    }
}
