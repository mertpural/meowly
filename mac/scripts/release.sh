#!/bin/zsh
# Mac ve Windows sürümünü AYNI numarayla birlikte yayınlar:
#   Mac'i derleyip imzalar, Windows kurulumunu kaynak deponun windows-release dalından alıp doğrular,
#   siteyi yayına alır, GitHub Releases'a ikisini birden koyar, kaynak depoyu ve açık kaynak depoyu günceller.
# Kullanım: scripts/release.sh 1.1.0 "Kısa sürüm notu"
# Yalnızca Mac düzeltmesi (Windows aynı kalır): MEOWLY_MAC_ONLY=1 scripts/release.sh 1.1.1 "not"
# Kurulu uygulamalar günde bir kez latest.json / latest-windows.json'a bakıp sorar.
set -euo pipefail
VERSION=${1:?"sürüm gerekli, ör. 1.1.0"}
NOTES=${2:-""}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
SITE=${MEOWLY_SITE:-"$HOME/Documents/meowly-site"}
PUB=${MEOWLY_PUBLIC:-"$HOME/Documents/meowly-public"}
URL="https://meowly.vercel.app"
MAC_ONLY=${MEOWLY_MAC_ONLY:-""}
WTMP=$(mktemp -d)
trap 'rm -rf "$WTMP"' EXIT

# 0) Ön denetim: bir şey eksikse hiçbir şey derlenmez ya da yayınlanmaz.
[ -z "$(git -C "$ROOT" status --porcelain -- Sources Resources Package.swift)" ] || { echo "HATA: kaynakta commit edilmemiş değişiklik var"; exit 1; }
[ -z "$(git -C "$SITE" status --porcelain)" ] || { echo "HATA: site klasöründe commit edilmemiş değişiklik var"; exit 1; }
[ -d "$PUB/.git" ] && [ -z "$(git -C "$PUB" status --porcelain)" ] || { echo "HATA: açık kaynak deposu ($PUB) yok ya da temiz değil"; exit 1; }
git -C "$ROOT" fetch -q origin
if [ -z "$MAC_ONLY" ]; then
  # Windows kurulumu Windows bilgisayarda aynı sürüm numarasıyla derlenip windows-release dalına gönderilmiş olmalı.
  git -C "$ROOT" archive origin/windows-release site | tar -x -C "$WTMP"
  WJSON="$WTMP/site/latest-windows.json"
  WVER=$(/usr/bin/python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['version'])" "$WJSON")
  [ "$WVER" = "$VERSION" ] || { echo "HATA: windows-release dalındaki Windows sürümü $WVER, beklenen $VERSION. Windows'ta $VERSION derlenip gönderilmeli."; exit 1; }
  WSHA=$(/usr/bin/python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['sha256'].lower())" "$WJSON")
  for f in "Meowly-Setup-$VERSION.exe" "Meowly-Setup.exe"; do
    [ -f "$WTMP/site/download/$f" ] || { echo "HATA: Windows dosyası yok: $f"; exit 1; }
    [ "$(shasum -a 256 "$WTMP/site/download/$f" | awk '{print $1}')" = "$WSHA" ] || { echo "HATA: $f SHA-256 tutmuyor"; exit 1; }
  done
  echo "Windows $VERSION doğrulandı · sha256 $WSHA"
fi

# 1) Sürüm numarası
sed -i '' -E "s|(<key>CFBundleShortVersionString</key><string>)[^<]*(</string>)|\1${VERSION}\2|" "$ROOT/scripts/build_app.sh"

# 2) Derle, imzala, /Applications'a kur
"$ROOT/scripts/build_app.sh"

# 3) Zip (imza korunur) ve SHA-256
mkdir -p "$SITE/download"
ZIP="$SITE/download/Meowly.zip"
rm -f "$ZIP"
ditto -c -k --sequesterRsrc --keepParent /Applications/Meowly.app "$ZIP"
SHA=$(shasum -a 256 "$ZIP" | awk '{print $1}')
SIZE=$(stat -f %z "$ZIP")

# 4) Güncelleme bilgisi (eski sürümler bunu okuyup yeni sürümü indirir)
/usr/bin/python3 - "$SITE/latest.json" "$VERSION" "$URL/download/Meowly-$VERSION.zip" "$SHA" "$NOTES" "$SIZE" <<'EOF'
import json, sys
path, v, url, sha, notes, size = sys.argv[1:]
json.dump({"version": v, "url": url, "sha256": sha, "notes": notes, "size": int(size)}, open(path, "w"), ensure_ascii=False, indent=2)
EOF
# Sürüme özel kopya: önbellekte eski dosya kalsa bile güncelleyici doğru zip'i indirir.
cp "$ZIP" "$SITE/download/Meowly-$VERSION.zip"

# 4b) Windows dosyaları siteye (win.js sitenin kendi kopyasıdır, üzerine yazılmaz)
if [ -z "$MAC_ONLY" ]; then
  cp "$WTMP/site/download/Meowly-Setup-$VERSION.exe" "$WTMP/site/download/Meowly-Setup.exe" "$SITE/download/"
  cp "$WJSON" "$SITE/latest-windows.json"
fi

# 5) Siteyi yayına al
cd "$SITE"
git add -A && git commit -qm "sürüm $VERSION" || true
npx --yes vercel@latest deploy --prod --yes

# 6) GitHub Releases: Mac zip'i ve Windows kurulumu aynı sürümde
WVER=$(/usr/bin/python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['version'])" "$SITE/latest-windows.json")
WSHA=$(/usr/bin/python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['sha256'].lower())" "$SITE/latest-windows.json")
WEXE="$SITE/download/Meowly-Setup-$WVER.exe"
gh release create "v$VERSION" "$SITE/download/Meowly-$VERSION.zip#Meowly-$VERSION.zip" "$WEXE#Meowly-Setup-$WVER.exe" \
  --repo mertpural/meowly --title "Meowly $VERSION" \
  --notes "${NOTES:-Yeni sürüm.}

**Mac:** \`Meowly-$VERSION.zip\`'i indir, aç ve Meowly'yi Uygulamalar klasörüne taşı.
**Windows:** \`Meowly-Setup-$WVER.exe\`'yi çalıştır.
Kurulu Meowly kendini günceller.

SHA-256 (Mac): \`$SHA\`
SHA-256 (Windows): \`$WSHA\`

$URL"

# 7) Kaynak depo: sürüm numarası commit edilip gönderilir
git -C "$ROOT" add scripts/build_app.sh
git -C "$ROOT" commit -qm "sürüm $VERSION" || true
git -C "$ROOT" push -q origin main

# 8) Açık kaynak depo: kodun son hâli (geçmiş ve özel dosyalar olmadan)
"$ROOT/scripts/sync_public.sh" "$VERSION"

echo "Yayında: $VERSION · $URL · Mac sha256 $SHA · Windows $WVER sha256 $WSHA"
