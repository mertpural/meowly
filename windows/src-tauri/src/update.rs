//! Güncelleme (Mac: Updater.swift). Sitedeki küçük `latest-windows.json` dosyasına günde en fazla bir kez bakar:
//! `{version, url, sha256, notes}`. Yalnızca sürüm okunur; kullanıcıya dair hiçbir şey gönderilmez (yalnızca
//! "Meowly/<sürüm>" tarayıcı kimliği). Yeni sürüm varsa bildirim gelir; kurulum yalnızca kullanıcı onaylayınca yapılır:
//! kurulum dosyası indirilir, SHA-256'sı ve içindeki sürüm numarası doğrulanır, tutmazsa hiçbir şey değiştirilmez.

use crate::{l10n, notify, settings, store};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::PathBuf;

const FEED: &str = "https://meowly.vercel.app/latest-windows.json";
const K_LAST: &str = "updateLastCheck";
const K_AVAIL: &str = "updateAvailable";
const K_NOTIFIED: &str = "updateNotified";

pub fn current() -> &'static str { env!("CARGO_PKG_VERSION") }

fn feed_url() -> String { std::env::var("MEOWLY_UPDATE_URL").unwrap_or_else(|_| FEED.into()) }

/// "0.10.1" > "0.9" gibi sayısal karşılaştırma.
pub fn is_newer(a: &str, b: &str) -> bool {
    let p = |s: &str| s.split('.').map(|x| x.trim().parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    let (x, y) = (p(a), p(b));
    for i in 0..x.len().max(y.len()) {
        let (m, n) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
        if m != n { return m > n; }
    }
    false
}

/// Yalnızca https ya da yerel deneme sunucusu (127.0.0.1).
fn allowed(url: &str) -> bool { url.starts_with("https://") || url.starts_with("http://127.0.0.1") }

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(15))
        .user_agent(&format!("Meowly/{}", current())).build()
}

#[derive(Debug)]
enum CheckErr { Read, Connect }

/// Beslemeyi okur. Ok(Some) yeni sürüm, Ok(None) güncel.
fn fetch() -> Result<Option<Value>, CheckErr> {
    let url = feed_url();
    if !allowed(&url) { return Err(CheckErr::Read); }
    let resp = agent().get(&url).set("Cache-Control", "no-cache").call().map_err(|e| match e {
        ureq::Error::Status(..) => CheckErr::Read,
        _ => CheckErr::Connect,
    })?;
    let mut body = String::new();
    resp.into_reader().take(64 << 10).read_to_string(&mut body).map_err(|_| CheckErr::Read)?;
    let o: Value = serde_json::from_str(&body).map_err(|_| CheckErr::Read)?;
    let (Some(v), Some(u), Some(sha)) = (o["version"].as_str(), o["url"].as_str(), o["sha256"].as_str()) else { return Err(CheckErr::Read) };
    if !allowed(u) || sha.len() != 64 { return Err(CheckErr::Read); }
    if !is_newer(v, current()) { return Ok(None); }
    Ok(Some(json!({ "version": v, "url": u, "sha256": sha.to_lowercase(), "notes": o["notes"].as_str().unwrap_or("") })))
}

/// Ana süreç, her veri okumasından sonra: kapalıysa hiçbir istek gitmez; açıksa en sık 20 saatte bir.
pub fn check_if_due() {
    let s = settings::get();
    if !s.update_check { return; }
    let now = crate::sense::now_secs();
    if now - store::kv_f64(K_LAST) < 20.0 * 3600.0 { return; }
    store::kv_set(K_LAST, &now.to_string());
    match fetch() {
        Ok(Some(info)) => {
            let v = info["version"].as_str().unwrap_or("").to_string();
            store::kv_set(K_AVAIL, &info.to_string());
            if s.notifications_on && store::kv_get(K_NOTIFIED).as_deref() != Some(v.as_str()) {
                store::kv_set(K_NOTIFIED, &v);
                notify::send(&format!("Meowly {}", l10n::f("%@ hazır", &[v])), &l10n::t("Ayarlar > Hakkında bölümünden güncelleyebilirsin."));
            }
        }
        Ok(None) => store::kv_set(K_AVAIL, ""),
        Err(_) => {}
    }
}

/// Kayıtlı yeni sürüm bilgisi (hâlâ bu sürümden yeniyse).
pub fn available() -> Option<Value> {
    let v: Value = serde_json::from_str(&store::kv_get(K_AVAIL)?).ok()?;
    is_newer(v["version"].as_str()?, current()).then_some(v)
}

/// Bu kopya kurulumla mı geldi (yanında Tauri'nin kaldırıcısı var mı). Yalnızca o zaman kendiliğinden güncellenir.
fn installed_dir() -> Option<PathBuf> {
    let dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    dir.join("uninstall.exe").exists().then_some(dir)
}

/// Hakkında kartı için durum.
pub fn page_state() -> Value {
    json!({ "available": available(), "installable": installed_dir().is_some() })
}

/// "Güncellemeleri denetle" düğmesi.
pub fn check_manual() -> Value {
    store::kv_set(K_LAST, &crate::sense::now_secs().to_string());
    let status = match fetch() {
        Ok(Some(info)) => {
            let v = info["version"].as_str().unwrap_or("").to_string();
            store::kv_set(K_AVAIL, &info.to_string());
            l10n::f("Yeni sürüm var: %@", &[v])
        }
        Ok(None) => { store::kv_set(K_AVAIL, ""); l10n::t("Güncelsin") }
        Err(CheckErr::Read) => l10n::t("Güncelleme bilgisi okunamadı"),
        Err(CheckErr::Connect) => l10n::t("Güncelleme sunucusuna bağlanılamadı"),
    };
    let mut st = page_state();
    st["status"] = Value::String(status);
    st
}

