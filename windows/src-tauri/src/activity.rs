//! Bilgisayar günlüğü (Mac: Tonti/Activity.swift → ComputerLog, AppCategory, Browser, FileWatcher).
//! Ana süreçte ayrı bir iş parçacığında çalışır: ön plan değişimi Windows olayıyla gelir, ayrıca 30 sn'de bir
//! başlık ve boşta durumu bakılır. Ekran görüntüsü, tuşlar, sayfa içeriği, tam adres ve dosya içeriği asla alınmaz.

use crate::{sense, settings, store};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub enum Msg { Foreground(isize), Pause, Resume, Settings, Stop }

fn tx() -> &'static Mutex<Option<Sender<Msg>>> {
    static T: OnceLock<Mutex<Option<Sender<Msg>>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(None))
}

pub fn send(m: Msg) { if let Some(t) = tx().lock().unwrap().as_ref() { let _ = t.send(m); } }

// MARK: Kategoriler (Mac: AppCategory; Windows'ta uygulama exe adıyla)

const BROWSERS: [&str; 7] = ["chrome.exe", "msedge.exe", "brave.exe", "firefox.exe", "opera.exe", "vivaldi.exe", "arc.exe"];

fn category(exe: &str) -> &'static str {
    let e = exe;
    let any = |l: &[&str]| l.iter().any(|x| e == *x);
    if BROWSERS.contains(&e) { return "Tarayıcı"; }
    if any(&["code.exe", "cursor.exe", "windsurf.exe", "windowsterminal.exe", "wt.exe", "powershell.exe", "pwsh.exe", "cmd.exe",
             "devenv.exe", "idea64.exe", "pycharm64.exe", "webstorm64.exe", "rider64.exe", "clion64.exe", "goland64.exe", "studio64.exe",
             "sublime_text.exe", "notepad++.exe", "mintty.exe", "alacritty.exe", "wezterm-gui.exe", "claude.exe", "docker desktop.exe",
             "github desktop.exe", "zed.exe", "warp.exe"]) { return "Geliştirme"; }
    if any(&["slack.exe", "discord.exe", "zoom.exe", "ms-teams.exe", "teams.exe", "whatsapp.exe", "telegram.exe", "outlook.exe",
             "olk.exe", "thunderbird.exe", "signal.exe", "skype.exe"]) { return "İletişim"; }
    if any(&["winword.exe", "excel.exe", "powerpnt.exe", "onenote.exe", "notion.exe", "obsidian.exe", "acrobat.exe", "acrord32.exe",
             "notepad.exe", "todo.exe", "evernote.exe"]) { return "Verimlilik"; }
    if any(&["photoshop.exe", "figma.exe", "illustrator.exe", "adobe premiere pro.exe", "afterfx.exe", "blender.exe", "spotify.exe",
             "vlc.exe", "obs64.exe", "capcut.exe", "gimp-2.10.exe", "davinci resolve.exe", "photos.exe"]) { return "Tasarım ve medya"; }
    if any(&["robloxplayerbeta.exe", "steam.exe", "epicgameslauncher.exe", "javaw.exe", "netflix.exe"]) { return "Eğlence"; }
    if any(&["explorer.exe", "taskmgr.exe", "systemsettings.exe", "mmc.exe", "control.exe", "7zfm.exe", "everything.exe", "snippingtool.exe"]) { return "Araçlar"; }
    "Diğer"
}

/// Bu uygulamalarda pencere başlığı hiç saklanmaz.
fn title_blocked(exe: &str) -> bool {
    ["1password", "bitwarden", "keepass", "lastpass", "dashlane", "enpass", "credentialuibroker"].iter().any(|x| exe.contains(x))
}

