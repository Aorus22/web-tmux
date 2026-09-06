# Building Tmux GUI (desktop-gpui)

This document covers prerequisites and build instructions for `desktop-gpui`, the GPUI-based desktop frontend for `web-tmux`.

---

## 1. Prerequisites

### Windows Prerequisites
1. **Rust Toolchain:**
   - Rust 1.80+ (MSVC target: `x86_64-pc-windows-msvc`).
   - Installed via [rustup](https://rustup.rs/).
2. **Visual Studio C++ Build Tools:**
   - MSVC C++ compiler and Windows SDK / Linker (from Visual Studio or VS Build Tools).
3. **Go Toolchain:**
   - Go 1.22+ to build the `tmux-gui-server` sidecar binary (`make be` or `go build ./cmd/server` under `be/`).
4. **HLSL Shader Compiler (`d3dcompiler_47.dll` / `tools/fxc`):**
   - Release builds require compiling HLSL shaders. Windows includes `d3dcompiler_47.dll` out-of-the-box in `C:\Windows\System32`.
   - The standalone `tools/fxc` drop-in tool compiles directly with `rustc` without needing the multi-gigabyte Windows 10/11 SDK.

### Linux Prerequisites
On Debian/Ubuntu-based distributions, install the required development headers and system libraries:

```bash
sudo apt-get update && sudo apt-get install -y \
  build-essential \
  pkg-config \
  libfontconfig1-dev \
  libwayland-dev \
  libxkbcommon-dev \
  libxkbcommon-x11-dev \
  libx11-dev \
  libxcursor-dev \
  libxrandr-dev \
  libgl1-mesa-dev \
  vulkan-loader \
  tmux
```

---

## 2. Windows Build Instructions

### Debug Build
Debug builds do not require pre-compiled shaders (`gpui-pre-windows` compiles shaders only in release configurations):

```powershell
# Build the entire workspace in debug mode
cargo build --manifest-path desktop-gpui/Cargo.toml

# Run the app
cargo run --manifest-path desktop-gpui/Cargo.toml -p webtmux
```

### Release Build
For release builds, compile the standalone `tools/fxc` tool and set `GPUI_FXC_PATH` before invoking `cargo build --release`:

```powershell
# 1. Compile the fxc shader compiler tool (zero external dependencies)
rustc -O desktop-gpui/tools/fxc/main.rs -o desktop-gpui/tools/fxc/fxc.exe

# 2. Point GPUI_FXC_PATH to the compiled fxc.exe
$env:GPUI_FXC_PATH = (Resolve-Path desktop-gpui/tools/fxc/fxc.exe).Path

# 3. Build the release binary
cargo build --release --manifest-path desktop-gpui/Cargo.toml --package webtmux
```

The compiled release binary will be available at `desktop-gpui/target/release/webtmux.exe`.

---

## 3. Linux Build Instructions

Ensure all `apt-get` packages listed above (specifically `libfontconfig1-dev`, `libwayland-dev`, `libxkbcommon-dev`, and graphics libraries) are installed.

```bash
# Debug build
cargo build --manifest-path desktop-gpui/Cargo.toml

# Release build
cargo build --release --manifest-path desktop-gpui/Cargo.toml --package webtmux

# Run the application
cargo run --manifest-path desktop-gpui/Cargo.toml -p webtmux
```

---

## 4. Running Workspace Tests

To run the automated headless unit and integration tests across the workspace:

```bash
# Run tests for all workspace crates (supervisor, settings, webtmux)
cargo test --manifest-path desktop-gpui/Cargo.toml --workspace
```
