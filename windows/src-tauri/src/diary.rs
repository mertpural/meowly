//! Günlük sayfası (Mac: Tonti/TontiUI.swift → DiaryModel, DiaryPage, EntryRow, HeatGrid; Tonti/Journal.swift → GitLog).
//! Arayüz sürecinde çalışır; veritabanı ve git yalnızca sayfa açıkken okunur. Sohbetler okunmaz.

use crate::cat::format_tokens;
use crate::store::{self, WorkSession};
use crate::{l10n, look, sense, settings};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

/// "1 sa 14 dk" / "45 dk" (Mac: CatController.duration, en az 1 dk).
pub fn duration(s: f64) -> String {
    let m = ((s / 60.0) as i64).max(1);
    let h = m / 60;
    if h == 0 { return l10n::f("%lld dk", &[m.to_string()]); }
    if m % 60 == 0 { l10n::f("%lld sa", &[h.to_string()]) } else { l10n::f("%lld sa %lld dk", &[h.to_string(), (m % 60).to_string()]) }
}

fn hm(t: f64) -> String { let l = look::local(t); format!("{:02}:{:02}", l.3, l.4) }

// MARK: Git (yalnızca senin attığın commit'lerin başlıkları)

#[derive(Clone)]
pub struct Commit { pub ts: f64, pub subject: String }

fn git_path() -> Option<std::path::PathBuf> {
    static G: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    G.get_or_init(|| {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path).map(|d| d.join("git.exe")).find(|p| p.is_file())
    }).clone()
}

/// git'i gizli pencerede çalıştırır; en çok 10 sn bekler (kilitli depo ya da OneDrive'ın indirmesi sayfayı takmasın).
fn git(path: &str, args: &[&str]) -> Option<String> {
    use std::io::Read;
    let mut child = std::process::Command::new(git_path()?)
        .arg("-C").arg(path).args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn().ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || { let mut b = Vec::new(); let _ = out.by_ref().take(8 << 20).read_to_end(&mut b); b });
    let t0 = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => { let b = reader.join().unwrap_or_default(); return st.success().then(|| String::from_utf8_lossy(&b).into_owned()); }
            Ok(None) if t0.elapsed().as_secs() < 10 => std::thread::sleep(std::time::Duration::from_millis(20)),
            _ => { let _ = child.kill(); let _ = child.wait(); return None; }
        }
    }
}

fn enabled() -> bool { settings::get().journal_commits }

/// Bir deponun aralıktaki commit'leri (eskiden yeniye). Biten dönemler önbellekte tutulur.
pub fn commits(path: &str, start: f64, end: f64) -> Vec<Commit> {
    static CACHE: OnceLock<Mutex<HashMap<String, Vec<Commit>>>> = OnceLock::new();
    static EMAIL: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    if !enabled() || git_path().is_none() || !Path::new(path).join(".git").exists() { return vec![]; }
    let key = format!("{path}|{}|{}", start as i64, end as i64);
    let finished = end <= sense::now_secs();
    let cache = CACHE.get_or_init(Default::default);
    if finished { if let Some(c) = cache.lock().unwrap().get(&key) { return c.clone(); } }
    let emails = EMAIL.get_or_init(Default::default);
    // Kilit ayrı satırda bırakılır: aynı kilidi ikinci kez almak süreci kilitliyordu.
    let known = emails.lock().unwrap().get(path).cloned();
    let email = match known {
        Some(e) => e,
        None => {
            let e = git(path, &["config", "user.email"]).map(|s| s.trim().to_string()).unwrap_or_default();
            emails.lock().unwrap().insert(path.to_string(), e.clone());
            e
        }
    };
    let since = format!("--since=@{}", start as i64);
    let until = format!("--until=@{}", end as i64);
    let author = format!("--author={email}");
    let mut args = vec!["log", "--all", "--no-merges", "--format=%ct%x09%s", since.as_str(), until.as_str()];
    if !email.is_empty() { args.push(author.as_str()); }
    let mut list: Vec<Commit> = git(path, &args).unwrap_or_default().lines().filter_map(|l| {
        let (t, s) = l.split_once('\t')?;
        Some(Commit { ts: t.parse().ok()?, subject: s.to_string() })
    }).collect();
    list.sort_by(|a, b| a.ts.partial_cmp(&b.ts).unwrap());
    if finished { cache.lock().unwrap().insert(key, list.clone()); }
    list
}

/// Oturum klasörleri ve onların bir üst klasöründeki kardeş git depoları (en çok 60).
pub fn nearby_repos() -> Vec<String> {
    let home = std::env::var("USERPROFILE").unwrap_or_default().to_lowercase();
    let roots = store::recent_roots(sense::now_secs() - 90.0 * 86400.0);
    let mut set: HashSet<String> = HashSet::new();
    for r in &roots { if Path::new(r).join(".git").exists() { set.insert(r.clone()); } }
    let parents: HashSet<String> = roots.iter().filter_map(|r| Path::new(r).parent().map(|p| p.to_string_lossy().into_owned())).collect();
    for parent in parents {
        if parent.to_lowercase().len() <= home.len() { continue; }
        let Ok(rd) = std::fs::read_dir(&parent) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') { continue; }
            if e.path().join(".git").exists() { set.insert(e.path().to_string_lossy().into_owned()); }
        }
    }
    let mut v: Vec<String> = set.into_iter().collect();
    v.sort();
    v.truncate(60);
    v
}

/// Oturuma düşen commit'ler: başlangıçtan 10 dk önce ile bitişten 30 dk sonrası arası.
pub fn commits_for(s: &WorkSession, day: &[Commit]) -> Vec<Commit> {
    let (a, b) = (s.start - 600.0, s.end + 1800.0);
    day.iter().filter(|c| c.ts >= a && c.ts <= b).cloned().collect()
}

