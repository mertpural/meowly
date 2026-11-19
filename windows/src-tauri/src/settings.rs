//! Ayarlar: %APPDATA%\Meowly\settings.json (Mac'te UserDefaults). Yalnızca değişince yazılır.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

fn yes() -> bool { true }
fn sys() -> String { "system".into() }

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    /// Kedi penceresinin sol üst köşesi (fiziksel piksel, ekran koordinatı).
    #[serde(default)] pub cat_home: Option<[i32; 2]>,
    #[serde(default)] pub cat_in_box: bool,
    #[serde(default)] pub cat_name: Option<String>,
    /// Tanıtım turu gösterildi (Mac: tourDone). Yalnızca ad ilk kez girilince bir kez çıkar.
    #[serde(default)] pub tour_done: bool,
    /// Takvim katmanı. Windows izin sormadan erişim verdiği için varsayılan kapalı; kullanıcı açınca okunur.
    #[serde(default)] pub calendar_on: bool,
    /// Günlük'teki izin kartı kapatıldı (Mac: logNudgeHidden).
    #[serde(default)] pub log_nudge_hidden: bool,
    /// Yıldız ricası (Mac: firstUseAt, starAskState 0 sorulmadı / 1 "sonra" / 2 bitti, starAskAt).
    #[serde(default)] pub first_use_at: f64,
    #[serde(default)] pub star_state: u8,
    #[serde(default)] pub star_ask_at: f64,
    #[serde(default = "sys")] pub language: String,
    #[serde(default = "yes")] pub cat_bubbles: bool,
    #[serde(default = "yes")] pub bubble_work: bool,
    #[serde(default = "yes")] pub bubble_break: bool,
    #[serde(default = "yes")] pub bubble_daily: bool,
    #[serde(default = "yes")] pub bubble_record: bool,
    #[serde(default = "yes")] pub bubble_chat: bool,
    #[serde(default)] pub cat_record_day: Option<i64>,
    #[serde(default)] pub cat_record: u64,
    #[serde(default)] pub cat_daily_day: Option<i64>,
    /// Token toplamlarına cache okumaları da eklensin mi (varsayılan kapalı).
    #[serde(default)] pub include_cache: bool,
    /// Açık sağlayıcılar (varsayılan yalnızca Claude).
    #[serde(default = "claude_only")] pub enabled: Vec<String>,
    /// "Limiti Claude Code komutuyla ölç" (varsayılan açık).
    #[serde(default = "yes")] pub cli_probe: bool,
    /// "Claude oturumunu otomatik yenile" (varsayılan açık).
    #[serde(default = "yes")] pub refresh_login: bool,
    /// Otomatik yenileme aralığı (sn); 0 = yalnızca elle. Varsayılan 30 sn.
    #[serde(default = "thirty")] pub sync_interval: u64,
    /// Görünüm: Açık (false) / Koyu (true). Pencere, açılır pencere ve şerit birlikte değişir.
    #[serde(default)] pub dark: bool,
    /// Şeritte "5s" / "H" etiketleri.
    #[serde(default = "yes")] pub show_tags: bool,
    #[serde(default = "yes")] pub show_five: bool,
    #[serde(default = "yes")] pub show_week: bool,
    /// Gösterge stili: percent, bar, barPercent, ring, dot, battery, segments.
    #[serde(default = "pct")] pub style_five: String,
    #[serde(default = "pct")] pub style_week: String,
    /// "Bitiş tahmini notu".
    #[serde(default = "yes")] pub show_forecast: bool,
    /// Açılır penceredeki seçili sekme ("all" ya da sağlayıcı).
    #[serde(default = "all")] pub selected_tab: String,
    /// Kullanım: ısı haritası sağlayıcıya göre renkli mi (varsayılan evet).
    #[serde(default = "yes")] pub heat_by_provider: bool,
    /// Kullanım: projeler kartının süresi (30, 90; 0 = tümü).
    #[serde(default)] pub projects_days: i64,
    /// "Günlükte commit'leri göster" (varsayılan açık).
    #[serde(default = "yes")] pub journal_commits: bool,
    /// Günlük: "ai" ya da "pc".
    #[serde(default = "ai")] pub diary_mode: String,
    /// Bildirim ve otomasyon (Mac varsayılanları).
    #[serde(default = "yes")] pub notifications_on: bool,
    #[serde(default)] pub login_item_on: bool,
    #[serde(default = "yes")] pub show_next_start: bool,
    #[serde(default = "yes")] pub weekly_summary: bool,
    /// "Claude limitini Claude Code'dan al" (durum satırı); yalnızca kullanıcı açarsa.
    #[serde(default)] pub statusline_on: bool,
    /// Claude pencere başlatıcı (varsayılan kapalı).
    #[serde(default)] pub starter_on: bool,
    /// Bilgisayar günlüğü katmanları (hepsi varsayılan açık).
    #[serde(default = "yes")] pub log_apps: bool,
    #[serde(default = "yes")] pub log_titles: bool,
    #[serde(default = "yes")] pub log_sites: bool,
    #[serde(default = "yes")] pub log_files: bool,
    #[serde(default = "yes")] pub log_calendar: bool,
    /// GitHub profil kartı.
    #[serde(default)] pub gh_on: bool,
    #[serde(default)] pub gh_user: String,
    #[serde(default = "daily")] pub gh_interval: String,
    #[serde(default = "yes")] pub gh_split: bool,
    #[serde(default = "yes")] pub gh_dark: bool,
    /// Son başarılı gönderim, son deneme, gönderilen kartın SHA-256 özeti, son durum metni (Mac: ghLastPush, ghLastTry, ghLastHash, ghStatus).
    #[serde(default)] pub gh_last_push: f64,
    #[serde(default)] pub gh_last_try: f64,
    #[serde(default)] pub gh_last_hash: String,
    #[serde(default)] pub gh_status: String,
    /// Güncellemeleri kendiliğinden denetle.
    #[serde(default = "yes")] pub update_check: bool,
}

fn daily() -> String { "daily".into() }

fn ai() -> String { "ai".into() }

fn pct() -> String { "percent".into() }
fn all() -> String { "all".into() }

fn claude_only() -> Vec<String> { vec!["claude".into()] }
fn thirty() -> u64 { 30 }

impl Settings {
    pub fn is_enabled(&self, p: &str) -> bool { self.enabled.iter().any(|e| e == p) }
}

impl Default for Settings {
    fn default() -> Self { serde_json::from_str("{}").unwrap() }
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let d = base.join("Meowly");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn store() -> &'static Mutex<Settings> {
    static S: OnceLock<Mutex<Settings>> = OnceLock::new();
    S.get_or_init(|| {
        let s = std::fs::read_to_string(data_dir().join("settings.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        Mutex::new(s)
    })
}

pub fn get() -> Settings { store().lock().unwrap().clone() }

/// Diğer süreç ayar dosyasını değiştirdi: yeniden oku.
pub fn reload() {
    if let Some(s) = std::fs::read_to_string(data_dir().join("settings.json")).ok().and_then(|t| serde_json::from_str(&t).ok()) {
        *store().lock().unwrap() = s;
    }
}

pub fn update(f: impl FnOnce(&mut Settings)) {
    let mut g = store().lock().unwrap();
    f(&mut g);
    if let Ok(t) = serde_json::to_string_pretty(&*g) {
        let p = data_dir().join("settings.json");
        let tmp = p.with_extension("json.tmp");
        if std::fs::write(&tmp, t).is_ok() { let _ = std::fs::rename(&tmp, &p); }
    }
}
