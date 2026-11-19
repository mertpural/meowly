//! Claude: token geçmişi ~/.claude/projects/**/*.jsonl (Claude Code ve Claude Desktop'ın Code sekmesi).
//! Limit yüzdeleri öncelik sırasıyla: durum satırı dosyası → Claude Code komutuyla ölçüm → Anthropic OAuth adresi (son çare).
//! Mac: ClaudeProvider.swift, CLIProbe.swift, StatusLine.swift.
//!
//! OAuth anahtarı yalnızca bellekte tutulur; diske, loga ya da ekrana asla yazılmaz ve asla yenilenmez
//! (yenilemek Claude Code'un oturumunu bozar). Süresi dolarsa Claude Code'a kısa bir istekle kendisi yeniletilir.

use crate::data::{Confidence, Limits, ProviderUsage, Source};
use crate::{l10n, sense, settings, store};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn home() -> PathBuf { std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default() }

// MARK: Proje kökü (RepoRoot)

/// Çalışma klasörünü projenin git kök klasörüne bağlar. Git yoksa klasörün kendisi. Ev klasörü proje sayılmaz.
pub fn repo_root(cwd: &str) -> Option<String> {
    static C: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    let c = C.get_or_init(Default::default);
    if let Some(r) = c.lock().unwrap().get(cwd) { return r.clone(); }
    let r = find_root(cwd);
    c.lock().unwrap().insert(cwd.to_string(), r.clone());
    r
}

fn norm(p: &Path) -> String { p.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase() }

fn find_root(cwd: &str) -> Option<String> {
    let h = norm(&home());
    if cwd.is_empty() || norm(Path::new(cwd)) == h { return None; }
    // Claude Code çalışma ağaçları: …\proje\.claude\worktrees\ad → proje
    for sep in ["\\.claude\\worktrees\\", "/.claude/worktrees/"] {
        if let Some(i) = cwd.find(sep) { return find_root(&cwd[..i]); }
    }
    let mut u = Path::new(cwd);
    loop {
        if norm(u) == h || u.parent().is_none() { break; }
        let g = u.join(".git");
        if g.is_dir() { return Some(u.to_string_lossy().into_owned()); }
        if g.is_file() {
            // git worktree: .git bir dosyadır, ana deponun yolunu gösterir.
            if let Ok(s) = std::fs::read_to_string(&g) {
                if let Some(gd) = s.trim().strip_prefix("gitdir: ") {
                    for sep in ["/.git/worktrees/", "\\.git\\worktrees\\"] {
                        if let Some(i) = gd.find(sep) { return Some(gd[..i].to_string()); }
                    }
                }
            }
            return Some(u.to_string_lossy().into_owned());
        }
        u = u.parent().unwrap();
    }
    Some(cwd.to_string())
}

pub fn project_name(root: &str) -> String {
    Path::new(root).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

// MARK: Token geçmişi

fn jsonl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() { jsonl_files(&p, out); } else if p.extension().map(|x| x == "jsonl").unwrap_or(false) { out.push(p); }
    }
}

/// Bir turda bir dosyadan okunacak en çok bayt.
pub const CHUNK: u64 = 32 << 20;

