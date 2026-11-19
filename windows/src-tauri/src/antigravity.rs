//! Antigravity (Mac: AntigravityProvider.swift, AntigravityQuota).
//! Token: her konuşma ~/.gemini/antigravity-ide/conversations/*.db (SQLite) içinde; gen_metadata tablosundaki protobuf
//! bloblarında üretim başına model, zaman ve token sayıları var. Alan numaraları resmi bir şemadan değil, blob içeriğinin
//! çözümlenmesinden çıkarıldı (Mac ile aynı), bu yüzden token "tahmini".
//! Limit: IDE açıkken çalışan yerel dil sunucusundan (127.0.0.1, kendinden imzalı sertifika) kota özeti. IDE kapalıysa okunmaz.

use crate::data::{min_row, Confidence, Limits, ProviderUsage, Row, Source};
use crate::{l10n, sense, store};
use serde_json::Value;
use std::path::PathBuf;

const NOTE: &str = "Token alanları (girdi, cache yazma, çıktı, cache okuma) kayıt yapısından çıkarıldı; Antigravity resmi bir şema yayınlamıyor, bu yüzden tahminidir.";

pub fn read(quota: bool) -> ProviderUsage {
    ingest();
    let now = sense::now_secs();
    let mut u = ProviderUsage::default();
    if store::latest_event("antigravity") > 0.0 {
        u.today_tokens = store::billable("antigravity", store::start_of_day(now));
        u.notes.push(l10n::t(NOTE));
    } else {
        u.notes.push(l10n::t("Yerel konuşma kayıtlarında token bulunamadı."));
    }
    if !quota { return u; }
    match quota_rows() {
        Ok(rows) => {
            let five = min_row(&rows, "5 saatlik");
            let week = min_row(&rows, "Haftalık");
            u.limits = Some(Limits {
                five_left: five.map(|x| x.0), five_reset: five.and_then(|x| x.1),
                week_left: week.map(|x| x.0), week_reset: week.and_then(|x| x.1),
                fetched_at: now, source: Source::Api,
            });
            u.extra = rows;
            u.confidence = Confidence::Exact;
            u.notes.push(l10n::t("Limit: Antigravity'nin yerel dil sunucusundan (IDE açıkken)."));
        }
        Err(e) => u.notes.push(l10n::f("Limit alınamadı: %@", &[e])),
    }
    u
}

// MARK: Protobuf (şemasız, yalnızca varint ve uzunluk-önekli alanlar)

enum Wire<'a> { Varint(u64), Bytes(&'a [u8]) }

fn pb_fields(d: &[u8]) -> Vec<(u64, Wire<'_>)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    let varint = |i: &mut usize| -> Option<u64> {
        let (mut r, mut s) = (0u64, 0u32);
        while *i < d.len() {
            let b = d[*i]; *i += 1;
            r |= ((b & 0x7f) as u64) << s;
            if b < 0x80 { return Some(r); }
            s += 7;
            if s > 63 { return None; }
        }
        None
    };
    while i < d.len() {
        let Some(key) = varint(&mut i) else { break };
        let field = key >> 3;
        match key & 7 {
            0 => { let Some(v) = varint(&mut i) else { break }; out.push((field, Wire::Varint(v))); }
            2 => {
                let Some(l) = varint(&mut i) else { break };
                let end = i.saturating_add(l as usize);
                if end > d.len() { break; }
                out.push((field, Wire::Bytes(&d[i..end])));
                i = end;
            }
            1 => { if i + 8 > d.len() { break; } i += 8; }
            5 => { if i + 4 > d.len() { break; } i += 4; }
            _ => break,
        }
    }
    out
}

