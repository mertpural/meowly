//! Gemini (Mac: GeminiProvider.swift). Gemini CLI'ın Google oturumuyla (~/.gemini/oauth_creds.json) Code Assist kota
//! API'sinden model grubu başına kalan günlük kotayı okur. Yalnızca kullanıcı Gemini'yi açtıysa çalışır.
//! Anahtar yalnızca bellekte kullanılır, hiçbir yere yazılmaz ve günlüğe girmez. Windows sürümü anahtarı ASLA yenilemez
//! (kullanıcının kuralı): süresi dolmuşsa Gemini CLI'ın kendisi yenileyene kadar "veri yok" gösterilir.

use crate::data::{Confidence, ProviderUsage, Row};
use crate::{l10n, sense};
use serde_json::{json, Value};
use std::io::Read;
use std::sync::Mutex;

const BASE: &str = "https://cloudcode-pa.googleapis.com/v1internal";
static CACHE: Mutex<Option<(f64, ProviderUsage)>> = Mutex::new(None);

fn fail(msg: &str) -> ProviderUsage { ProviderUsage { notes: vec![l10n::f("Limit alınamadı: %@", &[msg.to_string()])], ..Default::default() } }

/// 2 dakikada en çok bir kez sorulur (Mac ile aynı).
pub fn read() -> ProviderUsage {
    let now = sense::now_secs();
    if let Some((at, u)) = CACHE.lock().unwrap().as_ref() { if now - at < 120.0 { return u.clone(); } }
    let u = fetch();
    *CACHE.lock().unwrap() = Some((now, u.clone()));
    u
}

fn access_token() -> Result<String, String> {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let path = std::path::Path::new(&home).join(".gemini").join("oauth_creds.json");
    let o: Value = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok())
        .ok_or_else(|| "Gemini CLI oturumu bulunamadı (~/.gemini/oauth_creds.json)".to_string())?;
    let exp = o["expiry_date"].as_f64().unwrap_or(0.0) / 1000.0;
    match o["access_token"].as_str() {
        Some(t) if exp > sense::now_secs() + 60.0 => Ok(t.to_string()),
        _ => Err(l10n::t("Oturum süresi dolmuş; Gemini CLI'ı bir kez çalıştırınca yenilenir")),
    }
}

fn call(method: &str, token: &str, body: &Value) -> Option<(u16, Value)> {
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(15)).build();
    let r = agent.post(&format!("{BASE}:{method}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string());
    let (code, resp) = match r {
        Ok(r) => (r.status(), r),
        Err(ureq::Error::Status(c, r)) => (c, r),
        Err(_) => return None,
    };
    let mut t = String::new();
    let _ = resp.into_reader().take(1 << 20).read_to_string(&mut t);
    Some((code, serde_json::from_str(&t).unwrap_or(Value::Null)))
}

fn parse_time(s: &str) -> Option<f64> { crate::store::parse_iso(s) }

fn fetch() -> ProviderUsage {
    let token = match access_token() { Ok(t) => t, Err(e) => return fail(&e) };
    let meta = json!({ "metadata": { "ideType": "IDE_UNSPECIFIED", "platform": "PLATFORM_UNSPECIFIED", "pluginType": "GEMINI" } });
    let mut project: Option<String> = None;
    if let Some((200, o)) = call("loadCodeAssist", &token, &meta) {
        project = o["cloudaicompanionProject"].as_str().map(String::from).or_else(|| o["cloudaicompanionProject"]["id"].as_str().map(String::from));
    }
    let body = project.map(|p| json!({ "project": p })).unwrap_or_else(|| json!({}));
    let Some((code, o)) = call("retrieveUserQuota", &token, &body) else { return fail(&l10n::t("Ağ hatası")) };
    let buckets = o["buckets"].as_array().cloned().unwrap_or_default();
    if code != 200 || buckets.is_empty() {
        let reason = o["error"]["details"].as_array().and_then(|d| d.iter().find_map(|x| x["reason"].as_str().map(String::from)));
        if reason.as_deref() == Some("SUBSCRIPTION_REQUIRED") {
            return fail("Bu Google hesabında Gemini Code Assist lisansı yok, Google kota bilgisini vermiyor (SUBSCRIPTION_REQUIRED)");
        }
        return fail(&l10n::f("Kota yanıtı %lld döndürdü%@", &[code.to_string(), reason.map(|r| format!(" ({r})")).unwrap_or_default()]));
    }
    // Model gruplarına göre en düşük kalan oran (Pro, Flash, Flash Lite).
    let mut groups: Vec<(String, f64, Option<f64>)> = Vec::new();
    for b in &buckets {
        let (Some(f), Some(id)) = (b["remainingFraction"].as_f64(), b["modelId"].as_str()) else { continue };
        let id = id.to_lowercase();
        let name = if id.contains("flash-lite") { "Flash Lite" } else if id.contains("flash") { "Flash" } else if id.contains("pro") { "Pro" } else { id.as_str() }.to_string();
        let reset = b["resetTime"].as_str().and_then(parse_time);
        match groups.iter_mut().find(|g| g.0 == name) {
            Some(g) => if f < g.1 { g.1 = f; g.2 = reset; },
            None => groups.push((name, f, reset)),
        }
    }
    let order = |n: &str| ["Pro", "Flash", "Flash Lite"].iter().position(|x| *x == n).unwrap_or(9);
    groups.sort_by_key(|g| order(&g.0));
    ProviderUsage {
        extra: groups.into_iter().map(|(n, f, r)| Row { title: format!("{n} · günlük kota"), left: f.clamp(0.0, 1.0), reset: r }).collect(),
        confidence: Confidence::Exact,
        notes: vec![
            l10n::t("Limit: Google Code Assist kota API'sinden (Gemini CLI oturumuyla). Gemini'de 5 saatlik/haftalık pencere yok, günlük kota var."),
            l10n::t("Token sayısı yerelde tutulmuyor."),
        ],
        ..Default::default()
    }
}
