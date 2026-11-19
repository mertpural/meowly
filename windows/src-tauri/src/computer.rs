//! Günlük ve Raporlar için bilgisayar günlüğü özeti (Mac: Tonti/ComputerUI.swift → ComputerDay, AppIcons). Arayüz sürecinde.

use crate::diary::duration;
use crate::img::Img;
use crate::{look, store};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Uygulamanın simgesi (exe'den, 32×32 PNG veri adresi). Önbellekte tutulur.
pub fn icon_url(path: &str) -> Option<String> {
    static C: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    let c = C.get_or_init(Default::default);
    if let Some(v) = c.lock().unwrap().get(path) { return v.clone(); }
    let v = extract_icon(path).map(|png| format!("data:image/png;base64,{}", crate::settings_ui::b64(&png)));
    c.lock().unwrap().insert(path.to_string(), v.clone());
    v
}

fn extract_icon(path: &str) -> Option<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::Shell::ExtractIconExW;
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
    let w: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut large = HICON::default();
        if ExtractIconExW(windows::core::PCWSTR(w.as_ptr()), 0, Some(&mut large), None, 1) == 0 || large.is_invalid() { return None; }
        let mut ii = ICONINFO::default();
        let ok = GetIconInfo(large, &mut ii).is_ok();
        let out = (|| {
            if !ok { return None; }
            let dc = GetDC(None);
            let mut bi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32, ..Default::default() }, ..Default::default() };
            GetDIBits(dc, ii.hbmColor, 0, 0, None, &mut bi, DIB_RGB_COLORS);
            let (wd, ht) = (bi.bmiHeader.biWidth.max(1) as usize, bi.bmiHeader.biHeight.unsigned_abs().max(1) as usize);
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB.0;
            bi.bmiHeader.biHeight = -(ht as i32);
            let mut buf = vec![0u8; wd * ht * 4];
            let n = GetDIBits(dc, ii.hbmColor, 0, ht as u32, Some(buf.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
            ReleaseDC(None, dc);
            if n == 0 { return None; }
            let has_alpha = buf.chunks(4).any(|p| p[3] != 0);
            let mut img = Img::new(wd, ht);
            for (i, p) in buf.chunks(4).enumerate() {
                let a = if has_alpha { p[3] } else { 255 };
                let pm = |c: u8| ((c as u32 * a as u32 + 127) / 255) as u8;
                img.px[i] = [pm(p[0]), pm(p[1]), pm(p[2]), a];
            }
            Some(img.png_bytes())
        })();
        if !ii.hbmColor.is_invalid() { let _ = DeleteObject(ii.hbmColor.into()); }
        if !ii.hbmMask.is_invalid() { let _ = DeleteObject(ii.hbmMask.into()); }
        let _ = DestroyIcon(large);
        out
    }
}

/// Bir günün (ya da aralığın) ayrıntılı özeti: saatlik dağılım, uygulamalar, başlıklar/siteler, kategoriler, dosyalar.
pub fn day(from: f64, to: f64) -> Value {
    struct A { name: String, cat: String, time: f64, titles: HashMap<String, f64>, domains: HashMap<String, f64> }
    let spans: Vec<store::Span> = store::spans(from, to).into_iter().filter(|s| s.end - s.start >= 3.0).collect();
    let mut by: HashMap<String, A> = HashMap::new();
    let mut cats: HashMap<String, f64> = HashMap::new();
    let mut hours = [0f64; 24];
    let mut active = 0.0;
    for s in &spans {
        let d = s.end - s.start;
        let a = by.entry(s.bundle.clone()).or_insert_with(|| A { name: s.app.clone(), cat: s.category.clone(), time: 0.0, titles: HashMap::new(), domains: HashMap::new() });
        a.time += d;
        if let Some(t) = &s.title { *a.titles.entry(t.clone()).or_default() += d; }
        if let Some(dm) = &s.domain { *a.domains.entry(dm.clone()).or_default() += d; }
        *cats.entry(s.category.clone()).or_default() += d;
        active += d;
        // Saatlik dağılım (saat sınırını aşan kayıtlar bölünür).
        let mut t = s.start;
        while t < s.end {
            let h = look::local(t).3 as usize;
            let next = (t - (t - store::start_of_day(t)) % 3600.0 + 3600.0).min(s.end);
            hours[h.min(23)] += next - t;
            if next <= t { break; }
            t = next;
        }
    }
    let mut apps: Vec<(String, A)> = by.into_iter().collect();
    apps.sort_by(|a, b| b.1.time.partial_cmp(&a.1.time).unwrap());
    let top = apps.first().map(|a| a.1.time).unwrap_or(1.0);
    let sorted = |m: &HashMap<String, f64>, n: usize| {
        let mut v: Vec<(&String, &f64)> = m.iter().collect();
        v.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        v.into_iter().take(n).map(|(k, t)| json!({ "t": k, "dur": duration(*t) })).collect::<Vec<_>>()
    };
    let mut catv: Vec<(String, f64)> = cats.into_iter().collect();
    catv.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    json!({
        "active": duration(active),
        "activeSecs": active,
        "hours": hours.to_vec(),
        "apps": apps.iter().take(25).map(|(k, a)| {
            let details = if a.titles.is_empty() { sorted(&a.domains, 8) } else { sorted(&a.titles, 6) };
            json!({ "key": k, "name": a.name, "category": crate::l10n::t(&a.cat), "dur": duration(a.time), "frac": a.time / top,
                    "details": details, "icon": icon_url(k) })
        }).collect::<Vec<_>>(),
        "categories": catv.iter().take(6).map(|(c, t)| json!({ "name": crate::l10n::t(c), "dur": duration(*t) })).collect::<Vec<_>>(),
        "files": store::file_activity(from, to).into_iter().take(6).map(|(p, n)| json!({ "project": p, "count": n })).collect::<Vec<_>>(),
        // Takvim (Mac: ComputerSide → TAKVİM): başlık ve saat yalnızca Günlük'te görünür.
        "meetings": crate::calendar::meetings(from, to).iter().map(|m| json!({ "title": m.title, "range": format!("{}–{}", hm(m.start), hm(m.end)) })).collect::<Vec<_>>(),
    })
}

fn hm(t: f64) -> String { let l = look::local(t); format!("{:02}:{:02}", l.3, l.4) }

/// Raporlar için hafif özet: yalnızca uygulama toplamları (başlık/site yok) ve toplantıların toplam süresi.
pub fn summary(from: f64, to: f64) -> Value {
    let rows = store::app_totals(from, to);
    let active: f64 = rows.iter().map(|r| r.3).sum();
    let top = rows.first().map(|r| r.3).unwrap_or(1.0).max(1.0);
    let meet = crate::calendar::total(&crate::calendar::meetings(from, to));
    json!({
        "active": active,
        "activeText": duration(active),
        "meeting": meet,
        "meetingText": duration(meet),
        "apps": rows.iter().take(5).map(|(k, n, _c, t)| json!({ "name": n, "dur": duration(*t), "frac": t / top, "icon": icon_url(k) })).collect::<Vec<_>>(),
    })
}
