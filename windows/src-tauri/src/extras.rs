//! Ek araçlar: OpenCode, Kimi CLI, Cline / Roo Code (Mac: ExtraProviders.swift). Yalnızca token sayısı okunur (bu araçlarda
//! kalan limit yok, kullanıcının kendi API anahtarıyla çalışırlar). Bilgisayarda izi yoksa hiç görünmezler.
//! Kayıt biçimleri Mac'teki gibi belgelerden alındı; alanlar okunamazsa o kayıt atlanır, hata vermez.

use crate::data::{Confidence, ProviderUsage};
use crate::{l10n, sense, store};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const ALL: [&str; 3] = ["opencode", "kimi", "cline"];

fn home() -> PathBuf { PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()) }

/// Kayıt klasörleri (Windows yolları).
fn roots(p: &str) -> Vec<PathBuf> {
    match p {
        "opencode" => vec![home().join(".local").join("share").join("opencode").join("storage").join("message")],
        "kimi" => vec![home().join(".kimi").join("sessions"), home().join(".kimi-code").join("sessions")],
        "cline" => {
            let appdata = PathBuf::from(std::env::var("APPDATA").unwrap_or_default());
            let mut v = Vec::new();
            for a in ["Code", "Code - Insiders", "Cursor", "Windsurf", "VSCodium"] {
                for id in ["saoudrizwan.claude-dev", "rooveterinaryinc.roo-cline"] {
                    v.push(appdata.join(a).join("User").join("globalStorage").join(id).join("tasks"));
                }
            }
            v
        }
        _ => vec![],
    }
}

/// Verisi bulunan ek araçlar (klasörü var ya da daha önce olay kaydedilmiş). 60 sn önbellekte.
pub fn present() -> Vec<&'static str> {
    static C: Mutex<Option<(f64, Vec<&'static str>)>> = Mutex::new(None);
    let now = sense::now_secs();
    if let Some((at, l)) = C.lock().unwrap().as_ref() { if now - at < 60.0 { return l.clone(); } }
    let l: Vec<&'static str> = ALL.into_iter().filter(|p| roots(p).iter().any(|r| r.exists()) || store::latest_event(p) > 0.0).collect();
    *C.lock().unwrap() = Some((now, l.clone()));
    l
}

pub fn read_present() -> Vec<(&'static str, ProviderUsage)> {
    present().into_iter().map(|p| {
        ingest(p);
        let today = store::billable(p, store::start_of_day(sense::now_secs()));
        (p, ProviderUsage {
            today_tokens: today,
            confidence: Confidence::None,
            notes: vec![l10n::t("Bu araçta kalan limit okunmuyor; yalnızca token sayılıyor.")],
            ..Default::default()
        })
    }).collect()
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if depth > 8 { return; }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() { walk(&p, out, depth + 1); } else { out.push(p); }
    }
}

fn int(v: &Value) -> i64 {
    match v { Value::Number(n) => n.as_i64().unwrap_or(0).max(0), Value::String(s) => s.parse::<i64>().unwrap_or(0).max(0), _ => 0 }
}

/// Saniye ya da milisaniye (sayı) ya da ISO metin.
fn seconds(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64().map(|x| if x > 1e11 { x / 1000.0 } else { x }),
        Value::String(s) => store::parse_iso(s),
        _ => None,
    }
}