fn same_path(a: &str, b: &str) -> bool { a.replace('/', "\\").to_lowercase() == b.replace('/', "\\").to_lowercase() }

// MARK: Sayfa verisi

#[tauri::command]
pub async fn diary_page(day: f64) -> Value {
    let d = store::start_of_day(day + 3600.0); // yerel gece yarısı
    let end = d + 86400.0;
    let ss: Vec<WorkSession> = store::sessions(d, end).into_iter().filter(|s| s.duration() >= 60.0 || s.tokens > 0).collect();
    let mut by_path: BTreeMap<String, Vec<Commit>> = BTreeMap::new();
    for p in ss.iter().filter_map(|s| s.path.clone()) {
        if !by_path.contains_key(&p) { let c = commits(&p, d, end); by_path.insert(p, c); }
    }
    // Hiç token'ı ve commit'i olmayan oturumlar gösterilmez; en yeni üstte.
    let entries: Vec<(WorkSession, Vec<Commit>)> = ss.iter().rev().map(|s| {
        let c = s.path.as_ref().map(|p| commits_for(s, by_path.get(p).map(|v| v.as_slice()).unwrap_or(&[]))).unwrap_or_default();
        (s.clone(), c)
    }).filter(|(s, c)| s.tokens > 0 || !c.is_empty()).collect();

    let known: Vec<&String> = by_path.keys().collect();
    let others: Vec<(String, Vec<Commit>)> = nearby_repos().into_iter()
        .filter(|p| !known.iter().any(|k| same_path(k, p)))
        .filter_map(|p| {
            let c = commits(&p, d, end);
            (!c.is_empty()).then(|| (Path::new(&p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), c))
        }).collect();

    // Son 15 hafta (105 gün), eskiden yeniye.
    let today = store::start_of_day(sense::now_secs());
    let first = today - 104.0 * 86400.0;
    let mut daily: HashMap<String, i64> = HashMap::new();
    for (b, _p, _m, t) in store::series(first, false) { *daily.entry(b).or_default() += t; }
    let heat: Vec<Value> = (0..105).map(|i| {
        let t = first + i as f64 * 86400.0 + 3600.0;
        let l = look::local(t);
        let k = format!("{:04}-{:02}-{:02}", l.0, l.1, l.2);
        json!({ "k": k, "t": daily.get(&k).copied().unwrap_or(0), "wd": l.5, "ts": store::start_of_day(t), "tip": format!("{} token", format_tokens(daily.get(&k).copied().unwrap_or(0))) })
    }).collect();

    // Bu hafta (son 7 gün) en çok çalışılan 5 proje.
    let mut wp: HashMap<String, f64> = HashMap::new();
    for s in store::sessions(today - 6.0 * 86400.0, 1e11) { if !s.project.is_empty() { *wp.entry(s.project.clone()).or_default() += s.duration(); } }
    let mut week: Vec<(String, f64)> = wp.into_iter().collect();
    week.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    week.truncate(5);
    let wmax = week.first().map(|w| w.1).unwrap_or(1.0);

    let commit_count: usize = entries.iter().map(|e| e.1.len()).sum::<usize>() + others.iter().map(|o| o.1.len()).sum::<usize>();
    let s = settings::get();
    json!({
        "dark": s.dark,
        "en": l10n::english(),
        "mode": s.diary_mode,
        "day": d,
        "isToday": (d - today).abs() < 1.0,
        "entries": entries.iter().map(|(s, c)| { let mut provs = s.providers.clone(); provs.sort(); json!({
            "start": hm(s.start), "dur": duration(s.duration()),
            "project": if s.project.is_empty() { l10n::t("Projesiz") } else { s.project.clone() },
            "providers": provs,
            "range": format!("{}–{}", hm(s.start), hm(s.end)),
            "commits": c.iter().map(|c| c.subject.clone()).collect::<Vec<_>>(),
            "tokens": format!("{} token", format_tokens(s.tokens)),
        }) }).collect::<Vec<_>>(),
        "others": others.iter().map(|(n, c)| json!({
            "name": n, "count": l10n::f("%lld commit", &[c.len().to_string()]),
            "commits": c.iter().rev().take(6).map(|c| json!({ "time": hm(c.ts), "subject": c.subject })).collect::<Vec<_>>(),
            "more": (c.len() > 6).then(|| l10n::f("+%lld commit daha", &[(c.len() - 6).to_string()])),
        })).collect::<Vec<_>>(),
        "stats": {
            "work": duration(entries.iter().map(|e| e.0.duration()).sum()),
            "sessions": entries.len(),
            "commits": commit_count,
            "tokens": format_tokens(entries.iter().map(|e| e.0.tokens).sum()),
        },
        "heat": heat,
        "week": week.iter().map(|(n, t)| json!({ "name": n, "dur": duration(*t), "frac": t / wmax })).collect::<Vec<_>>(),
        "pc": crate::computer::day(d, d + 86400.0),
        // Mac: PermissionNudge — açık ama izni eksik katman (Windows'ta yalnızca takvim).
        "nudge": (!s.log_nudge_hidden && s.calendar_on && !crate::calendar::access_ok()).then(|| l10n::f("İzin gerekenler: %@. Ne aldığını ve ne almadığını ayarlarda tek tek anlattım.", &[l10n::t("takvim")])),
    })
}

#[tauri::command]
pub fn set_diary_mode(mode: String) {
    settings::update(|s| s.diary_mode = mode);
    crate::ipc::post_main(crate::ipc::WM_IPC_SETTINGS, 0);
}