fn pb_bytes<'a>(f: &[(u64, Wire<'a>)], n: u64) -> Option<&'a [u8]> {
    f.iter().find_map(|(k, w)| match w { Wire::Bytes(b) if *k == n => Some(*b), _ => None })
}
fn pb_varint(f: &[(u64, Wire<'_>)], n: u64) -> Option<i64> {
    f.iter().find_map(|(k, w)| match w { Wire::Varint(v) if *k == n => Some(*v as i64), _ => None })
}

fn conv_dir() -> PathBuf {
    PathBuf::from(std::env::var("USERPROFILE").unwrap_or_default()).join(".gemini").join("antigravity-ide").join("conversations")
}

/// Değişen veritabanlarını okur. WAL kipindeki dosya salt okunur açılamadığı için geçici bir kopya üzerinde çalışılır.
fn ingest() {
    let Ok(rd) = std::fs::read_dir(conv_dir()) else { return };
    let tmp = std::env::temp_dir().join("meowly-ag");
    let _ = std::fs::create_dir_all(&tmp);
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("db") { continue; }
        let name = e.file_name().to_string_lossy().to_string();
        // Ana dosya ve -wal'in en yenisi: konuşma sürerken yalnızca -wal değişir.
        let m = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs_f64()).unwrap_or(0.0);
        let wal = PathBuf::from(format!("{}-wal", p.display()));
        let modt = m(&p).max(m(&wal));
        let key = format!("agMod_{name}");
        if modt <= store::kv_f64(&key) { continue; }
        let copy = tmp.join(&name);
        for suf in ["", "-wal", "-shm"] {
            let src = PathBuf::from(format!("{}{suf}", p.display()));
            let dst = PathBuf::from(format!("{}{suf}", copy.display()));
            let _ = std::fs::remove_file(&dst);
            if src.exists() { let _ = std::fs::copy(&src, &dst); }
        }
        let events = read_db(&copy, &name);
        for suf in ["", "-wal", "-shm"] { let _ = std::fs::remove_file(PathBuf::from(format!("{}{suf}", copy.display()))); }
        let Some(events) = events else { continue };
        store::upsert(&events);
        store::kv_set(&key, &modt.to_string());
    }
}

fn read_db(path: &std::path::Path, name: &str) -> Option<Vec<store::Event>> {
    let c = rusqlite::Connection::open(path).ok()?;
    let mut st = c.prepare("SELECT idx, data FROM gen_metadata").ok()?;
    let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))).ok()?;
    let mut out = Vec::new();
    for (idx, blob) in rows.flatten() {
        let top = pb_fields(&blob);
        let Some(m1) = pb_bytes(&top, 1).map(pb_fields) else { continue };
        let (Some(usage), Some(timing)) = (pb_bytes(&m1, 4).map(pb_fields), pb_bytes(&m1, 9).map(pb_fields)) else { continue };
        let Some(t) = pb_bytes(&timing, 4).map(pb_fields) else { continue };
        let Some(secs) = pb_varint(&t, 1) else { continue };
        let model = pb_bytes(&m1, 19).and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("").to_string();
        out.push(store::Event {
            provider: "antigravity", key: format!("{name}|{idx}"), ts: secs as f64, model,
            input: pb_varint(&usage, 1).unwrap_or(0), output: pb_varint(&usage, 3).unwrap_or(0),
            cache_write: pb_varint(&usage, 2).unwrap_or(0), cache_read: pb_varint(&usage, 5).unwrap_or(0),
            project: String::new(), path: String::new(),
        });
    }
    Some(out)
}

// MARK: Kota (yerel dil sunucusu)

struct Server { pid: u32, csrf: String }

/// Antigravity'nin dil sunucusu süreçleri ve CSRF anahtarları (komut satırından; yalnızca bellekte, yerel sunucuya gider).
fn find_servers() -> Vec<Server> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::*;
    let mut out = Vec::new();
    unsafe {
        let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return out };
        let mut pe = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut ok = Process32FirstW(snap, &mut pe).is_ok();
        while ok {
            let n = String::from_utf16_lossy(&pe.szExeFile[..pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(0)]).to_lowercase();
            if n.starts_with("language_server") {
                if let Some(cl) = command_line(pe.th32ProcessID) {
                    let low = cl.to_lowercase();
                    if low.contains("antigravity") {
                        if let Some(csrf) = flag(&cl, "csrf_token") { out.push(Server { pid: pe.th32ProcessID, csrf }); }
                    }
                }
            }
            ok = Process32NextW(snap, &mut pe).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    out
}

fn flag(cl: &str, name: &str) -> Option<String> {
    let k = format!("--{name}");
    let i = cl.find(&k)? + k.len();
    let rest = cl[i..].trim_start_matches(|c| c == ' ' || c == '=');
    let v: String = rest.chars().take_while(|c| !c.is_whitespace() && *c != '"').collect();
    (!v.is_empty()).then_some(v)
}