/// Her dosya kaldığı bayt konumundan okunur; yarım son satır bir sonraki tura bırakılır.
pub fn ingest() {
    let mut files = Vec::new();
    jsonl_files(&home().join(".claude").join("projects"), &mut files);
    for f in files {
        let key = f.to_string_lossy().into_owned();
        let size = std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
        let start = store::file_offset(&key);
        if size <= start { continue; }
        let Ok(mut h) = std::fs::File::open(&f) else { continue };
        if h.seek(SeekFrom::Start(start)).is_err() { continue; }
        let mut data = Vec::new();
        // Bir turda en çok 32 MB okunur (çok büyük kayıt dosyası belleği birden şişirmesin); kalanı sonraki turlarda.
        if h.by_ref().take(CHUNK).read_to_end(&mut data).is_err() || data.is_empty() { continue; }
        let Some(last_nl) = data.iter().rposition(|&b| b == b'\n') else {
            // 32 MB'tan uzun tek satır: atlanır (yoksa her turda aynı yer yeniden okunurdu).
            if data.len() as u64 >= CHUNK { store::set_file_offset(&key, start + data.len() as u64); }
            continue;
        };
        let complete = &data[..=last_nl];
        let mut events = Vec::new();
        for line in complete.split(|&b| b == b'\n') {
            if !contains(line, b"\"usage\"") { continue; }
            let Ok(v) = serde_json::from_slice::<serde_json::Value>(line) else { continue };
            if v["type"] != "assistant" { continue; }
            let msg = &v["message"];
            let usage = &msg["usage"];
            let (Some(mid), Some(ts)) = (msg["id"].as_str(), v["timestamp"].as_str().and_then(store::parse_iso)) else { continue };
            if !usage.is_object() { continue; }
            let req = v["requestId"].as_str().unwrap_or("");
            let root = v["cwd"].as_str().and_then(repo_root);
            let n = |k: &str| usage[k].as_i64().unwrap_or(0);
            events.push(store::Event {
                provider: "claude",
                key: format!("{mid}|{req}"),
                ts,
                model: msg["model"].as_str().unwrap_or("").to_string(),
                input: n("input_tokens"),
                output: n("output_tokens"),
                cache_write: n("cache_creation_input_tokens"),
                cache_read: n("cache_read_input_tokens"),
                project: root.as_deref().map(project_name).unwrap_or_default(),
                path: root.unwrap_or_default(),
            });
        }
        store::upsert(&events);
        store::set_file_offset(&key, start + complete.len() as u64);
    }
}

pub fn contains(h: &[u8], n: &[u8]) -> bool { h.windows(n.len()).any(|w| w == n) }

// MARK: Limit kaydı (son başarılı değer, uygulama kapansa da saklanır)

fn store_limits(l: &Limits) {
    if let Ok(t) = serde_json::to_string(l) { store::kv_set("claudeLastLimits", &t); }
}
fn stored() -> Option<Limits> { store::kv_get("claudeLastLimits").and_then(|t| serde_json::from_str(&t).ok()) }

// MARK: Durum satırı (Claude Code'un stdin'e verdiği JSON → claude-status.json)

pub fn status_file() -> PathBuf { settings::data_dir().join("claude-status.json") }

/// `meowly.exe --statusline`: stdin'i dosyaya yazar, ekrana hiçbir şey basmaz.
pub fn statusline_main() {
    let mut buf = Vec::new();
    if std::io::stdin().read_to_end(&mut buf).is_ok() && !buf.is_empty() {
        let f = status_file();
        let tmp = f.with_extension("json.tmp");
        if std::fs::write(&tmp, &buf).is_ok() { let _ = std::fs::rename(&tmp, &f); }
    }
}

fn reset_time(v: &serde_json::Value) -> Option<f64> {
    if let Some(n) = v.as_f64() { return Some(if n > 1e11 { n / 1000.0 } else { n }); }
    v.as_str().and_then(store::parse_iso)
}

pub fn read_status_line() -> Option<Limits> {
    let f = status_file();
    let m = std::fs::metadata(&f).and_then(|m| m.modified()).ok()?;
    let at = m.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs_f64();
    let raw = std::fs::read(&f).ok()?;
    let body = raw.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(&raw); // BOM varsa atla
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    let rl = &v["rate_limits"];
    let win = |k: &str| {
        let w = &rl[k];
        (w["used_percentage"].as_f64().map(|u| (1.0 - u / 100.0).clamp(0.0, 1.0)), reset_time(&w["resets_at"]))
    };
    let (five, week) = (win("five_hour"), win("seven_day"));
    if five.0.is_none() && week.0.is_none() { return None; }
    Some(Limits { five_left: five.0, week_left: week.0, five_reset: five.1, week_reset: week.1, fetched_at: at, source: Source::StatusLine })
}

const OUR_CMD_FLAG: &str = "--statusline";

