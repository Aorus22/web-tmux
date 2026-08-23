# Tmux GUI — native GTK desktop

This directory contains the third frontend for Tmux GUI. It uses GTK 4,
libadwaita, and a libvterm-backed drawing widget; it does not embed the React
frontend or a browser runtime. The existing web and Electron frontends remain
unchanged and continue to use the same Go backend protocol.

## Development

Linux requires GTK 4, libadwaita, libvterm and `pkg-config` development files.
On Windows, install the MSYS2 MINGW64 packages `mingw-w64-x86_64-gtk4`,
`mingw-w64-x86_64-libadwaita`, `mingw-w64-x86_64-libvterm`, and `mingw-w64-x86_64-toolchain`.

```bash
# Linux
./desktop-gtk/scripts/dev-linux.sh

# Windows PowerShell (MSYS2 default: C:\msys64)
./desktop-gtk/scripts/dev-windows.ps1
```

## Release artifacts

```bash
# Windows portable ZIP
pwsh ./desktop-gtk/scripts/build-windows.ps1

# Linux AppImage (when linuxdeploy is installed) and .deb
./desktop-gtk/scripts/build-linux.sh
```

The native frontend starts `tmux-gui-server` as a localhost sidecar, stores
settings under the platform config directory, and can also attach to an
already-running server with `--no-backend --port <port>`.