/// Sürecin komut satırı (NtQueryInformationProcess, ProcessCommandLineInformation).
fn command_line(pid: u32) -> Option<String> {
    use windows::Wdk::System::Threading::{NtQueryInformationProcess, PROCESSINFOCLASS};
    use windows::Win32::Foundation::{CloseHandle, UNICODE_STRING};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u8; 32 * 1024];
        let mut len = 0u32;
        let st = NtQueryInformationProcess(h, PROCESSINFOCLASS(60), buf.as_mut_ptr() as *mut _, buf.len() as u32, &mut len);
        let _ = CloseHandle(h);
        if st.is_err() { return None; }
        let us = &*(buf.as_ptr() as *const UNICODE_STRING);
        if us.Buffer.is_null() || us.Length == 0 { return None; }
        Some(String::from_utf16_lossy(std::slice::from_raw_parts(us.Buffer.0, us.Length as usize / 2)))
    }
}

/// Sürecin dinlediği yerel TCP portları.
fn ports_of(pid: u32) -> Vec<u16> {
    use windows::Win32::NetworkManagement::IpHelper::{GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER};
    let mut out = Vec::new();
    unsafe {
        let mut size = 0u32;
        let _ = GetExtendedTcpTable(None, &mut size, false, 2, TCP_TABLE_OWNER_PID_LISTENER, 0);
        if size == 0 { return out; }
        let mut buf = vec![0u8; size as usize];
        if GetExtendedTcpTable(Some(buf.as_mut_ptr() as *mut _), &mut size, false, 2, TCP_TABLE_OWNER_PID_LISTENER, 0) != 0 { return out; }
        let t = &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
        let rows = std::slice::from_raw_parts(t.table.as_ptr() as *const MIB_TCPROW_OWNER_PID, t.dwNumEntries as usize);
        for r in rows {
            if r.dwOwningPid == pid { out.push(u16::from_be((r.dwLocalPort & 0xffff) as u16)); }
        }
    }
    out
}

/// 127.0.0.1'e HTTPS POST. Yerel sunucunun kendinden imzalı sertifikası yalnızca bu adres için kabul edilir.
fn post(port: u16, method: &str, csrf: &str, body: &str) -> Option<(u32, Vec<u8>)> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Networking::WinHttp::*;
    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    unsafe {
        let ses = WinHttpOpen(w!("Meowly"), WINHTTP_ACCESS_TYPE_NO_PROXY, PCWSTR::null(), PCWSTR::null(), 0);
        if ses.is_null() { return None; }
        let _ = WinHttpSetTimeouts(ses, 4000, 4000, 4000, 4000);
        let res = (|| {
            let con = WinHttpConnect(ses, w!("127.0.0.1"), port, 0);
            if con.is_null() { return None; }
            let path = wide(&format!("/exa.language_server_pb.LanguageServerService/{method}"));
            let req = WinHttpOpenRequest(con, w!("POST"), PCWSTR(path.as_ptr()), PCWSTR::null(), PCWSTR::null(), std::ptr::null(), WINHTTP_FLAG_SECURE);
            let out = (|| {
                if req.is_null() { return None; }
                let flags: u32 = SECURITY_FLAG_IGNORE_UNKNOWN_CA | SECURITY_FLAG_IGNORE_CERT_DATE_INVALID | SECURITY_FLAG_IGNORE_CERT_CN_INVALID | SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE;
                let fb = flags.to_ne_bytes();
                WinHttpSetOption(Some(req), WINHTTP_OPTION_SECURITY_FLAGS, Some(&fb)).ok()?;
                let headers = wide(&format!("Content-Type: application/json\r\nX-Codeium-Csrf-Token: {csrf}\r\nConnect-Protocol-Version: 1\r\n"));
                let b = body.as_bytes();
                WinHttpSendRequest(req, Some(&headers[..headers.len() - 1]), Some(b.as_ptr() as *const _), b.len() as u32, b.len() as u32, 0).ok()?;
                WinHttpReceiveResponse(req, std::ptr::null_mut()).ok()?;
                let mut code = 0u32;
                let mut sz = 4u32;
                WinHttpQueryHeaders(req, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER, PCWSTR::null(), Some(&mut code as *mut u32 as *mut _), &mut sz, std::ptr::null_mut()).ok()?;
                let mut data = Vec::new();
                loop {
                    let mut chunk = [0u8; 16384];
                    let mut n = 0u32;
                    if WinHttpReadData(req, chunk.as_mut_ptr() as *mut _, chunk.len() as u32, &mut n).is_err() || n == 0 { break; }
                    data.extend_from_slice(&chunk[..n as usize]);
                    if data.len() > 4 << 20 { break; }
                }
                Some((code, data))
            })();
            if !req.is_null() { let _ = WinHttpCloseHandle(req); }
            let _ = WinHttpCloseHandle(con);
            out
        })();
        let _ = WinHttpCloseHandle(ses);
        res
    }
}