fn settings_file() -> PathBuf { home().join(".claude").join("settings.json") }

fn our_command() -> String {
    let exe = std::env::current_exe().unwrap_or_default();
    format!("\"{}\" {}", exe.to_string_lossy(), OUR_CMD_FLAG)
}

fn is_ours(cmd: &str) -> bool { cmd.contains("meowly") && cmd.ends_with(OUR_CMD_FLAG) }

/// Claude Code ayarlarında şu an hangi durum satırı var: (bizimki mi, başkası mı).
pub fn status_line_state() -> (bool, bool) {
    let v: Option<serde_json::Value> = std::fs::read(settings_file()).ok().and_then(|b| serde_json::from_slice(&b).ok());
    match v.as_ref().and_then(|v| v["statusLine"]["command"].as_str()) {
        Some(c) if is_ours(c) => (true, false),
        Some(_) => (false, true),
        None => (false, false),
    }
}

/// Yalnızca kullanıcı ayarlardan açınca çağrılır. Önce yedek alınır; başka bir durum satırı varsa dokunulmaz.
#[allow(dead_code)]
pub fn install_status_line() -> Result<(), String> {
    let f = settings_file();
    let mut obj = serde_json::Map::new();
    if f.exists() {
        let b = std::fs::read(&f).map_err(|e| e.to_string())?;
        obj = serde_json::from_slice::<serde_json::Value>(&b).ok().and_then(|v| v.as_object().cloned())
            .ok_or_else(|| "settings.json okunamadı, dokunulmadı".to_string())?;
        let backup = f.with_file_name("settings.json.meowly-yedek");
        if !backup.exists() { std::fs::write(&backup, &b).map_err(|e| e.to_string())?; }
    }
    if let Some(c) = obj.get("statusLine").and_then(|s| s["command"].as_str()) {
        if !is_ours(c) { return Err("Başka bir durum satırı tanımlı, değiştirilmedi".into()); }
    }
    obj.insert("statusLine".into(), serde_json::json!({ "type": "command", "command": our_command(), "padding": 0 }));
    let t = serde_json::to_string_pretty(&serde_json::Value::Object(obj)).map_err(|e| e.to_string())?;
    let tmp = f.with_extension("json.meowly-tmp");
    std::fs::write(&tmp, t).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &f).map_err(|e| e.to_string())
}

/// Kaldırırken: yalnızca bu exe'yi çağıran tanım silinir (başka bir Meowly kopyasınınkine dokunulmaz).
pub fn uninstall_status_line_of_this_exe() {
    let cur = std::fs::read(settings_file()).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    if cur.as_ref().and_then(|v| v["statusLine"]["command"].as_str()) == Some(our_command().as_str()) { uninstall_status_line(); }
}

/// Yalnızca bizim eklediğimiz tanımı kaldırır.
#[allow(dead_code)]
pub fn uninstall_status_line() {
    let f = settings_file();
    let Some(mut v) = std::fs::read(&f).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()) else { return };
    let ours = v["statusLine"]["command"].as_str().map(is_ours).unwrap_or(false);
    if !ours { return; }
    if let Some(o) = v.as_object_mut() { o.remove("statusLine"); }
    if let Ok(t) = serde_json::to_string_pretty(&v) {
        let tmp = f.with_extension("json.meowly-tmp");
        if std::fs::write(&tmp, t).is_ok() { let _ = std::fs::rename(&tmp, &f); }
    }
}

// MARK: Claude Code komutu

