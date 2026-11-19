//! Katmanlı (layered) Win32 penceresi: piksel başına saydamlık. Tamamen saydam piksellere gelen tıklama
//! Windows tarafından arkadaki pencereye geçirilir; imleç yoklamaya gerek kalmaz.

use crate::img::Img;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn register_class(name: PCWSTR, proc_: WNDPROC) {
    unsafe {
        let hinst = GetModuleHandleW(None).unwrap();
        let wc = WNDCLASSW {
            lpfnWndProc: proc_,
            hInstance: hinst.into(),
            lpszClassName: name,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        RegisterClassW(&wc);
    }
}

/// Kenarlıksız, en üstte, görev çubuğunda görünmeyen, odak çalmayan katmanlı pencere.
pub fn create_layered(class: PCWSTR, click_through: bool, owner: Option<HWND>) -> HWND {
    unsafe {
        let mut ex = WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;
        if click_through { ex |= WS_EX_TRANSPARENT; }
        let hinst = GetModuleHandleW(None).unwrap();
        CreateWindowExW(ex, class, w!("Meowly"), WS_POPUP, 0, 0, 1, 1, owner, None, Some(hinst.into()), None).unwrap()
    }
}

pub fn dpi_scale(hwnd: HWND) -> f64 {
    if let Some(v) = std::env::var("MEOWLY_PT_SCALE").ok().and_then(|v| v.parse::<f64>().ok()) { return v; }
    let d = unsafe { GetDpiForWindow(hwnd) };
    if d == 0 { 1.0 } else { d as f64 / 96.0 }
}

/// Önbellekli DIB: aynı boyutta tekrar tekrar ayrılmaz.
pub struct Surface {
    w: usize,
    h: usize,
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u8,
}

impl Surface {
    pub fn new() -> Surface { Surface { w: 0, h: 0, dc: HDC::default(), bmp: HBITMAP::default(), old: HGDIOBJ::default(), bits: std::ptr::null_mut() } }

    fn ensure(&mut self, w: usize, h: usize) {
        if w == self.w && h == self.h && !self.bits.is_null() { return; }
        self.free();
        unsafe {
            let screen = GetDC(None);
            self.dc = CreateCompatibleDC(Some(screen));
            ReleaseDC(None, screen);
            let bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    biHeight: -(h as i32), // yukarıdan aşağı
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
            self.bmp = CreateDIBSection(Some(self.dc), &bi, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
            self.old = SelectObject(self.dc, self.bmp.into());
            self.bits = bits as *mut u8;
            self.w = w;
            self.h = h;
        }
    }

    fn free(&mut self) {
        unsafe {
            if !self.dc.is_invalid() {
                SelectObject(self.dc, self.old);
                let _ = DeleteObject(self.bmp.into());
                let _ = DeleteDC(self.dc);
            }
        }
        self.dc = HDC::default();
        self.bits = std::ptr::null_mut();
        self.w = 0;
        self.h = 0;
    }

    /// Görüntüyü pencereye basar; pencere konumu ve boyutu da aynı çağrıda ayarlanır.
    pub fn present(&mut self, hwnd: HWND, img: &Img, x: i32, y: i32, alpha: u8) {
        self.ensure(img.w, img.h);
        unsafe {
            std::ptr::copy_nonoverlapping(img.px.as_ptr() as *const u8, self.bits, img.w * img.h * 4);
            let pt = POINT { x, y };
            let size = SIZE { cx: img.w as i32, cy: img.h as i32 };
            let src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: alpha, AlphaFormat: AC_SRC_ALPHA as u8 };
            let _ = UpdateLayeredWindow(hwnd, None, Some(&pt), Some(&size), Some(self.dc), Some(&src), COLORREF(0), Some(&blend), ULW_ALPHA);
        }
    }
}

impl Drop for Surface { fn drop(&mut self) { self.free(); } }

/// İmlecin ekrandaki yeri (fiziksel piksel).
pub fn cursor() -> (i32, i32) {
    let mut p = POINT::default();
    unsafe { let _ = GetCursorPos(&mut p); }
    (p.x, p.y)
}

/// Noktayı içeren ekranın çalışma alanı (görev çubuğu hariç).
pub fn work_area_at(x: i32, y: i32) -> RECT {
    unsafe {
        let m = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(m, &mut mi);
        mi.rcWork
    }
}

pub fn primary_work_area() -> RECT { work_area_at(0, 0) }

/// Pencerenin ekrandaki dikdörtgeni (fiziksel piksel); görünmüyorsa yok.
pub fn window_rect(h: HWND) -> Option<RECT> {
    unsafe {
        if !IsWindowVisible(h).as_bool() { return None; }
        let mut r = RECT::default();
        GetWindowRect(h, &mut r).ok()?;
        (r.right > r.left && r.bottom > r.top).then_some(r)
    }
}

/// Noktayı içeren ekranın ölçeği (1.0 = 96 dpi).
pub fn scale_at(x: i32, y: i32) -> f64 {
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    unsafe {
        let m = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let (mut dx, mut dy) = (96u32, 96u32);
        if GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_err() || dx == 0 { return 1.0; }
        dx as f64 / 96.0
    }
}

/// Windows'un "Farklı kaydet" penceresi. `filter`: "Ad\0*.uzantı\0\0".
pub fn save_dialog(default_name: &str, filter: &str) -> Option<std::path::PathBuf> {
    use windows::Win32::UI::Controls::Dialogs::{GetSaveFileNameW, OFN_OVERWRITEPROMPT, OPENFILENAMEW};
    let mut buf = [0u16; 1024];
    for (i, c) in default_name.encode_utf16().enumerate().take(1000) { buf[i] = c; }
    let filt: Vec<u16> = filter.encode_utf16().collect();
    let ext: Vec<u16> = default_name.rsplit('.').next().unwrap_or("").encode_utf16().chain(std::iter::once(0)).collect();
    let mut ofn = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        lpstrFilter: windows::core::PCWSTR(filt.as_ptr()),
        lpstrFile: windows::core::PWSTR(buf.as_mut_ptr()),
        nMaxFile: buf.len() as u32,
        lpstrDefExt: windows::core::PCWSTR(ext.as_ptr()),
        Flags: OFN_OVERWRITEPROMPT,
        ..Default::default()
    };
    unsafe {
        if !GetSaveFileNameW(&mut ofn).as_bool() { return None; }
    }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(std::path::PathBuf::from(String::from_utf16_lossy(&buf[..n])))
}