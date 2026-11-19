//! GitHub profil kartı (Mac: GitHubProfile.swift → ProfileCard, GitHubSync).
//! Kart yerelde üretilen bir SVG ısı haritasıdır: yalnızca son 12 ayın günlük toplam token'ı (proje, model, sohbet yok).
//! Kullanıcının kendi profil reposuna (kullanıcıadı/kullanıcıadı) `meowly.svg` olarak konur. İlk gönderim her zaman elle;
//! sonra seçilen sıklıkla ve yalnızca kart değiştiyse. Anahtar Windows Kimlik Bilgisi Yöneticisi'nde durur, yalnızca
//! gönderirken okunur, hiçbir yere yazılmaz ve günlüğe girmez.

use crate::cat::format_tokens;
use crate::{l10n, look, settings, store};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub const FILE_NAME: &str = "meowly.svg";

fn key_of(t: f64) -> String { let (y, m, d, ..) = look::local(t); format!("{y:04}-{m:02}-{d:02}") }

/// "18 Kas 2026" / "Nov 18, 2026" (Mac: DateFormatter .medium).
fn medium_date(t: f64, en: bool) -> String {
    const TR: [&str; 12] = ["Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara"];
    const EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, d, ..) = look::local(t);
    let i = (m as usize).clamp(1, 12) - 1;
    if en { format!("{} {d}, {y}", EN[i]) } else { format!("{d} {} {y}", TR[i]) }
}

