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
/// Kurulumu başlatılan sürüm: bir sonraki açılışta gerçekten kurulmuş mu diye bakılır.
const K_PENDING: &str = "updatePending";
/// Kurulamayan sürüm (Hakkında'da uyarı).
const K_FAILED: &str = "updateFailed";

/// Güncelleme günlüğü: %APPDATA%\Meowly\update.log (en çok 64 KB). Yalnızca adımlar ve sürümler; kişisel bir şey yok.
/// Kurulum programı da aynı dosyaya yazar (installer-hooks.nsh).
pub fn log(s: impl AsRef<str>) {
    use std::io::Write;
    let p = settings::data_dir().join("update.log");
    if std::fs::metadata(&p).map(|m| m.len() > 65536).unwrap_or(false) { let _ = std::fs::remove_file(&p); }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let (y, m, d, h, mi, _) = crate::look::local(crate::sense::now_secs());
        let _ = writeln!(f, "{d:02}.{m:02}.{y} {h:02}:{mi:02} [{}] {}", current(), s.as_ref());
    }
}

/// Ana süreç açılışta: başlatılan kurulum gerçekten yapıldı mı? Yapılmadıysa Hakkında'da uyarılır.
pub fn check_pending() {
    let Some(v) = store::kv_get(K_PENDING).filter(|v| !v.is_empty()) else { return };
    store::kv_set(K_PENDING, "");
    if is_newer(&v, current()) {
        log(format!("{v} kurulamadı: uygulama hâlâ {}", current()));
        store::kv_set(K_FAILED, &v);
    } else {
        log(format!("{v} kuruldu"));
        store::kv_set(K_FAILED, "");
    }
}

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
    json!({ "available": available(), "installable": installed_dir().is_some(),
            "failed": store::kv_get(K_FAILED).filter(|v| !v.is_empty() && is_newer(v, current())) })
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
    log(format!("{} kurulumu istendi", info["version"].as_str().unwrap_or("?")));
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
    if ok.is_none() { log("indirilemedi"); return fail("İndirilemedi"); }
    log(format!("indirildi: {} bayt", std::fs::metadata(&file).map(|m| m.len()).unwrap_or(0)));
    if sha256_of(&file).as_deref() != Some(want) { log("SHA-256 tutmadı"); return fail("İndirilen dosya doğrulanamadı (SHA-256 uyuşmuyor). Güncelleme yapılmadı."); }
    match exe_info(&file) {
        Some((name, v)) if name == "Meowly" && v == ver && is_newer(&v, current()) => {}
        other => { log(format!("sürüm bilgisi tutmadı: {other:?}")); return fail("Sürüm bilgisi tutmuyor. Güncelleme yapılmadı.") }
    }
    // Kurulum, Meowly'den bağımsız küçük bir bekçiyle (cmd) başlatılır: Meowly kapanırken kurulum onunla birlikte ölmez.
    // Bekçi kurulumun bitmesini bekler; Meowly sonra hâlâ kapalıysa (kurulum yarıda kaldıysa da) uygulamayı yeniden açar.
    // Önce kurulum başlatılır, sonra ana süreç kapatılır: başlatılamazsa uygulama açık kalır.
    let Some(dir) = installed_dir() else { return fail("Uygulama bu konumda güncellenemiyor. Yeni sürümü siteden indir.") };
    store::kv_set(K_PENDING, ver);
    if let Err(e) = spawn_detached_installer(&file, &dir.join("meowly.exe"), "meowly.exe") {
        log(format!("kurulum başlatılamadı: {e}"));
        store::kv_set(K_PENDING, "");
        return fail("Güncelleme başarısız oldu");
    }
    log("kurulum başlatıldı (/P /UPDATE /R, bağımsız bekçiyle)");
    // Ana süreci düzgün kapat (tepsi simgesi, açık kayıt). Kurulum da açık kalan Meowly süreçlerini kapatır.
    crate::ipc::post_main(crate::ipc::WM_IPC_QUIT, 0);
    for _ in 0..30 {
        if !crate::ipc::main_alive() { break; }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    log(format!("ana süreç kapandı mı: {}", !crate::ipc::main_alive()));
    String::new()
}

