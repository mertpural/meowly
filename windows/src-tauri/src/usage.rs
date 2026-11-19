//! Kullanım sayfası (Mac: HistoryView.swift, Extras.swift → ProjectsCard). Arayüz sürecinde, veritabanından okunur.

use crate::store;
use crate::{cat::format_tokens, ipc, look, sense, settings};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const PROVIDERS: [&str; 3] = ["claude", "codex", "antigravity"];

fn cost_since(since: f64) -> f64 { store::model_sums(since).iter().map(|(m, s)| store::cost(m, s)).sum() }

/// Sayfanın tüm verisi. `range`: day | week | month; `project_days`: 30 | 90 | 0 (tümü).
#[tauri::command]
pub async fn usage_page(range: String, project_days: i64) -> Value {
    let project_days = if project_days < 0 { settings::get().projects_days } else { project_days };
    let now = sense::now_secs();
    let t0 = store::start_of_day(now);
    let day = 86400.0;
    let sum_all = |since: f64| PROVIDERS.iter().map(|p| store::billable(p, since)).sum::<i64>();
    let (today, y_and_t, w, pw_and_w, m) = (sum_all(t0), sum_all(t0 - day), sum_all(t0 - 6.0 * day), sum_all(t0 - 13.0 * day), sum_all(t0 - 29.0 * day));

    // En çok kullanılan projeler (Claude Code, klasör adıyla), ilk 6.
    let psince = if project_days == 0 { 0.0 } else { t0 - (project_days - 1) as f64 * day };
    let mut by: BTreeMap<String, (i64, f64)> = BTreeMap::new();
    for (p, model, s) in store::project_sums(psince) {
        let e = by.entry(p).or_default();
        e.0 += s.billable();
        e.1 += store::cost(&model, &s);
    }
    let mut projects: Vec<(String, i64, f64)> = by.into_iter().map(|(k, v)| (k, v.0, v.1)).collect();
    projects.sort_by(|a, b| b.1.cmp(&a.1));
    projects.truncate(6);

    // Isı haritası: son 3 yıl, gün → sağlayıcı → token.
    let mut heat: BTreeMap<String, BTreeMap<String, i64>> = BTreeMap::new();
    for (b, p, _m, t) in store::series(t0 - 3.0 * 365.0 * day, false) {
        *heat.entry(b).or_default().entry(p).or_default() += t;
    }
    let all_time = PROVIDERS.iter().map(|p| store::billable(p, 0.0)).sum::<i64>();

    // Grafik: seçili aralık.
    let (since, hourly) = match range.as_str() {
        "day" => (now - 24.0 * 3600.0, true),
        "month" => (t0 - 29.0 * day, false),
        _ => (t0 - 6.0 * day, false),
    };
    let points: Vec<Value> = store::series(since, hourly).into_iter()
        .map(|(b, p, m, t)| json!({ "b": b, "p": p, "m": if m.is_empty() { "?".into() } else { m }, "t": t })).collect();

    let colors: serde_json::Map<String, Value> = ["claude", "codex", "gemini", "antigravity"].iter()
        .map(|p| (look::provider_name(p).to_string(), Value::String(look::hex(look::provider_color(p))))).collect();
    let s = settings::get();
    json!({
        "dark": s.dark,
        "en": crate::l10n::english(),
        "includeCache": s.include_cache,
        "heatByProvider": s.heat_by_provider,
        "projectDays": project_days,
        "today": today, "yesterday": y_and_t - today, "last7": w, "prev7": pw_and_w - w, "last30": m,
        "cost": [store::format_usd(cost_since(t0)), store::format_usd(cost_since(t0 - 6.0 * day)), store::format_usd(cost_since(t0 - 29.0 * day))],
        "projects": projects.iter().map(|(n, t, c)| json!({ "name": n, "tokens": t, "fmt": format_tokens(*t), "cost": store::format_usd(*c) })).collect::<Vec<_>>(),
        "heat": heat,
        "allTime": all_time,
        "points": points,
        "hourly": hourly,
        "since": since,
        "now": now,
        "colors": colors,
        "providerColors": (["claude", "codex", "gemini", "antigravity", "opencode", "kimi", "cline"].iter().map(|p| (p.to_string(), Value::String(look::hex(look::provider_color(p))))).collect::<serde_json::Map<String, Value>>()),
    })
}

#[tauri::command]
pub fn set_usage_pref(key: String, value: Value) {
    settings::update(|s| match key.as_str() {
        "includeCache" => s.include_cache = value.as_bool().unwrap_or(false),
        "heatByProvider" => s.heat_by_provider = value.as_bool().unwrap_or(true),
        "projectDays" => s.projects_days = value.as_i64().unwrap_or(0),
        _ => {}
    });
    ipc::post_main(ipc::WM_IPC_SETTINGS, 0);
}

/// CSV'yi Windows'un kaydet penceresiyle kaydeder.
#[tauri::command]
pub async fn save_csv() -> bool {
    let csv = store::export_csv();
    match crate::win::save_dialog("meowly-gecmis.csv", "CSV (*.csv)\0*.csv\0\0") {
        Some(p) => std::fs::write(p, csv).is_ok(),
        None => false,
    }
}
