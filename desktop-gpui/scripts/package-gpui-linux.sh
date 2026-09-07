#!/usr/bin/env bash
# Tmux GUI Desktop Packaging Script for Linux (GPUI)
# Port of web-term's desktop-gpui/scripts/package-linux.sh, 3 steps verbatim
# modulo binary names (webterm -> webtmux, backend -> tmux-gui-server),
# plus the Phase 7 runtime-closure note (Pitfall 5).
set -euo pipefail

echo "======================================================="
echo "Building Tmux GUI Desktop Distribution Bundle (Linux)"
echo "======================================================="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist/tmux-gui-linux-x64"

echo "Target distribution directory: ${DIST_DIR}"
mkdir -p "${DIST_DIR}"

echo "[1/3] Compiling Go backend with loopback binding..."
(
    cd "${ROOT_DIR}/be"
    go build -ldflags "-s -w" -o "${DIST_DIR}/tmux-gui-server" ./cmd/server
)

echo "[2/3] Compiling GPUI desktop client in release mode..."
cargo build --release --manifest-path "${ROOT_DIR}/desktop-gpui/Cargo.toml" --package webtmux

echo "[3/3] Assembling distribution bundle..."
cp "${ROOT_DIR}/desktop-gpui/target/release/webtmux" "${DIST_DIR}/webtmux"
chmod +x "${DIST_DIR}/webtmux" "${DIST_DIR}/tmux-gui-server"

# NOTE: no fonts/assets copy step - JetBrains Mono is `include_bytes!`
# compiled into the binary; no linuxdeploy/AppImage/deb per D4
# (plain dist dir + tarball only).

# Pitfall 5: the dev box has build-time headers that a clean target lacks.
# Record the runtime `.so` closure next to the bundle so the clean-distro
# smoke test can compare. No `.so` files are bundled (A5 - escalate only
# if the clean-distro smoke fails).
echo "[3/3+] Recording runtime shared-library closure..."
if command -v ldd >/dev/null 2>&1; then
    {
        echo "Runtime closure (recorded by package-gpui-linux.sh; compare on the clean target)"
        echo ""
        echo "== webtmux =="
        ldd "${DIST_DIR}/webtmux" || echo "(ldd failed for webtmux)"
        echo ""
        echo "== tmux-gui-server =="
        ldd "${DIST_DIR}/tmux-gui-server" || echo "(statically linked Go binary - no .so closure)"
    } > "${DIST_DIR}/RUNTIME-DEPS.txt" 2>&1 || true
else
    echo "(ldd not available on the build host - install libc-bin and re-run to record the closure)" > "${DIST_DIR}/RUNTIME-DEPS.txt"
fi

cat << 'EOF' > "${DIST_DIR}/README.txt"
Tmux GUI Desktop Client (Linux)
===============================
Launch with: ./webtmux

Architecture:
- Single launchable bundle with tmux-gui-server and webtmux side-by-side.
- The backend process is automatically supervised and bound strictly to loopback (127.0.0.1).
- Native GPU terminal rendering via Alacritty.
- Expected siblings beside webtmux: tmux-gui-server, README.txt, RUNTIME-DEPS.txt.
  If tmux-gui-server is missing the app falls back to dev-path probing;
  paranoid users can pin an absolute backend via settings `backend_path`.
- Fonts: JetBrains Mono is embedded in the binary - no assets directory needed.

Runtime dependencies (see RUNTIME-DEPS.txt for the exact closure):
- The bundle needs the distro's Wayland/X11 + GL runtime libraries.
  On Debian/Ubuntu install them with:
    sudo apt-get install -y libfontconfig1 libwayland-client0 libxkbcommon0 \
      libx11-6 libxcursor1 libxrandr2 libgl1 libvulkan1 tmux
  If the clean-distro smoke test fails on a missing `.so`, record it here
  and escalate to bundling `.so` files (A5).
EOF

echo ""
echo "======================================================="
echo "Tmux GUI Linux Bundle assembled successfully!"
echo "Location: ${DIST_DIR}"
ls -la "${DIST_DIR}"
echo "======================================================="
