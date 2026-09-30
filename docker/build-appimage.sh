#!/usr/bin/env bash
# In-container steps for the Tmux GUI AppImage.
#
# Split of responsibilities:
#   docker/Dockerfile.build  -> toolchain + system headers
#   this script              -> the steps, run against the MOUNTED repo (/src)
#   desktop-gpui/scripts/package-appimage.sh -> the actual packaging (reused)
#
# Usage (from the repo root):
#   docker run --rm -v "$PWD:/src" -w /src webtmux-build bash docker/build-appimage.sh
#
# Output: dist/Tmux-GUI-<version>-x86_64.AppImage (+ dist/SHA256SUMS.txt)
set -euo pipefail

ROOT_DIR="${ROOT_DIR:-/src}"
cd "${ROOT_DIR}"

# [1/3] Frontend — the Go server `go:embed all:dist`s this directory, so a
# build without it fails outright. Vite writes straight to
# be/internal/web/dist (fe/vite.config.ts `build.outDir`).
echo "[1/3] Building frontend (npm ci + vite build)..."
(
    cd "${ROOT_DIR}/fe"
    npm ci --no-audit --no-fund
    npm run build
)

# [2/3] + [3/3] Go backend, GPUI client, AppDir, appimagetool — all in the
# repo's own packaging script (APPIMAGETOOL is baked into the image env).
echo "[2/3] Building Go backend + GPUI release client + AppImage..."
"${ROOT_DIR}/desktop-gpui/scripts/package-appimage.sh"

echo "[3/3] Checksums..."
cd "${ROOT_DIR}"
sha256sum dist/Tmux-GUI-*.AppImage | tee dist/SHA256SUMS.txt

echo
echo "Done. Artifacts:"
ls -la dist/Tmux-GUI-*.AppImage