/// `claude.exe`'nin yeri: yerel kurulum, npm, PATH, Claude uygulamasının kendi içindeki.
pub fn cli_path() -> Option<PathBuf> {
    static P: OnceLock<Option<PathBuf>> = OnceLock::new();
    P.get_or_init(|| {
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
        let mut cands = vec![
            home().join(".local").join("bin").join("claude.exe"),
            appdata.join("npm").join("node_modules").join("@anthropic-ai").join("claude-code").join("bin").join("claude.exe"),
        ];
        if let Some(path) = std::env::var_os("PATH") {
            for d in std::env::split_paths(&path) { cands.push(d.join("claude.exe")); }
        }
        if let Some(p) = cands.into_iter().find(|p| p.is_file()) { return Some(p); }
        // Yalnızca Claude uygulamasını kullananlar: uygulamanın içindeki en yeni claude.exe.
        let mut roots = vec![appdata.join("Claude").join("claude-code")];
        if let Some(la) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
            if let Ok(rd) = std::fs::read_dir(la.join("Packages")) {
                for e in rd.flatten() {
                    if e.file_name().to_string_lossy().starts_with("Claude_") {
                        roots.push(e.path().join("LocalCache").join("Roaming").join("Claude").join("claude-code"));
                    }
                }
            }
        }
        for r in roots {
            let Ok(rd) = std::fs::read_dir(&r) else { continue };
            let mut vers: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
            vers.sort();
            for v in vers.iter().rev() {
                let mut found = Vec::new();
                find_exe(v, 3, &mut found);
                if let Some(p) = found.into_iter().next() { return Some(p); }
            }
        }
        None
    }).clone()
}

fn find_exe(d: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(d) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_file() && p.file_name().map(|n| n == "claude.exe").unwrap_or(false) { out.push(p); }
        else if p.is_dir() && depth > 0 { find_exe(&p, depth - 1, out); }
    }
}

fn cli_available() -> bool { settings::get().cli_probe && cli_path().is_some() }

/// Komutu gizli pencerede çalıştırır, en çok `secs` saniye bekler; stdout'u döndürür.
pub fn run_cli(args: &[&str], secs: u64) -> Option<(bool, Vec<u8>)> {
    let cli = cli_path()?;
    let mut child = Command::new(cli)
        .args(args)
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || { let mut b = Vec::new(); let _ = out.read_to_end(&mut b); b });
    let t0 = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => return Some((st.success(), reader.join().unwrap_or_default())),
            Ok(None) if t0.elapsed().as_secs() < secs => std::thread::sleep(std::time::Duration::from_millis(250)),
            _ => { let _ = child.kill(); return None; }
        }
    }
}

static LAUNCHED: OnceLock<f64> = OnceLock::new();
pub fn mark_launch() { let _ = LAUNCHED.set(sense::now_secs()); }

/// Limiti Claude Code'un kendi komutuyla ölçer (akıştaki `rate_limit_event`). En sık 5 dakikada bir ve yalnızca
/// son değerden sonra yeni yerel Claude kullanımı olduysa. Açılışta kayıtlı değer varsa ilk 3 dakika beklenir.
fn probe_if_due() -> Option<Limits> {
    if !cli_available() { return None; }
    let last = store::kv_f64("claudeLastProbe");
    let now = sense::now_secs();
    if now - last < 300.0 { return None; }
    let st = stored();
    if st.is_some() && now - LAUNCHED.get().copied().unwrap_or(now) < 180.0 { return None; }
    let r#ref = st.as_ref().map(|s| s.fetched_at).unwrap_or(0.0).max(last);
    if st.is_some() && store::latest_event("claude") <= r#ref { return None; }
    store::kv_set("claudeLastProbe", &now.to_string());
    let (_, out) = run_cli(&["-p", ".", "--model", "haiku", "--output-format", "stream-json", "--verbose",
                             "--no-session-persistence", "--tools", "", "--strict-mcp-config", "--disable-slash-commands"], 90)?;
    let r = parse_probe(&out);
    let text = String::from_utf8_lossy(&out);
    let expired = r.is_none() && ["Failed to authenticate", "OAuth session expired", "Not logged in", "Please run /login"].iter().any(|s| text.contains(s));
    store::kv_set("claudeAuthExpired", if expired { "1" } else { "0" });
    r
}