/// Uygulama sayılmayan sistem süreçleri (kilit ekranı, başlat menüsü, arama, bildirimler…).
fn ignored(exe: &str, class: &str) -> bool {
    matches!(exe, "lockapp.exe" | "shellexperiencehost.exe" | "startmenuexperiencehost.exe" | "searchhost.exe" | "searchapp.exe"
        | "textinputhost.exe" | "logonui.exe" | "consent.exe" | "meowly.exe" | "msedgewebview2.exe")
        || (exe == "explorer.exe" && matches!(class, "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "Progman" | "WorkerW" | "NotifyIconOverflowWindow"))
}

// MARK: Pencere bilgisi

fn class_of(h: HWND) -> String {
    let mut b = [0u16; 128];
    let n = unsafe { GetClassNameW(h, &mut b) };
    String::from_utf16_lossy(&b[..n.max(0) as usize])
}

fn title_of(h: HWND) -> String {
    let mut b = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, &mut b) };
    String::from_utf16_lossy(&b[..n.max(0) as usize])
}

fn exe_of_pid(pid: u32) -> Option<PathBuf> {
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut n).is_ok();
        let _ = CloseHandle(h);
        ok.then(|| PathBuf::from(String::from_utf16_lossy(&buf[..n as usize])))
    }
}

/// Ön plandaki pencerenin süreci. UWP uygulamalarında (ApplicationFrameHost) içteki asıl süreç bulunur.
fn process_of(h: HWND) -> Option<PathBuf> {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)); }
    let exe = exe_of_pid(pid)?;
    if exe.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("ApplicationFrameHost.exe")).unwrap_or(false) {
        struct Ctx { host: u32, found: u32 }
        extern "system" fn cb(c: HWND, lp: LPARAM) -> windows::core::BOOL {
            let ctx = unsafe { &mut *(lp.0 as *mut Ctx) };
            let mut p = 0u32;
            unsafe { GetWindowThreadProcessId(c, Some(&mut p)); }
            if p != ctx.host && p != 0 { ctx.found = p; return windows::core::BOOL(0); }
            windows::core::BOOL(1)
        }
        let mut ctx = Ctx { host: pid, found: 0 };
        unsafe { let _ = EnumChildWindows(Some(h), Some(cb), LPARAM(&mut ctx as *mut Ctx as isize)); }
        if ctx.found != 0 { return exe_of_pid(ctx.found).or(Some(exe)); }
    }
    Some(exe)
}

/// Uygulamanın görünen adı (exe'nin "dosya açıklaması"; yoksa dosya adı).
fn display_name(path: &Path) -> String {
    static C: OnceLock<Mutex<HashMap<PathBuf, String>>> = OnceLock::new();
    let c = C.get_or_init(Default::default);
    if let Some(n) = c.lock().unwrap().get(path) { return n.clone(); }
    let name = file_description(path).filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
    c.lock().unwrap().insert(path.to_path_buf(), name.clone());
    name
}

fn file_description(path: &Path) -> Option<String> {
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    let w: Vec<u16> = path.as_os_str().to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let size = GetFileVersionInfoSizeW(windows::core::PCWSTR(w.as_ptr()), None);
        if size == 0 { return None; }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(windows::core::PCWSTR(w.as_ptr()), None, size, data.as_mut_ptr() as *mut _).ok()?;
        // Dil/kod sayfası tablosu
        let mut p: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let q: Vec<u16> = "\\VarFileInfo\\Translation".encode_utf16().chain(std::iter::once(0)).collect();
        if !VerQueryValueW(data.as_ptr() as *const _, windows::core::PCWSTR(q.as_ptr()), &mut p, &mut len).as_bool() || len < 4 { return None; }
        let lang = *(p as *const u16);
        let cp = *((p as *const u16).add(1));
        let key = format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\FileDescription");
        let k: Vec<u16> = key.encode_utf16().chain(std::iter::once(0)).collect();
        let mut sp: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut sl = 0u32;
        if !VerQueryValueW(data.as_ptr() as *const _, windows::core::PCWSTR(k.as_ptr()), &mut sp, &mut sl).as_bool() || sl == 0 { return None; }
        let s = std::slice::from_raw_parts(sp as *const u16, sl as usize);
        Some(String::from_utf16_lossy(s).trim_end_matches('\0').to_string())
    }
}

// MARK: Tarayıcı alan adı (UI Automation ile adres çubuğu; yalnızca alan adı saklanır)

fn private_window(title: &str) -> bool {
    let t = title.to_lowercase();
    ["incognito", "inprivate", "gizli", "özel pencere", "private browsing"].iter().any(|x| t.contains(x))
}

