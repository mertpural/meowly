//! Codex: ~/.codex/sessions/**/*.jsonl ya da .jsonl.zst (Mac: CodexProvider.swift).
//! Limit: en yeni kayıttaki payload.rate_limits (primary = 5 saatlik, secondary = haftalık).
//! Token: payload.type == "token_count" → info.last_token_usage. Değişmemiş dosyalar atlanır.

use crate::claude::{contains, project_name, repo_root};
use crate::data::{Confidence, Limits, ProviderUsage, Source};
use crate::{sense, store};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn root() -> PathBuf { std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default().join(".codex").join("sessions") }

fn files(d: &Path, out: &mut Vec<(PathBuf, f64)>) {
    let Ok(rd) = std::fs::read_dir(d) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() { files(&p, out); continue; }
        let n = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if !(n.ends_with(".jsonl") || n.ends_with(".jsonl.zst")) { continue; }
        let m = e.metadata().and_then(|m| m.modified()).ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs_f64()).unwrap_or(0.0);
        out.push((p, m));
    }
}

fn is_zst(p: &Path) -> bool { p.extension().map(|e| e == "zst").unwrap_or(false) }

/// Düz dosyanın `start` baytından itibaren en çok `cap` baytı.
fn read_range(p: &Path, start: u64, cap: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(p).ok()?;
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut out = Vec::new();
    f.take(cap).read_to_end(&mut out).ok()?;
    Some(out)
}

/// Dosyayı satır satır okur; bellekte yalnızca o anki satır durur (.zst arşivleri akış hâlinde açılır).
/// `skip` bayttan sonraki tam satırlar `f`'ye verilir (`f` false dönerse durur); yarım son satır verilmez.
/// Dönen değer: verilen son tam satırın bittiği yer (açılmış hâldeki bayt), sonraki tur buradan başlar.
fn for_each_line(p: &Path, skip: u64, mut f: impl FnMut(&[u8]) -> bool) -> Option<u64> {
    use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
    const MAX_LINE: u64 = 64 << 20;
    let mut file = std::fs::File::open(p).ok()?;
    let (mut r, mut pos): (Box<dyn BufRead>, u64) = if is_zst(p) {
        let dec = ruzstd::StreamingDecoder::new(BufReader::new(file)).ok()?;
        (Box::new(BufReader::with_capacity(1 << 16, dec)), 0)
    } else {
        file.seek(SeekFrom::Start(skip)).ok()?;
        (Box::new(BufReader::with_capacity(1 << 16, file)), skip)
    };
    let mut line = Vec::new();
    loop {
        line.clear();
        let n = r.by_ref().take(MAX_LINE).read_until(b'\n', &mut line).ok()? as u64;
        if n == 0 { return Some(pos); }
        if line.last() != Some(&b'\n') {
            // 64 MB'tan uzun satır atlanır (bozuk kayıt belleği doldurmasın); dosya sonundaki yarım satır bekletilir.
            if n >= MAX_LINE { pos += n; continue; }
            return Some(pos);
        }
        pos += n;
        if pos <= skip { continue; }
        if !f(&line[..line.len() - 1]) { return Some(pos); }
    }
}

/// Dosyanın ilk satırı (oturum bilgisi).
fn first_line(p: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    for_each_line(p, 0, |l| { out = l.to_vec(); false });
    out
}

/// Dosyadaki en yeni limit kaydı. Düz (etkin) dosyada sondan geriye 4 MB'lık parçalarla aranır (kayıt genelde sondadır);
/// arşivde satır satır. Hiçbir zaman bütün dosya belleğe alınmaz.
fn latest_in(p: &Path) -> Option<(Option<Window>, Option<Window>, f64)> {
    if !is_zst(p) {
        const CH: u64 = 4 << 20;
        let mut end = std::fs::metadata(p).ok()?.len();
        while end > 0 {
            let start = end.saturating_sub(CH);
            let buf = read_range(p, start, end - start)?;
            let cut = if start == 0 { 0 } else {
                match buf.iter().position(|&b| b == b'\n') { Some(i) => i + 1, None => { end = start; continue; } }
            };
            if let Some(r) = latest_rate_limits(&buf[cut..]) { return Some(r); }
            if start == 0 { return None; }
            let next = start + cut as u64;
            end = if next >= end { start } else { next };
        }
        return None;
    }
    let mut best: Option<(Option<Window>, Option<Window>, f64)> = None;
    for_each_line(p, 0, |line| {
        if contains(line, b"rate_limits") {
            if let Some(r) = latest_rate_limits(line) {
                if best.as_ref().map(|b| r.2 > b.2).unwrap_or(true) { best = Some(r); }
            }
        }
        true
    });
    best
}

struct Window { used: f64, resets: f64 }

fn window(v: &serde_json::Value) -> Option<Window> {
    Some(Window { used: v["used_percent"].as_f64()?, resets: v["resets_at"].as_f64()? })
}

fn latest_rate_limits(data: &[u8]) -> Option<(Option<Window>, Option<Window>, f64)> {
    let mut best: Option<(Option<Window>, Option<Window>, f64)> = None;
    for line in data.split(|&b| b == b'\n') {
        if !contains(line, b"rate_limits") { continue; }
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(line) else { continue };
        let rl = &v["payload"]["rate_limits"];
        if !rl.is_object() { continue; }
        let Some(ts) = v["timestamp"].as_str().and_then(store::parse_iso) else { continue };
        if best.as_ref().map(|b| ts > b.2).unwrap_or(true) { best = Some((window(&rl["primary"]), window(&rl["secondary"]), ts)); }
    }
    best
}