fn parse_probe(data: &[u8]) -> Option<Limits> {
    for line in data.split(|&b| b == b'\n') {
        if !contains(line, b"rate_limit_event") { continue; }
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(line) else { continue };
        if v["type"] != "rate_limit_event" { continue; }
        let w = &v["rate_limit_info"]["unifiedWindows"];
        let win = |k: &str| (w[k]["utilization"].as_f64().map(|u| (1.0 - u).clamp(0.0, 1.0)), reset_time(&w[k]["resetsAt"]));
        let (five, week) = (win("five_hour"), win("seven_day"));
        if five.0.is_none() && week.0.is_none() { continue; }
        return Some(Limits { five_left: five.0, week_left: week.0, five_reset: five.1, week_reset: week.1, fetched_at: sense::now_secs(), source: Source::Cli });
    }
    None
}

// MARK: OAuth kullanım adresi (yalnızca Claude Code komutu yoksa)

struct Token { value: String, expires: Option<f64> }

fn token_slot() -> &'static Mutex<Option<Token>> {
    static T: OnceLock<Mutex<Option<Token>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(None))
}

/// ~/.claude/.credentials.json → claudeAiOauth.accessToken. Yalnızca bellekte tutulur.
fn load_token(force: bool) -> Result<String, String> {
    let mut slot = token_slot().lock().unwrap();
    if !force {
        if let Some(t) = slot.as_ref() {
            if t.expires.map(|e| e > sense::now_secs() + 30.0).unwrap_or(true) { return Ok(t.value.clone()); }
        }
    }
    let v: serde_json::Value = std::fs::read(home().join(".claude").join(".credentials.json")).ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or_else(|| l10n::t("Claude Code oturumu bulunamadı"))?;
    let o = &v["claudeAiOauth"];
    let tok = o["accessToken"].as_str().ok_or_else(|| l10n::t("Claude Code oturumu bulunamadı"))?.to_string();
    let expires = o["expiresAt"].as_f64().map(|ms| ms / 1000.0);
    *slot = Some(Token { value: tok.clone(), expires });
    Ok(tok)
}

/// Süresi dolmuş oturumu anahtara dokunmadan Claude Code'un kendisine yeniletir. En sık 30 dakikada bir.
fn refresh_login_via_cli() -> bool {
    if !settings::get().refresh_login { return false; }
    let now = sense::now_secs();
    if now - store::kv_f64("claudeLoginRefreshAt") < 1800.0 { return false; }
    store::kv_set("claudeLoginRefreshAt", &now.to_string());
    matches!(run_cli(&["-p", "--model", "haiku", "--no-session-persistence", "--tools", "", "--", "Reply with only: ok"], 90), Some((true, _)))
}

enum ApiErr { Msg(String), RateLimited(f64) }

fn request(retry: bool) -> Result<Limits, ApiErr> {
    let mut tok = load_token(!retry).map_err(ApiErr::Msg)?;
    let exp = token_slot().lock().unwrap().as_ref().and_then(|t| t.expires);
    if exp.map(|e| e < sense::now_secs() + 120.0).unwrap_or(false) && refresh_login_via_cli() {
        if let Ok(t) = load_token(true) { tok = t; }
    }
    let resp = ureq::get("https://api.anthropic.com/api/oauth/usage")
        .timeout(std::time::Duration::from_secs(15))
        .set("Authorization", &format!("Bearer {tok}"))
        .set("anthropic-beta", "oauth-2025-04-20")
        .set("Accept", "application/json")
        .call();
    drop(tok);
    let body = match resp {
        Ok(r) => r.into_string().map_err(|_| ApiErr::Msg(l10n::t("Ağ hatası")))?,
        Err(ureq::Error::Status(401, _)) => {
            if retry { refresh_login_via_cli(); return request(false); }
            return Err(ApiErr::Msg(l10n::t("Oturum süresi dolmuş; Claude Code'u açınca yenilenir")));
        }
        Err(ureq::Error::Status(429, r)) => {
            let ra = r.header("retry-after").and_then(|v| v.parse().ok()).unwrap_or(300.0);
            return Err(ApiErr::RateLimited(ra));
        }
        Err(ureq::Error::Status(c, _)) => return Err(ApiErr::Msg(l10n::f("Uç nokta %lld döndürdü", &[c.to_string()]))),
        Err(_) => return Err(ApiErr::Msg(l10n::t("Ağ hatası"))),
    };
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|_| ApiErr::Msg(l10n::t("Yanıtta limit alanı yok")))?;
    let win = |k: &str| v[k]["utilization"].as_f64().map(|u| ((1.0 - u / 100.0).clamp(0.0, 1.0), reset_time(&v[k]["resets_at"])));
    let (five, week) = (win("five_hour"), win("seven_day"));
    if five.is_none() && week.is_none() { return Err(ApiErr::Msg(l10n::t("Yanıtta limit alanı yok"))); }
    Ok(Limits {
        five_left: five.map(|x| x.0), week_left: week.map(|x| x.0),
        five_reset: five.and_then(|x| x.1), week_reset: week.and_then(|x| x.1),
        fetched_at: sense::now_secs(), source: Source::Api,
    })
}