/// Kurulumu bağımsız bir cmd sürecinde başlatır (Meowly'nin iş nesnesinden ayrılır; gizli konsolda, pencere açılmaz).
fn spawn_detached_installer(setup: &std::path::Path, exe: &std::path::Path, image: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // start /wait: kurulum bitene kadar bekle. ping: ~3 sn (kurulumun /R ile açtığı uygulama başlasın).
    // tasklist | find: Meowly çalışmıyorsa aç (kurulum başarılıysa yeni, yarıda kaldıysa eski sürüm).
    let script = format!(
        "/c start \"\" /wait \"{}\" /P /UPDATE /R & ping -n 4 127.0.0.1 >nul & tasklist /FI \"IMAGENAME eq {image}\" | find /I \"{image}\" >nul || start \"\" \"{}\"",
        setup.display(), exe.display());
    let run = |flags: u32| std::process::Command::new("cmd.exe").raw_arg(&script).creation_flags(flags)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
    // Gizli konsol: içindeki ping/tasklist de pencere açmaz (DETACHED_PROCESS her birine yeni pencere açtırıyordu).
    let base = CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW;
    // İş nesnesinden ayrılmaya izin yoksa (erişim reddi) ayrılmadan dene.
    run(base | CREATE_BREAKAWAY_FROM_JOB).or_else(|_| run(base)).map(|_| ())
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
    fn pending_check() {
        // Geçici veri klasörü (gerçek %APPDATA%'ya dokunulmaz).
        let d = std::env::temp_dir().join(format!("meowly-upd-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        std::env::set_var("APPDATA", &d);
        // Kurulumu başlatılan sürüm kurulmamış: uyarı kalır.
        store::kv_set(K_PENDING, "99.0.0");
        check_pending();
        assert_eq!(store::kv_get(K_FAILED).as_deref(), Some("99.0.0"));
        assert_eq!(page_state()["failed"], "99.0.0");
        assert_eq!(store::kv_get(K_PENDING).as_deref(), Some(""));
        // Kurulmuş (şu anki sürüm ya da daha eski): uyarı silinir.
        store::kv_set(K_PENDING, current());
        check_pending();
        assert_eq!(store::kv_get(K_FAILED).as_deref(), Some(""));
        assert!(page_state()["failed"].is_null());
        let log = std::fs::read_to_string(d.join("Meowly").join("update.log")).unwrap();
        assert!(log.contains("99.0.0 kurulamadı") && log.contains("kuruldu"), "{log}");
        let _ = std::fs::remove_dir_all(&d);
    }
    /// Bekçi: kurulumu bekler, uygulama çalışmıyorsa açar. Pencere açmayan iki .vbs ile denenir (hiçbir şey kurulmaz).
    #[test]
    #[ignore]
    fn detached_watchdog() {
        let d = std::env::temp_dir().join(format!("meowly bekci {}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        let mark = |n: &str| format!("Set f = CreateObject(\"Scripting.FileSystemObject\").CreateTextFile(\"{}\")
f.Write WScript.Arguments.Count
f.Close
WScript.Sleep 500
", d.join(n).display());
        std::fs::write(d.join("kurulum.vbs"), mark("kurulum.txt")).unwrap();
        std::fs::write(d.join("uygulama.vbs"), mark("uygulama.txt")).unwrap();
        spawn_detached_installer(&d.join("kurulum.vbs"), &d.join("uygulama.vbs"), "boyle-bir-surec-yok.exe").unwrap();
        for _ in 0..100 { if d.join("uygulama.txt").exists() { break; } std::thread::sleep(std::time::Duration::from_millis(100)); }
        // Kurulum üç argümanla (/P /UPDATE /R) çalıştı, sonra uygulama açıldı.
        assert_eq!(std::fs::read_to_string(d.join("kurulum.txt")).unwrap(), "3");
        assert!(d.join("uygulama.txt").exists());
        // Uygulama zaten çalışıyorsa (burada her zaman açık olan explorer.exe) ikinci kopya açılmaz.
        let _ = std::fs::remove_file(d.join("uygulama.txt"));
        let _ = std::fs::remove_file(d.join("kurulum.txt"));
        spawn_detached_installer(&d.join("kurulum.vbs"), &d.join("uygulama.vbs"), "explorer.exe").unwrap();
        for _ in 0..40 { if d.join("kurulum.txt").exists() { break; } std::thread::sleep(std::time::Duration::from_millis(100)); }
        std::thread::sleep(std::time::Duration::from_secs(7));
        assert!(d.join("kurulum.txt").exists());
        assert!(!d.join("uygulama.txt").exists(), "uygulama zaten açıkken yeniden açılmamalı");
        let _ = std::fs::remove_dir_all(&d);
    }
    #[test]
    fn urls() {
        assert!(allowed("https://meowly.vercel.app/x.exe"));
        assert!(allowed("http://127.0.0.1:8766/x.exe"));
        assert!(!allowed("http://example.com/x.exe"));
        assert!(!allowed("file:///c:/x.exe"));
    }
}