fn host_of(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() || s.contains(' ') { return None; }
    let rest = if let Some(i) = s.find("://") {
        let scheme = &s[..i];
        if scheme != "http" && scheme != "https" { return None; }
        &s[i + 3..]
    } else { s };
    let host = rest.split(['/', '?', '#']).next()?.split('@').last()?.split(':').next()?.to_lowercase();
    if !host.contains('.') || host.starts_with('.') { return None; }
    Some(host.strip_prefix("www.").unwrap_or(&host).to_string())
}

fn browser_domain(h: HWND, exe: &str, title: &str) -> Option<String> {
    if exe == "firefox.exe" || private_window(title) { return None; }
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Accessibility::*;
    unsafe {
        let ua: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let root = ua.ElementFromHandle(h).ok()?;
        let cond = ua.CreatePropertyCondition(UIA_ControlTypePropertyId, &windows::Win32::System::Variant::VARIANT::from(UIA_EditControlTypeId.0)).ok()?;
        let edit = root.FindFirst(TreeScope_Descendants, &cond).ok()?;
        let vp: IUIAutomationValuePattern = edit.GetCurrentPatternAs(UIA_ValuePatternId).ok()?;
        let v = vp.CurrentValue().ok()?;
        host_of(&v.to_string())
    }
}

// MARK: Kayıt motoru

struct Current { id: i64, start: f64, exe: String, title: Option<String>, domain: Option<String> }

struct Recorder { current: Option<Current>, paused: bool, files: Option<FileCounter>, ticks: u64 }

impl Recorder {
    fn close(&mut self, at: f64) {
        let Some(c) = self.current.take() else { return };
        // 3 sn'den kısa kayıtlar 0 süre kalır (gösterilmez).
        store::set_span_end(c.id, if at - c.start < 3.0 { c.start } else { at });
    }

    fn details(h: HWND, exe: &str) -> (Option<String>, Option<String>) {
        let s = settings::get();
        let browser = BROWSERS.contains(&exe);
        let title = title_of(h);
        if browser {
            return (None, if s.log_sites { browser_domain(h, exe, &title) } else { None });
        }
        if !s.log_titles || title_blocked(exe) || title.trim().is_empty() { return (None, None); }
        (Some(title.chars().take(160).collect()), None)
    }

    fn switch_to(&mut self, h: HWND) {
        let now = sense::now_secs();
        self.close(now);
        if self.paused || !settings::get().log_apps || h.is_invalid() { return; }
        let Some(path) = process_of(h) else { return };
        let exe = path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ignored(&exe, &class_of(h)) { return; }
        let (title, domain) = Self::details(h, &exe);
        let key = path.to_string_lossy().to_lowercase();
        let id = store::insert_span(now, &key, &display_name(&path), title.as_deref(), domain.as_deref(), category(&exe));
        self.current = Some(Current { id, start: now, exe, title, domain });
    }

    fn tick(&mut self) {
        if let Some(f) = &self.files { f.flush(); }
        self.ticks += 1;
        if self.ticks % 120 == 0 { self.restart_files(); } // yaklaşık saatte bir yeni projeler izlenir
        if self.paused || !settings::get().log_apps { return; }
        let idle = sense::user_idle();
        if idle > 5.0 * 60.0 && !sense::media_active() {
            // Uzun süre fare/klavye yok: boşta, kayıt son hareket anında kapanır.
            self.close(sense::now_secs() - idle);
            return;
        }
        let h = unsafe { GetForegroundWindow() };
        let Some(c) = &self.current else { self.switch_to(h); return; };
        let (title, domain) = Self::details(h, &c.exe);
        let same_app = process_of(h).and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase())).map(|e| e == c.exe).unwrap_or(false);
        if !same_app || title != c.title || domain != c.domain {
            self.switch_to(h);
        } else {
            // Bitiş zamanı güncellenir (çökmede en çok 30 sn kaybolur).
            store::set_span_end(c.id, sense::now_secs());
        }
    }

    fn restart_files(&mut self) {
        if let Some(f) = self.files.take() { f.flush(); }
        if settings::get().log_files { self.files = FileCounter::start(); }
    }
}

