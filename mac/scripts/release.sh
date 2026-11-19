#!/bin/zsh
# Yeni sürüm yayınlar: sürümü yazar, uygulamayı derleyip imzalar, zip'ler, siteye koyar ve siteyi yayına alır.
# Kullanım: scripts/release.sh 1.0.1 "Kısa sürüm notu"
# Sonraki her güncellemede yalnızca bu komut çalıştırılır; kurulu uygulamalar günde bir kez latest.json'a bakıp sorar.
set -euo pipefail
VERSION=${1:?"sürüm gerekli, ör. 1.0.1"}
NOTES=${2:-""}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
SITE=${MEOWLY_SITE:-"$HOME/Documents/meowly-site"}
URL="https://meowly.vercel.app"

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

# 5) Siteyi yayına al
cd "$SITE"
git add -A && git commit -qm "sürüm $VERSION" || true
npx --yes vercel@latest deploy --prod --yes

# 6) GitHub'daki kodsuz depoya da sürüm olarak eklenir (Releases bölümü).
gh release create "v$VERSION" "$SITE/download/Meowly-$VERSION.zip#Meowly-$VERSION.zip" --repo mertpural/meowly \
  --title "Meowly $VERSION" \
  --notes "${NOTES:-Yeni sürüm.}

**Kurulum:** zip'i indir, aç ve Meowly'yi Uygulamalar klasörüne taşı. Kurulu Meowly kendini günceller.

SHA-256: \`$SHA\`

$URL"
# 7) Windows kurulum dosyası varsa aynı sürüme eklenir (site klasöründe latest-windows.json ile birlikte durur).
if [ -f "$SITE/latest-windows.json" ]; then
  WVER=$(/usr/bin/python3 -c "import json,sys;print(json.load(open(sys.argv[1]))['version'])" "$SITE/latest-windows.json")
  WEXE="$SITE/download/Meowly-Setup-$WVER.exe"
  [ -f "$WEXE" ] && gh release upload "v$VERSION" "$WEXE#Meowly-Setup-$WVER.exe" --repo mertpural/meowly
fi
echo "Yayında: $VERSION · $URL · sha256 $SHA"