fn fetch_via_api() -> Result<Limits, String> {
    if let Some(f) = read_status_line() {
        if sense::now_secs() - f.fetched_at < 900.0 { store_limits(&f); return Ok(f); }
    }
    // Claude komutu varsa Anthropic'in sert sınırlı adresine hiç gidilmez; ölçümü komut yapar, aralarda son değer.
    if cli_available() {
        return stored().ok_or_else(|| l10n::t("Limit henüz ölçülmedi"));
    }
    let now = sense::now_secs();
    if now < store::kv_f64("claudeBackoffUntil") {
        return stored().ok_or_else(|| l10n::t("Anthropic hız sınırı uyguladı; biraz sonra yeniden denenecek"));
    }
    // Yeni yerel kullanım varsa dakikada bir, yoksa 10 dakikada bir sorulur.
    let last = store::kv_f64("claudeLastAttempt");
    let min_interval = if store::latest_event("claude") > last - 5.0 { 60.0 } else { 600.0 };
    if now - last < min_interval { if let Some(s) = stored() { return Ok(s); } }
    store::kv_set("claudeLastAttempt", &now.to_string());
    match request(true) {
        Ok(l) => { store_limits(&l); Ok(l) }
        Err(e) => {
            if let ApiErr::RateLimited(ra) = &e { store::kv_set("claudeBackoffUntil", &(now + ra.max(300.0)).to_string()); }
            if let Some(s) = stored() { return Ok(s); }
            Err(match e { ApiErr::Msg(m) => m, ApiErr::RateLimited(_) => l10n::t("Anthropic hız sınırı uyguladı (429)") })
        }
    }
}

fn fetch() -> Result<Limits, String> {
    let r = fetch_via_api();
    if let Ok(l) = &r {
        let age = sense::now_secs() - l.fetched_at;
        if age < 90.0 || (l.source == Source::StatusLine && age < 900.0) { return r; }
    }
    if let Some(p) = probe_if_due() { store_limits(&p); return Ok(p); }
    r
}

/// Claude'un güncel durumu (token + limit). Ağır işler (komut, ağ) bu iş parçacığında yapılır.
pub fn read() -> ProviderUsage {
    ingest();
    let now = sense::now_secs();
    let today = store::billable("claude", store::start_of_day(now));
    let mut u = ProviderUsage { today_tokens: today, ..Default::default() };
    match fetch() {
        Ok(r) => {
            let past = |t: Option<f64>| t.map(|t| t <= now).unwrap_or(false);
            let age = now - r.fetched_at;
            let used_since = store::latest_event("claude") > r.fetched_at + 90.0;
            u.limits = Some(Limits {
                five_left: if past(r.five_reset) { Some(1.0) } else { r.five_left },
                week_left: if past(r.week_reset) { Some(1.0) } else { r.week_left },
                ..r.clone()
            });
            if age < 600.0 || !used_since { u.confidence = Confidence::Exact; } else { u.confidence = Confidence::Estimate; u.stale_since = Some(r.fetched_at); }
        }
        Err(m) => u.notes.push(m),
    }
    u.auth_expired = store::kv_get("claudeAuthExpired").as_deref() == Some("1");
    u
}
