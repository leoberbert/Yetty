#!/usr/bin/env bash
# Builds dist/yetty-v<versão>-linux-x86_64.AppImage a partir de target/release/yetty.
# Uso: packaging/linux/build-appimage.sh [versão]   (padrão: a de Cargo.toml)
# Precisa de `appimagetool` no PATH (ou em $APPIMAGETOOL). Sem FUSE, defina APPIMAGE_EXTRACT_AND_RUN=1.
# O binário não depende de libssl (OpenSSL embutido); janela, teclado e fontes vêm das bibliotecas do sistema
# (wayland/x11/xkbcommon são carregadas em tempo de execução), por isso nada é empacotado além do executável.
set -euo pipefail
cd "$(dirname "$0")/../.."

VERSION="${1:-$(grep -m1 '^version' Cargo.toml | sed -E 's/version *= *"([^"]+)"/\1/')}"
TOOL="${APPIMAGETOOL:-appimagetool}"
BIN=target/release/yetty
[ -x "$BIN" ] || { echo "falta $BIN (rode: cargo build --release --locked)" >&2; exit 1; }

APPDIR="$(mktemp -d)/Yetty.AppDir"
trap 'rm -rf "$(dirname "$APPDIR")"' EXIT
mkdir -p "$APPDIR/usr/bin" dist
install -m755 "$BIN" "$APPDIR/usr/bin/yetty"
install -m644 packaging/linux/yetty.desktop "$APPDIR/yetty.desktop"
install -m644 assets/yetty.png "$APPDIR/yetty.png"
ln -s yetty.png "$APPDIR/.DirIcon"

# AppRun só chama o binário: não define LD_LIBRARY_PATH nem PATH, então ssh e o shell local herdam o ambiente limpo.
cat > "$APPDIR/AppRun" <<'R'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/yetty" "$@"
R
chmod 755 "$APPDIR/AppRun"

OUT="dist/yetty-v${VERSION}-linux-x86_64.AppImage"
ARCH=x86_64 "$TOOL" "$APPDIR" "$OUT"
(cd dist && sha256sum "$(basename "$OUT")" > "$(basename "$OUT").sha256")
echo "$OUT"