fn mtime(p: &Path) -> f64 {
    std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

fn ingest(p: &'static str) {
    let key = format!("extraMtimes_{p}");
    let mut seen: HashMap<String, f64> = store::kv_get(&key).and_then(|v| serde_json::from_str(&v).ok()).unwrap_or_default();
    let mut files = Vec::new();
    for r in roots(p) { walk(&r, &mut files, 0); }
    let mut events = Vec::new();
    let mut changed = false;
    for f in files {
        let n = f.file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        let ok = match p { "opencode" => n.starts_with("msg_") && n.ends_with(".json"), "kimi" => n == "wire.jsonl", _ => n == "ui_messages.json" };
        if !ok { continue; }
        let k = f.to_string_lossy().to_string();
        let m = mtime(&f);
        if seen.get(&k) == Some(&m) { continue; }
        changed = true;
        let Ok(data) = std::fs::read(&f) else { continue };
        match p {
            "opencode" => events.extend(parse_opencode(&data, &n)),
            "kimi" => events.extend(parse_kimi(&data, &k, m)),
            _ => {
                let task = f.parent().and_then(|x| x.file_name()).map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                events.extend(parse_cline(&data, &task, k.contains("roo-cline")));
            }
        }
        seen.insert(k, m);
        if events.len() > 2000 { store::upsert(&events); events.clear(); }
    }
    store::upsert(&events);
    if changed { store::kv_set(&key, &serde_json::to_string(&seen).unwrap_or_default()); }
}

fn ev(p: &'static str, key: String, ts: f64, model: &str, i: i64, o: i64, cw: i64, cr: i64) -> store::Event {
    store::Event { provider: p, key, ts, model: model.to_string(), input: i, output: o, cache_write: cw, cache_read: cr, project: String::new(), path: String::new() }
}

/// OpenCode: storage/message/<oturum>/msg_<id>.json → {role, modelID, time.created (ms), tokens{input,output,reasoning,cache{read,write}}}
pub fn parse_opencode(data: &[u8], name: &str) -> Vec<store::Event> {
    let Ok(o) = serde_json::from_slice::<Value>(data) else { return vec![] };
    if o["role"].as_str() != Some("assistant") || !o["tokens"].is_object() { return vec![]; }
    let t = &o["tokens"];
    let (i, out) = (int(&t["input"]), int(&t["output"]) + int(&t["reasoning"]));
    let (cw, cr) = (int(&t["cache"]["write"]), int(&t["cache"]["read"]));
    let Some(ts) = seconds(&o["time"]["created"]) else { return vec![] };
    if i + out + cw + cr <= 0 { return vec![]; }
    vec![ev("opencode", name.to_string(), ts, o["modelID"].as_str().unwrap_or("opencode"), i, out, cw, cr)]
}

/// Kimi: wire.jsonl satırlarında bir yerde token_usage{input_other, output, input_cache_read, input_cache_creation}
pub fn parse_kimi(data: &[u8], path: &str, fallback: f64) -> Vec<store::Event> {
    fn find(v: &Value, depth: usize) -> Option<&Value> {
        if depth >= 5 { return None; }
        match v {
            Value::Object(m) => { if let Some(u) = m.get("token_usage").filter(|u| u.is_object()) { return Some(u); } m.values().find_map(|x| find(x, depth + 1)) }
            Value::Array(a) => a.iter().find_map(|x| find(x, depth + 1)),
            _ => None,
        }
    }
    let mut out = Vec::new();
    for (i, line) in data.split(|&b| b == b'\n').enumerate() {
        if !line.windows(11).any(|w| w == b"token_usage") { continue; }
        let Ok(o) = serde_json::from_slice::<Value>(line) else { continue };
        let Some(u) = find(&o, 0) else { continue };
        let (inp, outp, cr, cw) = (int(&u["input_other"]), int(&u["output"]), int(&u["input_cache_read"]), int(&u["input_cache_creation"]));
        if inp + outp + cw + cr <= 0 { continue; }
        let ts = seconds(&o["timestamp"]).unwrap_or(fallback);
        out.push(ev("kimi", format!("{path}|{i}"), ts, "kimi", inp, outp, cw, cr));
    }
    out
}

/// Cline / Roo Code: ui_messages.json dizisinde say == "api_req_started", text = JSON {tokensIn, tokensOut, cacheWrites, cacheReads}
pub fn parse_cline(data: &[u8], task: &str, roo: bool) -> Vec<store::Event> {
    let Ok(Value::Array(arr)) = serde_json::from_slice::<Value>(data) else { return vec![] };
    let mut out = Vec::new();
    for m in arr.iter().filter(|m| m["say"].as_str() == Some("api_req_started")) {
        let Some(u) = m["text"].as_str().and_then(|t| serde_json::from_str::<Value>(t).ok()) else { continue };
        let Some(ts) = seconds(&m["ts"]) else { continue };
        let (i, o, cw, cr) = (int(&u["tokensIn"]), int(&u["tokensOut"]), int(&u["cacheWrites"]), int(&u["cacheReads"]));
        if i + o + cw + cr <= 0 { continue; }
        out.push(ev("cline", format!("{task}|{}", (ts * 1000.0) as i64), ts, if roo { "roo-code" } else { "cline" }, i, o, cw, cr));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats() {
        let oc = br#"{"role":"assistant","modelID":"m","time":{"created":1763460000000},"tokens":{"input":10,"output":5,"reasoning":1,"cache":{"read":2,"write":3}}}"#;
        let e = parse_opencode(oc, "msg_1.json");
        assert_eq!((e[0].input, e[0].output, e[0].cache_write, e[0].cache_read), (10, 6, 3, 2));
        assert!((e[0].ts - 1763460000.0).abs() < 1.0);
        let k = b"{\"timestamp\":1763460000,\"payload\":{\"x\":{\"token_usage\":{\"input_other\":7,\"output\":2,\"input_cache_read\":1,\"input_cache_creation\":0}}}}\n";
        assert_eq!(parse_kimi(k, "p", 0.0)[0].input, 7);
        let c = br#"[{"say":"api_req_started","ts":1763460000000,"text":"{\"tokensIn\":4,\"tokensOut\":3,\"cacheWrites\":0,\"cacheReads\":9}"}]"#;
        assert_eq!(parse_cline(c, "t", true)[0].cache_read, 9);
    }
}
