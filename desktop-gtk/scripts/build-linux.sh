#!/usr/bin/env bash
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
version="${VERSION:-0.1.0}"
arch="${ARCH:-$(dpkg --print-architecture 2>/dev/null || uname -m)}"
out="$repo/dist/gtk"
appdir="$out/AppDir"
debroot="$out/debroot"

for pc in gtk4 libadwaita-1 vterm; do pkg-config --exists "$pc" || { echo "missing pkg-config package: $pc" >&2; exit 1; }; done
rm -rf "$appdir" "$debroot"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" "$appdir/usr/share/icons/hicolor/scalable/apps" "$appdir/usr/share/metainfo"

(cd "$repo/be" && CGO_ENABLED=0 go build -trimpath -ldflags='-s -w' -o "$appdir/usr/bin/tmux-gui-server" ./cmd/server)
(cd "$repo/desktop-gtk" && CGO_ENABLED=1 go build -trimpath -ldflags='-s -w' -o "$appdir/usr/bin/tmux-gui-gtk" .)
install -Dm644 "$repo/desktop-gtk/packaging/linux/dev.tmuxgui.gtk.desktop" "$appdir/usr/share/applications/dev.tmuxgui.gtk.desktop"
install -Dm644 "$repo/desktop-gtk/assets/dev.tmuxgui.gtk.svg" "$appdir/usr/share/icons/hicolor/scalable/apps/dev.tmuxgui.gtk.svg"
install -Dm644 "$repo/desktop-gtk/packaging/linux/dev.tmuxgui.gtk.metainfo.xml" "$appdir/usr/share/metainfo/dev.tmuxgui.gtk.metainfo.xml"

if command -v linuxdeploy >/dev/null 2>&1; then
  (cd "$out" && OUTPUT="tmux-gui-gtk-${version}-${arch}.AppImage" linuxdeploy --appdir "$appdir" --executable "$appdir/usr/bin/tmux-gui-gtk" --desktop-file "$appdir/usr/share/applications/dev.tmuxgui.gtk.desktop" --icon-file "$appdir/usr/share/icons/hicolor/scalable/apps/dev.tmuxgui.gtk.svg" --output appimage)
else
  echo "linuxdeploy not found; AppDir is ready at $appdir" >&2
fi

mkdir -p "$debroot/DEBIAN" "$debroot/usr/bin" "$debroot/usr/share/applications" "$debroot/usr/share/icons/hicolor/scalable/apps" "$debroot/usr/share/metainfo"
cp "$appdir/usr/bin/tmux-gui-gtk" "$appdir/usr/bin/tmux-gui-server" "$debroot/usr/bin/"
cp "$appdir/usr/share/applications/dev.tmuxgui.gtk.desktop" "$debroot/usr/share/applications/"
cp "$appdir/usr/share/icons/hicolor/scalable/apps/dev.tmuxgui.gtk.svg" "$debroot/usr/share/icons/hicolor/scalable/apps/"
cp "$appdir/usr/share/metainfo/dev.tmuxgui.gtk.metainfo.xml" "$debroot/usr/share/metainfo/"
cat > "$debroot/DEBIAN/control" <<EOF
Package: tmux-gui-gtk
Version: $version
Architecture: $arch
Maintainer: Tmux GUI <tmux-gui@localhost>
Depends: tmux (>= 3.2), libgtk-4-1, libadwaita-1-0, libvterm0 (>= 0.3)
Section: utils
Priority: optional
Description: Native GTK control surface for tmux
 Manage sessions, windows, panes, layouts and terminals with libadwaita.
EOF
dpkg-deb --build --root-owner-group "$debroot" "$out/tmux-gui-gtk_${version}_${arch}.deb"
echo "Linux packages are in $out"
