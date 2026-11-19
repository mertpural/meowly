import Foundation

enum AppInfo {
    /// Kullanıcıya görünen ad. Değiştirmek için yalnızca burayı ve scripts/build_app.sh'taki APP_NAME'i düzenle.
    static let name = "Meowly"
    /// Güncelleme bilgi dosyasının adresi. Yayın sitesi hazır olana kadar nil: kontrol yapılmaz, hiçbir istek gitmez.
    static let updateFeed: String? = "https://meowly.vercel.app/latest.json"
    /// Tanıtım sitesi (profil kartı bağlantısı buraya gider). Yayında adres kesinleşince güncellenir.
    static let site = "https://meowly.vercel.app"
}