pub fn start() {
    let (t, rx) = channel::<Msg>();
    *tx().lock().unwrap() = Some(t);
    std::thread::spawn(move || {
        unsafe { let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED); }
        let mut r = Recorder { current: None, paused: false, files: None, ticks: 0 };
        r.restart_files();
        r.switch_to(unsafe { GetForegroundWindow() });
        loop {
            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(Msg::Foreground(h)) => { if !r.paused { r.switch_to(HWND(h as *mut _)); } }
                Ok(Msg::Pause) => { r.paused = true; r.close(sense::now_secs()); }
                Ok(Msg::Resume) => { r.paused = false; r.switch_to(unsafe { GetForegroundWindow() }); }
                Ok(Msg::Settings) => {
                    let s = settings::get();
                    if !s.log_apps { r.close(sense::now_secs()); } else if r.current.is_none() { r.switch_to(unsafe { GetForegroundWindow() }); }
                    r.restart_files();
                }
                Ok(Msg::Stop) | Err(RecvTimeoutError::Disconnected) => {
                    r.close(sense::now_secs());
                    if let Some(f) = &r.files { f.flush(); }
                    break;
                }
                Err(RecvTimeoutError::Timeout) => r.tick(),
            }
        }
    });
}

// MARK: Dosya değişiklikleri (Mac: FileWatcher)

/// Proje köklerindeki değişiklikleri sayar; saat başına proje toplamı saklanır. Dosya adı ve içeriği saklanmaz.
struct FileCounter { _w: notify::RecommendedWatcher, pending: std::sync::Arc<Mutex<HashMap<String, i64>>> }

const SKIP: [&str; 10] = ["\\.git\\", "\\node_modules\\", "\\.build\\", "\\deriveddata\\", "\\build\\", "\\.next\\", "\\dist\\", "\\pods\\", "\\.venv\\", "\\target\\"];

impl FileCounter {
    fn start() -> Option<FileCounter> {
        use notify::{RecursiveMode, Watcher};
        let mut roots: Vec<String> = store::recent_roots(sense::now_secs() - 90.0 * 86400.0).into_iter().filter(|r| Path::new(r).is_dir()).collect();
        if roots.is_empty() { return None; }
        roots.sort_by(|a, b| b.len().cmp(&a.len()));
        let pending: std::sync::Arc<Mutex<HashMap<String, i64>>> = Default::default();
        let p2 = pending.clone();
        let roots_l: Vec<(String, String)> = roots.iter().map(|r| (r.to_lowercase().replace('/', "\\"), Path::new(r).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())).collect();
        let mut w = notify::recommended_watcher(move |ev: notify::Result<notify::Event>| {
            let Ok(ev) = ev else { return };
            if ev.kind.is_access() { return; }
            let mut g = p2.lock().unwrap();
            for p in ev.paths {
                let s = p.to_string_lossy().to_lowercase().replace('/', "\\") + "\\";
                if SKIP.iter().any(|x| s.contains(x)) { continue; }
                if let Some((_, name)) = roots_l.iter().find(|(r, _)| s.starts_with(&(r.clone() + "\\"))) { *g.entry(name.clone()).or_default() += 1; }
            }
        }).ok()?;
        for r in &roots { let _ = w.watch(Path::new(r), RecursiveMode::Recursive); }
        Some(FileCounter { _w: w, pending })
    }

    fn flush(&self) {
        let c: HashMap<String, i64> = std::mem::take(&mut *self.pending.lock().unwrap());
        if c.is_empty() { return; }
        let now = sense::now_secs();
        let hour = now - (now - store::start_of_day(now)) % 3600.0;
        store::add_file_counts(&c, hour);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hosts() {
        assert_eq!(super::host_of("https://www.github.com/a/b?x"), Some("github.com".into()));
        assert_eq!(super::host_of("github.com/mertpural"), Some("github.com".into()));
        assert_eq!(super::host_of("chrome://settings"), None);
        assert_eq!(super::host_of("arama kelimesi"), None);
    }
}
