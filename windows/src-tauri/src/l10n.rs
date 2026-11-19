//! Türkçe + İngilizce. Anahtarlar Türkçe metnin kendisi (Mac: L10n.swift). Kedinin adı CatName.apply ile işlenir.

use crate::settings;

pub fn english() -> bool {
    // Geliştirme/önizleme: MEOWLY_LANG=en|tr (Mac: AILIMIT_LANG).
    let env = std::env::var("MEOWLY_LANG").ok();
    match env.as_deref().unwrap_or(settings::get().language.as_str()) {
        "tr" => false,
        "en" => true,
        _ => {
            let lang = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
            lang & 0x3ff != 0x1f // LANG_TURKISH
        }
    }
}

/// Mac'in L10n.swift sözlüğü (`windows/scripts/extract-l10n.js` ile çıkarılır; `exact` + `phrases`).
fn dict() -> &'static std::collections::HashMap<String, String> {
    static D: std::sync::OnceLock<std::collections::HashMap<String, String>> = std::sync::OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../l10n_en.json")).unwrap_or_default())
}

/// Windows'a uyarlanmış metinler (`windows/scripts/win-l10n.js` ile üretilir).
fn win_dict() -> &'static std::collections::HashMap<String, String> {
    static D: std::sync::OnceLock<std::collections::HashMap<String, String>> = std::sync::OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("../l10n_win_en.json")).unwrap_or_default())
}

/// Sözlükte olmayan metinler için Mac'teki gibi kelime/öbek değiştirme (sıra önemli).
fn phrases() -> &'static Vec<(String, String)> {
    static P: std::sync::OnceLock<Vec<(String, String)>> = std::sync::OnceLock::new();
    P.get_or_init(|| dict().get("__phrases").and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default())
}

fn en(k: &str) -> Option<&'static str> {
    // Windows'a uyarlanmış karşılık önce (Mac'teki "on your Mac" gibi metinlerin yerine geçer).
    if let Some(v) = win_dict().get(k) { return Some(v.as_str()); }
    if let Some(v) = dict().get(k) { return Some(v.as_str()); }
    Some(match k {
        "Aç" => "Open",
        "Bu sağlayıcı Windows sürümünde henüz okunmuyor." => "This provider is not read on Windows yet.",
        "Windows'a giriş yapınca uygulama kendiliğinden açılır." => "Opens automatically when you sign in to Windows.",
        "Görev çubuğunda göster." => "Show in the taskbar.",
        "Soldaki kısım menüyü açar, sağdaki kutu Tonti'yi kutuya koyar ya da masaüstüne çıkarır." => "The left part opens the menu; the box on the right puts Tonti in the box or back on the desktop.",
        "Görev çubuğu" => "Taskbar",
        "Ayarlar" => "Settings",
        "✎ Not alıyorum…" => "✎ Taking notes…",
        "✎ Yeni sayfa açtım" => "✎ New page",
        "Oturumu kaydettim" => "Session saved",
        " oldu" => " in",
        "Biraz esneyelim mi?" => "Time for a stretch?",
        "Bugün %@ token!" => "%@ tokens today!",
        "Deftere yazdım" => "Noted it down",
        "Dün" => "Yesterday",
        " · %lld proje" => " · %lld projects",
        "Rapor hazır, bana tıkla" => "Report ready, click me",
        "%lld dk" => "%lldm",
        "%lld sa" => "%lldh",
        "%lld sa %lld dk" => "%lldh %lldm",
        "Tonti'yi kutuya koy" => "Put Tonti in the box",
        "Tonti'yi masaüstüne çıkar" => "Bring Tonti to the desktop",
        "Mırr…" => "Purr…",
        "Çık" => "Quit",
        _ => return None,
    })
}

/// Arayüz için bütün sözlük: seçili dildeki karşılıklar (kedinin adı işlenmiş), öbekler ve kedinin adı.
/// Arayüz anahtarları tek tek istemez; listede unutulan bir metin de çevrilir.
pub fn table() -> serde_json::Value {
    let english = english();
    let mut m = serde_json::Map::new();
    if english {
        for k in dict().keys().chain(win_dict().keys()) {
            if k == "__phrases" { continue; }
            m.insert(k.clone(), serde_json::Value::String(t(k)));
        }
    }
    serde_json::json!({ "en": english, "map": m, "phrases": if english { phrases().clone() } else { vec![] }, "name": cat_name() })
}

/// Çeviri + kedinin adı.
pub fn t(k: &str) -> String {
    if !english() { return cat_name_apply(k); }
    match en(k) {
        Some(s) => cat_name_apply(s),
        None => {
            let mut r = k.to_string();
            for (a, b) in phrases() { r = r.replace(a.as_str(), b.as_str()); }
            cat_name_apply(&r)
        }
    }
}

/// `%lld` / `%@` yer tutucularını sırayla doldurur.
pub fn f(k: &str, args: &[String]) -> String {
    let mut s = t(k);
    for a in args {
        let i = match (s.find("%lld"), s.find("%@")) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            _ => break,
        };
        let len = if s[i..].starts_with("%lld") { 4 } else { 2 };
        s.replace_range(i..i + len, a);
    }
    s.replace("%%", "%")
}

pub fn cat_name() -> String {
    let n = settings::get().cat_name.unwrap_or_default().trim().to_string();
    if n.is_empty() { "Tonti".into() } else { n }
}

const VOWELS: &str = "aeıioöuüAEIİOÖUÜ";

fn tr_lower(c: char) -> char {
    match c { 'I' => 'ı', 'İ' => 'i', _ => c.to_lowercase().next().unwrap_or(c) }
}

/// Son ünlüye göre dar ünlü: ı, i, u, ü.
fn narrow(n: &str) -> &'static str {
    let v = n.chars().rev().map(tr_lower).find(|c| VOWELS.contains(*c)).unwrap_or('e');
    match v { 'a' | 'ı' => "ı", 'o' | 'u' => "u", 'ö' | 'ü' => "ü", _ => "i" }
}

fn ends_with_vowel(n: &str) -> bool { n.chars().last().map(|c| VOWELS.contains(c)).unwrap_or(false) }

pub fn genitive(n: &str) -> String { format!("'{}{}n", if ends_with_vowel(n) { "n" } else { "" }, narrow(n)) }
pub fn accusative(n: &str) -> String { format!("'{}{}", if ends_with_vowel(n) { "y" } else { "" }, narrow(n)) }

pub fn cat_name_apply(s: &str) -> String {
    if !s.contains("Tonti") { return s.to_string(); }
    let n = cat_name();
    if n == "Tonti" { return s.to_string(); }
    s.replace("Tonti'nin", &(n.clone() + &genitive(&n)))
        .replace("Tonti'yi", &(n.clone() + &accusative(&n)))
        .replace("Tonti's", &(n.clone() + "'s"))
        .replace("Tonti", &n)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn suffixes() {
        assert_eq!(genitive("Pamuk"), "'un");
        assert_eq!(accusative("Mia"), "'yı");
        assert_eq!(genitive("Tonti"), "'nin");
        assert_eq!(accusative("Duman"), "'ı");
        assert_eq!(genitive("Gözde"), "'nin");
        assert_eq!(accusative("Böcük"), "'ü");
    }
}
