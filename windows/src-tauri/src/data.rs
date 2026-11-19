//! Veri iş parçacığı: sağlayıcıları okur, sonucu paylaşılan anlık görüntüye koyar ve kediye haber verir.
//! Sürekli yoklama yok: kayıt klasörü değişince ya da yenileme aralığı dolunca (varsayılan 30 sn) çalışır.

use crate::{claude, codex, sense, settings};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Source { #[default] StatusLine, Cli, Api, Codex }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Confidence { Exact, Estimate, #[default] None }

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Limits {
    pub five_left: Option<f64>,
    pub week_left: Option<f64>,
    pub five_reset: Option<f64>,
    pub week_reset: Option<f64>,
    pub fetched_at: f64,
    pub source: Source,
}

/// 5 saatlik/haftalık dışındaki pencereler (Gemini günlük kota, Antigravity model grupları). Mac: LimitRow.
#[derive(Clone, Debug, Default)]
pub struct Row { pub title: String, pub left: f64, pub reset: Option<f64> }

#[derive(Clone, Debug, Default)]
pub struct ProviderUsage {
    pub limits: Option<Limits>,
    /// Mac: Usage.extra. Doluysa kartta ve çubukta bunlar gösterilir.
    pub extra: Vec<Row>,
    pub confidence: Confidence,
    pub today_tokens: i64,
    pub stale_since: Option<f64>,
    pub auth_expired: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub claude: Option<ProviderUsage>,
    pub codex: Option<ProviderUsage>,
    pub gemini: Option<ProviderUsage>,
    pub antigravity: Option<ProviderUsage>,
    /// Ek araçlar (OpenCode, Kimi, Cline/Roo): yalnızca bilgisayarda izi varsa (Mac: ExtraProviders.present).
    pub extras: Vec<(&'static str, ProviderUsage)>,
    pub updated_at: f64,
}

fn snap() -> &'static Mutex<Snapshot> {
    static S: OnceLock<Mutex<Snapshot>> = OnceLock::new();
    S.get_or_init(Default::default)
}

pub fn snapshot() -> Snapshot { snap().lock().unwrap().clone() }

/// Mac'teki sağlayıcı sırası: dört çekirdek, sonra ek araçlar.
pub const CORE: [&str; 4] = ["claude", "codex", "gemini", "antigravity"];

impl Snapshot {
    pub fn usage(&self, p: &str) -> Option<&ProviderUsage> {
        match p {
            "claude" => self.claude.as_ref(), "codex" => self.codex.as_ref(),
            "gemini" => self.gemini.as_ref(), "antigravity" => self.antigravity.as_ref(),
            _ => self.extras.iter().find(|e| e.0 == p).map(|e| &e.1),
        }
    }
    /// Gösterilebilecek bütün sağlayıcılar: dört çekirdek ve izi olan ek araçlar.
    pub fn ids(&self) -> Vec<&'static str> { CORE.into_iter().chain(self.extras.iter().map(|e| e.0)).collect() }
}

impl ProviderUsage {
    /// Mac: Usage.otherRows — 5 saatlik ya da haftalık olmayan pencereler.
    pub fn other_rows(&self) -> Vec<&Row> { self.extra.iter().filter(|r| !r.title.contains("5 saatlik") && !r.title.contains("Haftalık")).collect() }
}

/// Ek satırlardan 5 saatlik/haftalık en kısıtlayıcı olanı (Mac: fiveRow/weekRow, havuzlar varsa en düşüğü).
pub fn min_row(rows: &[Row], key: &str) -> Option<(f64, Option<f64>)> {
    rows.iter().filter(|r| r.title.contains(key)).min_by(|a, b| a.left.partial_cmp(&b.left).unwrap()).map(|r| (r.left, r.reset))
}

fn kick() -> &'static Mutex<Option<Sender<bool>>> {
    static K: OnceLock<Mutex<Option<Sender<bool>>>> = OnceLock::new();
    K.get_or_init(|| Mutex::new(None))
}

/// Hemen yenile (kayıt klasörü değişti ya da elle "yenile").
pub fn refresh_now() {
    if let Some(tx) = kick().lock().unwrap().as_ref() { let _ = tx.send(false); }
}

/// Elle "yenile": beklemeden oku.
pub fn refresh_now_fast() {
    if let Some(tx) = kick().lock().unwrap().as_ref() { let _ = tx.send(true); }
}

pub fn read_all() {
    let s = settings::get();
    let mut out = Snapshot { updated_at: sense::now_secs(), ..Default::default() };
    // Mac gibi tüm sağlayıcılar okunur (Sağlayıcılar sayfası durumlarını gösterir); şerit ve açılır pencere
    // yalnızca açık olanları gösterir.
    let _ = &s;
    out.claude = Some(claude::read());
    out.codex = Some(codex::read());
    // Gemini yalnızca açıksa okunur (Google'a Gemini CLI oturumuyla istek gider). Antigravity'nin token'ları yerel kayıtlardan
    // her zaman, kotası yerel dil sunucusundan yalnızca açıksa okunur. Ek araçlar yalnızca izi varsa.
    if s.is_enabled("gemini") { out.gemini = Some(crate::gemini::read()); }
    out.antigravity = Some(crate::antigravity::read(s.is_enabled("antigravity")));
    out.extras = crate::extras::read_present();
    *snap().lock().unwrap() = out;
}

/// `notify`: her okumadan sonra çağrılır (kediye haber vermek için; ana iş parçacığına mesaj gönderir).
pub fn start(notify: fn()) {
    claude::mark_launch();
    let (tx, rx): (Sender<bool>, Receiver<bool>) = channel();
    *kick().lock().unwrap() = Some(tx);
    std::thread::spawn(move || loop {
        read_all();
        crate::notify::after_read(&snapshot());
        crate::update::check_if_due();
        crate::starter::schedule(snapshot().claude.as_ref());
        crate::github::sync_if_due();
        notify();
        let iv = settings::get().sync_interval;
        let wait = if iv == 0 { Duration::from_secs(24 * 3600) } else { Duration::from_secs(iv.max(15)) };
        match rx.recv_timeout(wait) {
            Ok(fast) => {
                // Art arda gelen değişimleri topla: Claude bir cevap boyunca dosyaya defalarca yazar.
                // Dosya değişimiyle en sık 15 sn'de bir okunur; elle yenileme beklemez.
                if !fast { std::thread::sleep(Duration::from_secs(15)); }
                while rx.try_recv().is_ok() {}
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    });
}