fn quota_rows() -> Result<Vec<Row>, String> {
    let servers = find_servers();
    if servers.is_empty() { return Err(l10n::t("Antigravity IDE çalışmıyor; açınca okunur")); }
    let meta = r#"{"metadata":{"ideName":"antigravity","extensionName":"antigravity","locale":"en"}}"#;
    for s in &servers {
        for port in ports_of(s.pid) {
            match post(port, "GetUnleashData", &s.csrf, meta) { Some((200, _)) => {}, _ => continue }
            for method in ["RetrieveUserQuotaSummary", "GetUserStatus"] {
                let Some((200, data)) = post(port, method, &s.csrf, meta) else { continue };
                let Ok(obj) = serde_json::from_slice::<Value>(&data) else { continue };
                let rows = parse(&obj);
                if !rows.is_empty() { return Ok(rows); }
            }
        }
    }
    Err(l10n::t("Yerel sunucudan kota okunamadı"))
}

fn parse_date(v: &Value) -> Option<f64> {
    v.as_str().and_then(store::parse_iso).or_else(|| v.as_f64())
}

/// Mac: AntigravityQuota.parse — gruplar ve pencereler ("5 saatlik", "Haftalık") ya da model başına kota.
pub fn parse(root: &Value) -> Vec<Row> {
    let resp = if root["response"].is_object() { &root["response"] } else { root };
    let mut rows = Vec::new();
    if let Some(groups) = resp["groups"].as_array() {
        for g in groups {
            let gname = g["displayName"].as_str().unwrap_or("Grup")
                .replace(" Models", "").replace(" models", "").replace("Claude and GPT", "Claude + GPT");
            for b in g["buckets"].as_array().cloned().unwrap_or_default() {
                let Some(f) = b["remaining"]["remainingFraction"].as_f64().or_else(|| b["remainingFraction"].as_f64()) else { continue };
                let s = |v: &Value| match v { Value::String(x) => x.clone(), Value::Null => String::new(), x => x.to_string() };
                let key = format!("{} {} {}", s(&b["window"]), s(&b["bucketId"]), s(&b["displayName"])).to_lowercase();
                let win = if key.contains("week") { "Haftalık".to_string() }
                    else if key.contains("five") || key.contains('5') || key.contains("hour") { "5 saatlik".to_string() }
                    else { b["displayName"].as_str().unwrap_or("").to_string() };
                rows.push(Row { title: format!("{gname} · {win}"), left: f.clamp(0.0, 1.0), reset: parse_date(&b["resetTime"]) });
            }
        }
    }
    if rows.is_empty() {
        if let Some(cfgs) = resp["userStatus"]["cascadeModelConfigData"]["clientModelConfigs"].as_array() {
            for c in cfgs {
                let Some(f) = c["quotaInfo"]["remainingFraction"].as_f64() else { continue };
                rows.push(Row { title: c["label"].as_str().unwrap_or("Model").to_string(), left: f.clamp(0.0, 1.0), reset: parse_date(&c["quotaInfo"]["resetTime"]) });
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quota_parse() {
        let v: Value = serde_json::from_str(r#"{"response":{"groups":[{"displayName":"Claude and GPT Models","buckets":[
            {"window":"FIVE_HOUR","remaining":{"remainingFraction":0.4},"resetTime":"2026-11-18T12:00:00Z"},
            {"window":"WEEKLY","remainingFraction":0.8}]}]}}"#).unwrap();
        let r = parse(&v);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].title, "Claude + GPT · 5 saatlik");
        assert_eq!(r[1].title, "Claude + GPT · Haftalık");
        assert!(r[0].reset.is_some());
    }
    #[test]
    fn protobuf() {
        // alan 1 (bytes) → alan 4 (bytes) → alan 1 varint 150
        let inner = [0x08u8, 0x96, 0x01];
        let mut m1 = vec![0x22, inner.len() as u8]; m1.extend_from_slice(&inner);
        let mut top = vec![0x0a, m1.len() as u8]; top.extend_from_slice(&m1);
        let f = pb_fields(&top);
        let m = pb_fields(pb_bytes(&f, 1).unwrap());
        let u = pb_fields(pb_bytes(&m, 4).unwrap());
        assert_eq!(pb_varint(&u, 1), Some(150));
    }
}