fn esc(s: &str) -> String { s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;") }

/// Kartın SVG'si (Mac ile aynı ölçüler, renkler ve metinler).
pub fn svg(split: bool, dark: bool, now: f64) -> String {
    let en = l10n::english();
    // Haftalar pazardan başlar (Mac: firstWeekday = 1). Yaz saatine takılmamak için gün ortasından hesaplanır.
    let today = store::start_of_day(now);
    let wd = look::local(today + 12.0 * 3600.0).5 as f64; // 0 = pazar
    let this_week = store::start_of_day(today - wd * 86400.0 + 12.0 * 3600.0);
    let start = store::start_of_day(this_week - 52.0 * 7.0 * 86400.0 + 12.0 * 3600.0);

    let mut days: HashMap<String, HashMap<String, i64>> = HashMap::new();
    let mut total: i64 = 0;
    for (b, p, _m, t) in store::series(start, false) {
        *days.entry(b).or_default().entry(p).or_default() += t;
        total += t;
    }
    let mx = days.values().map(|d| d.values().sum::<i64>()).max().unwrap_or(1).max(1);

    let bg = if dark { "#0d1117" } else { "#ffffff" };
    let text = if dark { "#e6edf3" } else { "#1f2328" };
    let sub = if dark { "#8b949e" } else { "#656d76" };
    let line = if dark { "#30363d" } else { "#d0d7de" };
    let scale = if dark { ["#161b22", "#4a2232", "#7a3550", "#b9557a", "#e98aa9"] } else { ["#ebedf0", "#f3d1dc", "#e7a3b9", "#d27394", "#a44a6b"] };
    let prov_color = |p: &str| -> &str {
        match p {
            "claude" => "#d97850",
            "codex" => if dark { "#b8b8b8" } else { "#524d54" },
            "gemini" => "#78a0d6",
            "antigravity" => if dark { "#6b87e0" } else { "#334a9e" },
            "opencode" => "#78b88f", "kimi" => "#a387d6", "cline" => "#dbc266",
            _ => scale[3],
        }
    };
    let order = ["claude", "codex", "gemini", "antigravity", "opencode", "kimi", "cline"];
    let names = [("claude", "Claude"), ("codex", "Codex"), ("gemini", "Gemini"), ("antigravity", "Antigravity")];
    let opacity = [0.0, 0.4, 0.6, 0.8, 1.0];

    let (cell, gap, left, top) = (11i64, 3i64, 20i64, 52i64);
    let width = left * 2 + 53 * (cell + gap) - gap;
    let height = top + 7 * (cell + gap) - gap + 40;
    let title = "Meowly";
    let total_text = if en { format!("{} tokens · last 12 months", format_tokens(total)) } else { format!("{} token · son 12 ay", format_tokens(total)) };

    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"{title}\">\n");
    s += &format!("<rect width=\"{width}\" height=\"{height}\" rx=\"10\" fill=\"{bg}\" stroke=\"{line}\"/>\n");
    s += "<style>text{font-family:-apple-system,'Segoe UI',Helvetica,Arial,sans-serif}</style>\n";
    s += &format!("<text x=\"{left}\" y=\"32\" font-size=\"15\" font-weight=\"600\" fill=\"{text}\">{title}</text>\n");
    s += &format!("<text x=\"{}\" y=\"32\" font-size=\"12\" text-anchor=\"end\" fill=\"{sub}\">{}</text>\n", width - left, esc(&total_text));
    s += &format!("<defs><clipPath id=\"c\"><rect width=\"{cell}\" height=\"{cell}\" rx=\"2\"/></clipPath></defs>\n");
    for w in 0..53i64 {
        for d in 0..7i64 {
            let date = store::start_of_day(start + ((w * 7 + d) as f64) * 86400.0 + 12.0 * 3600.0);
            if date > today { continue; }
            let empty = HashMap::new();
            let day = days.get(&key_of(date + 12.0 * 3600.0)).unwrap_or(&empty);
            let v: i64 = day.values().sum();
            let r = v as f64 / mx as f64;
            let lv = if v == 0 { 0 } else if r < 0.15 { 1 } else if r < 0.4 { 2 } else if r < 0.7 { 3 } else { 4 };
            let (x, y) = (left + w * (cell + gap), top + d * (cell + gap));
            let usage = if v == 0 { (if en { "no usage" } else { "kullanım yok" }).to_string() } else { format!("{}{}", format_tokens(v), if en { " tokens" } else { " token" }) };
            let tip = format!("<title>{}: {}</title>", medium_date(date + 12.0 * 3600.0, en), usage);
            if split && lv > 0 {
                // Uygulamadaki gibi: kare, o günkü servis paylarına göre dikey bölümlere ayrılır; koyuluk yoğunluğu gösterir.
                s += &format!("<g transform=\"translate({x} {y})\" clip-path=\"url(#c)\" fill-opacity=\"{}\">", opacity[lv]);
                let mut yy = 0.0;
                for name in order {
                    let Some(&tv) = day.get(name) else { continue };
                    if tv <= 0 { continue; }
                    let h = cell as f64 * tv as f64 / v as f64;
                    s += &format!("<rect y=\"{yy:.2}\" width=\"{cell}\" height=\"{:.2}\" fill=\"{}\"/>", h + 0.3, prov_color(name));
                    yy += h;
                }
                s += &tip;
                s += "</g>\n";
            } else {
                s += &format!("<rect x=\"{x}\" y=\"{y}\" width=\"{cell}\" height=\"{cell}\" rx=\"2\" fill=\"{}\">{tip}</rect>\n", scale[lv]);
            }
        }
    }
    let fy = height - 14;
    if split {
        // Yalnızca son 12 ayda kullanılan servisler; kare solda, ad hemen yanında.
        let mut x = left;
        for (id, name) in names {
            if !days.values().any(|d| d.get(id).copied().unwrap_or(0) > 0) { continue; }
            s += &format!("<rect x=\"{x}\" y=\"{}\" width=\"9\" height=\"9\" rx=\"2\" fill=\"{}\"/>\n", fy - 9, prov_color(id));
            s += &format!("<text x=\"{}\" y=\"{fy}\" font-size=\"10\" fill=\"{sub}\">{name}</text>\n", x + 13);
            x += 13 + name.chars().count() as i64 * 6 + 16;
        }
    } else {
        let (less, more) = if en { ("Less", "More") } else { ("Az", "Çok") };
        let mut x = width - left;
        s += &format!("<text x=\"{x}\" y=\"{fy}\" font-size=\"10\" text-anchor=\"end\" fill=\"{sub}\">{more}</text>\n");
        x -= more.chars().count() as i64 * 6 + 6;
        for lv in (0..=4).rev() {
            x -= 11;
            s += &format!("<rect x=\"{x}\" y=\"{}\" width=\"10\" height=\"10\" rx=\"2\" fill=\"{}\"/>\n", fy - 9, scale[lv]);
            x -= 3;
        }
        s += &format!("<text x=\"{}\" y=\"{fy}\" font-size=\"10\" text-anchor=\"end\" fill=\"{sub}\">{less}</text>\n", x - 3);
    }
    s += "</svg>\n";
    s
}

