import Foundation

/// Büyük kayıt dosyalarını parça parça okur: bütün dosya hiçbir zaman belleğe alınmaz.
enum LineStream {
    static let chunk = 1 << 20   // 1 MB

    /// `h`'den sonuna kadar okur ve her tam satırı (\n hariç; boş satırlar atlanır) satırın başladığı
    /// konumla (okumaya başlanan yere göre bayt) `body`'ye verir. Sonu \n ile bitmeyen yarım son satır verilmez.
    /// Dönen değer: son \n dahil okunan bayt sayısı.
    @discardableResult
    static func each(_ h: FileHandle, _ body: (Data, Int) -> Void) -> Int {
        var carry = Data()
        var pos = 0   // carry'nin ilk baytının konumu
        while true {
            let more: Data? = autoreleasepool { try? h.read(upToCount: chunk) }
            guard let more, !more.isEmpty else { break }
            autoreleasepool {
                carry.append(more)
                let base = carry.startIndex
                var lineStart = base
                while let nl = carry[lineStart...].firstIndex(of: 10) {
                    if nl > lineStart { body(Data(carry[lineStart..<nl]), pos + (lineStart - base)) }
                    lineStart = nl + 1
                }
                pos += lineStart - base
                carry = Data(carry[lineStart...])
            }
        }
        return pos
    }
}
