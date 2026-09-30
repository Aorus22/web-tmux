# Docker build environments

Reproducible toolchains for the Tmux GUI desktop (GPUI) client. Every build in
CI goes through these images, so a CI failure is a code failure — never a
"worked on my machine" toolchain difference.

| File | Purpose | Extras on top of the Rust base |
| --- | --- | --- |
| `Dockerfile.build` | Builds the Linux AppImage | Go 1.27, Node 24, appimagetool, full window/GL/font headers |
| `Dockerfile.test` | Runs `cargo test --workspace` | window/GL/font headers only |

Both are **environment-only**: they never `COPY` the source. The repo is
mounted at `/src` at run time, which keeps the images cacheable across commits
and lets the repo own the build steps (`docker/build-appimage.sh` and
`desktop-gpui/scripts/package-appimage.sh`).

## Build the AppImage

```bash
docker build -f docker/Dockerfile.build -t webtmux-build .
docker run --rm -v "$PWD:/src" -w /src webtmux-build bash docker/build-appimage.sh
# -> dist/Tmux-GUI-<version>-x86_64.AppImage
```

Useful mounts to keep caches warm between runs (all optional):

```bash
docker run --rm -v "$PWD:/src" -w /src \
  -v "$HOME/.cargo/registry:/usr/local/cargo/registry" \
  -v "$HOME/.cargo/git:/usr/local/cargo/git" \
  -v "$HOME/go/pkg/mod:/root/go/pkg/mod" \
  -v "$HOME/.npm:/root/.npm" \
  webtmux-build bash docker/build-appimage.sh
```

## Run the tests

```bash
docker build -f docker/Dockerfile.test -t webtmux-test .
docker run --rm -v "$PWD:/src" -w /src/desktop-gpui webtmux-test \
  cargo test --workspace --locked
```

## Why Debian bookworm

The AppImage must run on the maintainer's Fedora and on Ubuntu alike. Building
against bookworm's glibc 2.36 (older than both) means the produced binary only
ever references symbols every target already exports; the GTK3 /
fontconfig / X11 / Wayland / Vulkan libraries it links are resolved from the
host at run time by SONAME.
