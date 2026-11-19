<div align="center">

<a href="https://meowly.vercel.app"><img src="assets/meowly-icon.png" width="110" alt="Meowly"></a>

# Meowly

### Kodunu yaz. Kedin not alsın.

Mac ve Windows masaüstünde yaşayan piksel bir kedi. Claude ve Codex limitlerin menü çubuğunda (Windows'ta görev çubuğunda);<br>
yapay zekâ oturumların ve commit'lerin kendiliğinden günlüğe ve rapora dönüşür.

<a href="https://meowly.vercel.app"><img src="https://img.shields.io/badge/%E2%86%93%20Meowly'yi%20indir-%C3%BCcretsiz%20%C2%B7%20macOS%20%26%20Windows-e88fa3?style=for-the-badge" alt="Meowly'yi indir"></a>
&nbsp;
<a href="https://meowly.vercel.app"><img src="https://img.shields.io/badge/Siteye%20git-meowly.vercel.app-160c0c?style=for-the-badge" alt="Site"></a>

[English](README.en.md) · [Hata bildir](https://github.com/mertpural/meowly/issues)

<a href="https://meowly.vercel.app"><img src="assets/stage-work.png" alt="Meowly: sen çalışırken kedi de laptopunda yazar"></a>

</div>

## Sen çalışırken o da çalışır

Claude ya da Codex'i açtığında kedin laptopunun başına geçer, hangi projede olduğunu not eder.
Limitlerin menü çubuğunda; bitmeye yakınken kedin söyler.

## Mola verince günlüğe yazar

<a href="https://meowly.vercel.app"><img src="assets/stage-diary.png" alt="Molada oturum Günlük'e yazılır"></a>

Her oturum süresi, token sayısı ve attığın commit'lerle **Günlük**'e düşer. Uzun oturumda mola hatırlatır.

## Gün sonunda rapor hazır

<a href="https://meowly.vercel.app/#report"><img src="assets/report-week.png" alt="Haftalık rapor"></a>

Günlük, haftalık, aylık ve yıllık rapor. Standup notu olarak kopyala ya da resim olarak paylaş.

## Sen uzaktayken uyur

<a href="https://meowly.vercel.app"><img src="assets/stage-sleep.png" alt="Kullanıcı uzaktayken kedi uyur"></a>

Bilgisayardan kalkınca kıvrılıp uyur, dönünce esneyip uyanır. Okşayınca mırlar, enseden tutup taşıyabilirsin.
**Adını sen koyarsın.**

## GitHub profil kartı

Bir yıllık token kullanımın profil README'nde. Kareler servise göre renklenir; içinde yalnızca günlük toplamlar var.

<a href="https://meowly.vercel.app/#github">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/card-tr-dark.svg">
  <img src="assets/card-tr-light.svg" alt="Meowly profil kartı (örnek veri)">
</picture>
</a>

## Kaynak kodu

Meowly açık kaynaktır: kod [GPL-3.0](LICENSE) ile lisanslıdır. Kedinin çizimleri, "Meowly" adı ve logosu bu lisansa dahil değildir ([ASSETS-LICENSE.md](ASSETS-LICENSE.md)).

| Klasör | İçerik | Derlemek |
|---|---|---|
| [`mac/`](mac) | Swift / SwiftUI, macOS 14+ | `cd mac && scripts/build_app.sh` |
| [`windows/`](windows) | Tauri 2 (Rust + WebView2), Windows 10/11 | `cd windows && npm install && npx tauri build` |

Kendin derlediğin sürüm kendini güncellemez; güncellemeler yalnızca sitedeki imzalı sürümlere gelir.

## Gizlilik

Sunucu yok, hesap yok. Her şey kendi bilgisayarında kalır. Sohbetlerini, ekranını, bastığın tuşları ve dosyalarının içeriğini asla okumaz.

<div align="center">

### [→ meowly.vercel.app](https://meowly.vercel.app)

Ücretsiz · macOS 14+ (Apple Silicon ve Intel) · Windows 10/11 (64 bit)

Meowly gününü biraz güzelleştiriyorsa bir ⭐ başkalarının da bulmasına yardım eder.

<sub>Kaynak kodu GPL-3.0 ile açık; hata bildirimleri ve öneriler için Issues.</sub>

</div>