fn mtimes() -> HashMap<String, f64> {
    store::kv_get("codexMtimes").and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn ingest(all: &[(PathBuf, f64)]) {
    let mut mt = mtimes();
    let mut changed = false;
    for (p, mod_t) in all {
        let key = p.to_string_lossy().into_owned();
        if mt.get(&key) == Some(mod_t) { continue; }
        let off = store::file_offset(&key);
        // Oturumun çalışma klasörü ilk satırdaki session_meta'da.
        let first = first_line(p);
        let cwd = serde_json::from_slice::<serde_json::Value>(&first).ok()
            .and_then(|v| v["payload"]["cwd"].as_str().map(String::from));
        let root = cwd.as_deref().and_then(repo_root);
        let project = root.as_deref().map(project_name).unwrap_or_default();
        let path = root.clone().unwrap_or_default();
        let mut prev_total = store::file_total(&key).unwrap_or(0);
        let fname = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mut model = "codex".to_string();
        let mut events = Vec::new();
        // Satır satır okunur; olaylar 2000'lik gruplar hâlinde yazılır (bellek sınırlı kalsın).
        let end = for_each_line(p, off, |line| {
            let Ok(v) = serde_json::from_slice::<serde_json::Value>(line) else { return true };
            let pl = &v["payload"];
            if !pl.is_object() { return true; }
            if v["type"] == "turn_context" { if let Some(m) = pl["model"].as_str() { model = m.to_string(); } return true; }
            if pl["type"] != "token_count" { return true; }
            let info = &pl["info"];
            let last = &info["last_token_usage"];
            let (Some(total), Some(ts), Some(ord)) = (info["total_token_usage"]["total_tokens"].as_i64(),
                v["timestamp"].as_str().and_then(store::parse_iso), v["ordinal"].as_i64()) else { return true };
            if !last.is_object() { return true; }
            // Aynı toplamı taşıyan tekrar olaylar çift sayılmasın.
            if total == prev_total { return true; }
            prev_total = total;
            let input = last["input_tokens"].as_i64().unwrap_or(0);
            let cached = last["cached_input_tokens"].as_i64().unwrap_or(0);
            events.push(store::Event {
                provider: "codex", key: format!("{fname}|{ord}"), ts, model: model.clone(),
                input: (input - cached).max(0), output: last["output_tokens"].as_i64().unwrap_or(0),
                cache_write: 0, cache_read: cached,
                project: project.clone(), path: path.clone(),
            });
            if events.len() >= 2000 { store::upsert(&events); events.clear(); }
            true
        });
        store::upsert(&events);
        store::set_file_total(&key, prev_total);
        if let Some(e) = end { if e > off { store::set_file_offset(&key, e); } }
        mt.insert(key, *mod_t);
        changed = true;
    }
    // Değişiklik yoksa yazılmaz (gereksiz disk yazımı olmasın).
    if changed { if let Ok(t) = serde_json::to_string(&mt) { store::kv_set("codexMtimes", &t); } }
}

/// En yeni dosyadan geriye, ilk limit kaydı. Sonuç en yeni dosyanın değişim zamanına göre önbellekte.
fn latest_limits(all: &[(PathBuf, f64)]) -> Option<(Option<Window>, Option<Window>, f64)> {
    static C: OnceLock<Mutex<Option<(f64, Option<(f64, f64, f64, f64, f64)>)>>> = OnceLock::new();
    let c = C.get_or_init(|| Mutex::new(None));
    let mut sorted: Vec<&(PathBuf, f64)> = all.iter().collect();
    sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let newest = sorted.first().map(|f| f.1).unwrap_or(0.0);
    let pack = |w: &Option<Window>| w.as_ref().map(|w| (w.used, w.resets));
    if let Some((m, cached)) = c.lock().unwrap().as_ref() {
        if *m == newest {
            return cached.map(|(pu, pr, su, sr, ts)| (
                (pu >= 0.0).then_some(Window { used: pu, resets: pr }), (su >= 0.0).then_some(Window { used: su, resets: sr }), ts));
        }
    }
    let mut found = None;
    for (p, _) in sorted {
        if let Some(r) = latest_in(p) { found = Some(r); break; }
    }
    let packed = found.as_ref().map(|(a, b, ts)| {
        let (pu, pr) = pack(a).unwrap_or((-1.0, 0.0));
        let (su, sr) = pack(b).unwrap_or((-1.0, 0.0));
        (pu, pr, su, sr, *ts)
    });
    *c.lock().unwrap() = Some((newest, packed));
    found
}

pub fn read() -> ProviderUsage {
    let mut all = Vec::new();
    files(&root(), &mut all);
    ingest(&all);
    let now = sense::now_secs();
    let mut u = ProviderUsage { today_tokens: store::billable("codex", store::start_of_day(now)), ..Default::default() };
    let Some((p, s, stamp)) = latest_limits(&all) else {
        u.notes.push(crate::l10n::t("Codex kayıtlarında limit bilgisi bulunamadı."));
        return u;
    };
    // Pencere sıfırlandıysa (resets_at geçmişte) kullanım 0 sayılır.
    let left = |w: &Option<Window>| w.as_ref().map(|w| if w.resets <= now { 1.0 } else { (1.0 - w.used / 100.0).clamp(0.0, 1.0) });
    u.limits = Some(Limits {
        five_left: left(&p), week_left: left(&s),
        five_reset: p.as_ref().map(|w| w.resets), week_reset: s.as_ref().map(|w| w.resets),
        fetched_at: stamp, source: Source::Codex,
    });
    u.confidence = Confidence::Exact;
    u
}