// MARK: Gönderme

fn set_status(m: &str) { settings::update(|s| s.gh_status = m.to_string()); }

/// Veri iş parçacığında (ana süreç): açıksa, en az bir kez elle gönderildiyse, süresi geldiyse ve kart değiştiyse gönderir.
pub fn sync_if_due() {
    let s = settings::get();
    // İlk gönderim her zaman elle yapılır: anahtar kaydedilir kaydedilmez profile commit atılmaz.
    if !s.gh_on || s.gh_interval == "manual" || s.gh_user.trim().is_empty() || s.gh_last_push <= 0.0 { return; }
    let now = crate::sense::now_secs();
    let need = if s.gh_interval == "weekly" { 6.5 * 86400.0 } else { 20.0 * 3600.0 };
    if now - s.gh_last_push < need || now - s.gh_last_try < 6.0 * 3600.0 { return; }
    if !crate::settings_ui::gh_has_token() { return; }
    let _ = push(false);
}

/// `force`: elle gönderimde değişiklik kontrolü yapılmaz. Dönen metin kullanıcıya gösterilir.
pub fn push(force: bool) -> String {
    let s = settings::get();
    let user = s.gh_user.trim().to_string();
    // Geliştirme/deneme: MEOWLY_GHTOKEN ve MEOWLY_GHAPI (yerel sahte sunucu) — Mac: AILIMIT_GHTOKEN, AILIMIT_GHAPI.
    let tok = std::env::var("MEOWLY_GHTOKEN").ok().or_else(crate::settings_ui::gh_read_token);
    let (Some(tok), false) = (tok, user.is_empty()) else { return l10n::t("Kullanıcı adı ve anahtar gerekli") };
    let now = crate::sense::now_secs();
    settings::update(|s| s.gh_last_try = now);
    let card = svg(s.gh_split, s.gh_dark, now);
    let hash: String = Sha256::digest(card.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
    if !force && s.gh_last_hash == hash {
        settings::update(|s| s.gh_last_push = now);
        return l10n::t("Kart değişmemiş, gönderilmedi");
    }
    let base = std::env::var("MEOWLY_GHAPI").unwrap_or_else(|_| "https://api.github.com".into());
    let url = format!("{base}/repos/{user}/{user}/contents/{FILE_NAME}");
    let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(20)).build();
    let req = |m: &str| agent.request(m, &url)
        .set("Authorization", &format!("Bearer {tok}"))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .set("User-Agent", "Meowly");
    let denied = || { let m = l10n::t("GitHub anahtarı geçersiz ya da yetkisiz (repo içeriğine yazma izni gerekir)"); set_status(&m); m };
    // Var olan dosyanın özeti (güncellemek için gerekir). Dosya yoksa sorun değil; repo da yoksa PUT 404 döner.
    let mut sha: Option<String> = None;
    match req("GET").call() {
        Ok(r) => { sha = r.into_string().ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).and_then(|v| v["sha"].as_str().map(String::from)); }
        Err(ureq::Error::Status(404, _)) => {}
        Err(ureq::Error::Status(401 | 403, _)) => return denied(),
        Err(ureq::Error::Status(..)) => {}
        Err(_) => { let m = l10n::t("GitHub'a bağlanılamadı"); set_status(&m); return m; }
    }
    let mut body = serde_json::json!({ "message": "Meowly profil kartı", "content": crate::settings_ui::b64(card.as_bytes()) });
    if let Some(sha) = sha { body["sha"] = serde_json::Value::String(sha); }
    let m = match req("PUT").set("Content-Type", "application/json").send_string(&body.to_string()) {
        Ok(_) => {
            settings::update(|s| { s.gh_last_push = now; s.gh_last_hash = hash.clone(); });
            l10n::t("Gönderildi")
        }
        Err(ureq::Error::Status(404, _)) => l10n::f("%@/%@ reposu bulunamadı. GitHub'da bu adla bir repo aç (README'li, herkese açık).", &[user.clone(), user.clone()]),
        Err(ureq::Error::Status(401 | 403, _)) => return denied(),
        Err(ureq::Error::Status(c, _)) => l10n::f("GitHub hatası (%lld)", &[c.to_string()]),
        Err(_) => l10n::t("GitHub'a bağlanılamadı"),
    };
    set_status(&m);
    m
}
