#!/usr/bin/env bash
# Tmux GUI AppImage packaging (GPUI desktop, Linux x86_64).
# Reuses package-gpui-linux.sh for the binaries, then assembles an AppDir
# (client + Go backend side-by-side, logo, .desktop) and runs appimagetool.
#
# Usage:
#   ./desktop-gpui/scripts/package-appimage.sh
#   APPIMAGETOOL=/path/to/appimagetool ./desktop-gpui/scripts/package-appimage.sh
#
# Output: dist/Tmux-GUI-<version>-x86_64.AppImage
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BUNDLE_DIR="${ROOT_DIR}/dist/tmux-gui-linux-x64"
APPDIR="${ROOT_DIR}/dist/Tmux.GUI.AppDir"
DIST_DIR="${ROOT_DIR}/dist"

VERSION="$(grep -m1 '^version' "${ROOT_DIR}/desktop-gpui/Cargo.toml" | cut -d'"' -f2)"
ARCH="$(uname -m)"
OUT="${DIST_DIR}/Tmux-GUI-${VERSION}-${ARCH}.AppImage"

APPIMAGETOOL="${APPIMAGETOOL:-appimagetool}"

echo "[1/3] Building bundle via package-gpui-linux.sh..."
"${ROOT_DIR}/desktop-gpui/scripts/package-gpui-linux.sh" >/dev/null
echo "      bundle ready: ${BUNDLE_DIR}"

echo "[2/3] Assembling AppDir..."
rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin"
cp "${BUNDLE_DIR}/webtmux" "${APPDIR}/usr/bin/webtmux"
cp "${BUNDLE_DIR}/tmux-gui-server" "${APPDIR}/usr/bin/tmux-gui-server"
chmod +x "${APPDIR}/usr/bin/webtmux" "${APPDIR}/usr/bin/tmux-gui-server"
cp "${ROOT_DIR}/desktop-gpui/webtmux-gpui.desktop" "${APPDIR}/webtmux-gpui.desktop"
cp "${ROOT_DIR}/desktop-gpui/assets/icons/hicolor/256x256/apps/webtmux-gpui.png" "${APPDIR}/webtmux-gpui.png"
ln -sf webtmux-gpui.png "${APPDIR}/.DirIcon"
cat > "${APPDIR}/AppRun" << 'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/webtmux" "$@"
EOF
chmod +x "${APPDIR}/AppRun"

echo "[3/3] Running appimagetool..."
rm -f "${OUT}"
ARCH="${ARCH}" "${APPIMAGETOOL}" "${APPDIR}" "${OUT}"
ls -la "${OUT}"
