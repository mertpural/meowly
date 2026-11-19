//! Takvim (Mac: Tonti/Activity.swift → CalendarLog). Windows'un kendi takvim deposundan (Windows'a eklenmiş hesaplar)
//! yalnızca okur, hiçbir şeyi değiştirmez. Gün boyu süren, iptal edilen ve reddedilen etkinlikler atlanır.
//! Mac'te sistem izni olmadan okunmaz; Windows masaüstü uygulamalarına sormadan izin verdiği için katman varsayılan kapalı,
//! kullanıcı Ayarlar → Bilgisayar günlüğü → Takvim'i açınca okunur. Raporlara toplantı adları girmez, yalnızca toplam süre.

use crate::settings;
use windows::ApplicationModel::Appointments::*;
use windows::Foundation::{DateTime, TimeSpan};

#[derive(Clone, Debug)]
pub struct Meeting { pub title: String, pub start: f64, pub end: f64 }

/// Unix saniyesi ↔ WinRT DateTime (1601'den beri 100 ns).
fn to_dt(t: f64) -> DateTime { DateTime { UniversalTime: ((t + 11_644_473_600.0) * 10_000_000.0) as i64 } }
fn from_dt(d: DateTime) -> f64 { d.UniversalTime as f64 / 10_000_000.0 - 11_644_473_600.0 }

fn init() {
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
    unsafe { let _ = RoInitialize(RO_INIT_MULTITHREADED); }
}

fn store() -> windows::core::Result<AppointmentStore> {
    init();
    AppointmentManager::RequestStoreAsync(AppointmentStoreAccessType::AllCalendarsReadOnly)?.get()
}

/// Windows takvim deposuna erişilebiliyor mu (Ayarlar'da "Çalışıyor" / "İZİN VER").
/// Ayrı iş parçacığında, en çok 3 sn beklenir: bazı bilgisayarlarda Windows'un takvim hizmeti yanıt vermiyor
/// (ya da "RPC sunucusu kullanılamıyor" dönüyor); o zaman sayfa takılmaz, yalnızca "erişim yok" sayılır. Sonuç 60 sn saklanır.
pub fn access_ok() -> bool {
    use std::sync::Mutex;
    static CACHE: Mutex<Option<(std::time::Instant, bool)>> = Mutex::new(None);
    if let Some((at, v)) = *CACHE.lock().unwrap() { if at.elapsed().as_secs() < 60 { return v; } }
    let v = bounded(std::time::Duration::from_secs(3), || store().is_ok()).unwrap_or(false);
    *CACHE.lock().unwrap() = Some((std::time::Instant::now(), v));
    v
}

/// İşi ayrı bir iş parçacığında çalıştırır; süre dolarsa None (iş arkada bitene kadar sürer, sonucu atılır).
fn bounded<T: Send + 'static>(d: std::time::Duration, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || { let _ = tx.send(f()); });
    rx.recv_timeout(d).ok()
}

/// Yalnızca katman açıksa. Başlangıç ve bitiş aralığa kırpılır, başlangıca göre sıralanır.
pub fn meetings(from: f64, to: f64) -> Vec<Meeting> {
    if !settings::get().calendar_on || to <= from || !access_ok() { return vec![]; }
    bounded(std::time::Duration::from_secs(4), move || meetings_now(from, to)).unwrap_or_default()
}

fn meetings_now(from: f64, to: f64) -> Vec<Meeting> {
    let Ok(st) = store() else { return vec![] };
    let run = || -> windows::core::Result<Vec<Meeting>> {
        let opt = FindAppointmentsOptions::new()?;
        let props = opt.FetchProperties()?;
        for p in [AppointmentProperties::Subject()?, AppointmentProperties::StartTime()?, AppointmentProperties::Duration()?,
                  AppointmentProperties::AllDay()?, AppointmentProperties::IsCanceledMeeting()?, AppointmentProperties::UserResponse()?] {
            props.Append(&p)?;
        }
        let list = st.FindAppointmentsAsyncWithOptions(to_dt(from), TimeSpan { Duration: ((to - from) * 10_000_000.0) as i64 }, &opt)?.get()?;
        let mut out = Vec::new();
        for a in list {
            if a.AllDay().unwrap_or(false) || a.IsCanceledMeeting().unwrap_or(false) { continue; }
            if a.UserResponse().map(|r| r == AppointmentParticipantResponse::Declined).unwrap_or(false) { continue; }
            let (Ok(s), Ok(d)) = (a.StartTime(), a.Duration()) else { continue };
            let start = from_dt(s);
            let end = start + d.Duration as f64 / 10_000_000.0;
            if end <= from || start >= to { continue; }
            out.push(Meeting { title: a.Subject().map(|h| h.to_string()).unwrap_or_default(), start: start.max(from), end: end.min(to) });
        }
        out.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
        Ok(out)
    };
    run().unwrap_or_default()
}

/// Toplantıların toplam süresi (raporlar: başlık yok, yalnızca toplam).
pub fn total(m: &[Meeting]) -> f64 { m.iter().map(|x| x.end - x.start).sum() }