fn sha256_of(p: &std::path::Path) -> Option<String> {
    let mut f = std::fs::File::open(p).ok()?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 { break; }
        h.update(&buf[..n]);
    }
    Some(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Kurulum dosyasının içine yazılmış ürün adı ve sürüm (VERSIONINFO).
fn exe_info(p: &std::path::Path) -> Option<(String, String)> {
    use windows::Win32::Storage::FileSystem::*;
    let w: Vec<u16> = p.as_os_str().to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let n = GetFileVersionInfoSizeW(windows::core::PCWSTR(w.as_ptr()), None);
        if n == 0 { return None; }
        let mut buf = vec![0u8; n as usize];
        GetFileVersionInfoW(windows::core::PCWSTR(w.as_ptr()), None, n, buf.as_mut_ptr() as *mut _).ok()?;
        // İlk dil/kod sayfası çiftini bul.
        let mut tp: *mut core::ffi::c_void = std::ptr::null_mut();
        let mut tl = 0u32;
        if !VerQueryValueW(buf.as_ptr() as *const _, windows::core::w!("\\VarFileInfo\\Translation"), &mut tp, &mut tl).as_bool() || tl < 4 { return None; }
        let (lang, cp) = (*(tp as *const u16), *(tp as *const u16).add(1));
        let get = |key: &str| -> Option<String> {
            let q: Vec<u16> = format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\{key}").encode_utf16().chain(std::iter::once(0)).collect();
            let mut vp: *mut core::ffi::c_void = std::ptr::null_mut();
            let mut vl = 0u32;
            if !VerQueryValueW(buf.as_ptr() as *const _, windows::core::PCWSTR(q.as_ptr()), &mut vp, &mut vl).as_bool() || vl == 0 { return None; }
            let s = std::slice::from_raw_parts(vp as *const u16, vl as usize);
            Some(String::from_utf16_lossy(s).trim_end_matches('\0').to_string())
        };
        Some((get("ProductName")?, get("ProductVersion")?))
    }
}

/// "… sürümüne güncelle": indirir, doğrular, kurulumu pasif kipte başlatır ve uygulamayı kapatır.
/// Başarılıysa None (uygulama kapanıyor), değilse kullanıcıya gösterilecek durum.
pub fn install() -> String {
    let Some(info) = available() else { return l10n::t("Güncelsin") };
    if installed_dir().is_none() { return l10n::t("Uygulama bu konumda güncellenemiyor. Yeni sürümü siteden indir."); }
    let (Some(url), Some(want), Some(ver)) = (info["url"].as_str(), info["sha256"].as_str(), info["version"].as_str()) else {
        return l10n::t("Güncelleme bilgisi okunamadı");
    };
    if !allowed(url) { return l10n::t("Güncelleme bilgisi okunamadı"); }
    // Önceki güncellemeden kalan kurulum dosyaları (çalışırken silinemedikleri için) şimdi silinir.
    if let Ok(rd) = std::fs::read_dir(std::env::temp_dir()) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with("meowly-update-") { let _ = std::fs::remove_dir_all(e.path()); }
        }
    }
    let dir = std::env::temp_dir().join(format!("meowly-update-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join(format!("Meowly-Setup-{ver}.exe"));
    let ok = (|| -> Option<()> {
        let resp = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(300))
            .user_agent(&format!("Meowly/{}", current())).build().get(url).call().ok()?;
        let mut out = std::fs::File::create(&file).ok()?;
        std::io::copy(&mut resp.into_reader().take(200 << 20), &mut out).ok()?;
        Some(())
    })();
    let fail = |m: &str| { let _ = std::fs::remove_dir_all(&dir); l10n::t(m) };
    if ok.is_none() { return fail("İndirilemedi"); }
    if sha256_of(&file).as_deref() != Some(want) { return fail("İndirilen dosya doğrulanamadı (SHA-256 uyuşmuyor). Güncelleme yapılmadı."); }
    match exe_info(&file) {
        Some((name, v)) if name == "Meowly" && v == ver && is_newer(&v, current()) => {}
        _ => return fail("Sürüm bilgisi tutmuyor. Güncelleme yapılmadı."),
    }
    // Ana süreci düzgün kapat (tepsi simgesi, açık kayıt), sonra kurulumu başlat: /P pasif, /UPDATE güncelleme, /R kurulunca aç.
    crate::ipc::post_main(crate::ipc::WM_IPC_QUIT, 0);
    for _ in 0..30 {
        if !crate::ipc::main_alive() { break; }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    if std::process::Command::new(&file).args(["/P", "/UPDATE", "/R"]).spawn().is_err() {
        return fail("Güncelleme başarısız oldu");
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newer() {
        assert!(is_newer("0.10.1", "0.9"));
        assert!(is_newer("1.0.0", "0.9.0"));
        assert!(!is_newer("0.9.0", "0.9"));
        assert!(!is_newer("0.8.9", "0.9.0"));
    }
    #[test]
    fn urls() {
        assert!(allowed("https://meowly.vercel.app/x.exe"));
        assert!(allowed("http://127.0.0.1:8766/x.exe"));
        assert!(!allowed("http://example.com/x.exe"));
        assert!(!allowed("file:///c:/x.exe"));
    }
}
