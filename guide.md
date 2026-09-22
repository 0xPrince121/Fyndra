# Fyndra — Full Build Guide

Detailed step-by-step guide for building and installing **Fyndra** on **Linux** and **macOS**.
This is the companion guide — for the quick overview, features, and CLI reference see the [**README**](README.md).

---

## 📑 Contents

- [1. Quick Reference](#1-quick-reference)
- [2. Install the Rust Toolchain](#2-install-the-rust-toolchain)
- [3. Linux Build Dependencies](#3-linux-build-dependencies)
- [4. macOS Build Dependencies](#4-macos-build-dependencies)
- [5. Build Fyndra](#5-build-fyndra)
- [6. Install Fyndra](#6-install-fyndra)
- [7. Run Fyndra](#7-run-fyndra)
- [8. Tests & Benchmarks](#8-tests--benchmarks)
- [9. Troubleshooting](#9-troubleshooting)

---

## 1. Quick Reference

| Action | Command |
|--------|---------|
| Install Rust | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Build (debug) | `cargo build` |
| Build (release) | `cargo build --release` |
| Only CLI | `cargo build --release -p fyndra-cli` |
| Only GUI | `cargo build --release -p fyndra-gui` |
| Run (no install) | `./target/release/fyndra-cli "query"` / `./target/release/fyndra-gui` |
| Auto-install | `./install.sh` |
| Tests | `cargo test --workspace` |
| Benchmarks | `cargo bench -p fyndra-core` |

> **Minimum Rust version: 1.92+.** The `gtk4 0.11` / `glib 0.22` crates require it — `rustup update stable` always stays safe.

---

## 2. Install the Rust Toolchain

Required on **both** Linux and macOS. One command installs `rustc` + `cargo` + `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"     # apply to the current shell, or open a new terminal
```

Verify:

```bash
rustc --version && cargo --version
# rustc 1.92+ · cargo 1.92+
```

Already installed but old?

```bash
rustup update stable
```

---

## 3. Linux Build Dependencies

### Ubuntu / Debian / Mint

```bash
sudo apt update && sudo apt install -y \
  build-essential pkg-config git curl \
  libgtk-4-dev libadwaita-1-dev
```

### Fedora

```bash
sudo dnf install -y \
  gcc make pkgconf-pkg-config git curl \
  gtk4-devel libadwaita-devel
```

### Arch / Manjaro

```bash
sudo pacman -S --needed \
  base-devel pkgconf git curl \
  gtk4 libadwaita
```

### openSUSE

```bash
sudo zypper install -y \
  gcc make pkg-config git curl \
  gtk4-devel libadwaita-devel
```

### Verify Linux dependencies

```bash
pkg-config --modversion gtk4 && pkg-config --modversion libadwaita-1
# gtk4 >= 4.0 · libadwaita-1 >= 1.0
```

> **CLI-only build?** GTK is only needed for the GUI. For `fyndra-cli`, Rust + a C toolchain (`build-essential` / `base-devel`) is enough — skip `gtk4`/`libadwaita` packages.

---

## 4. macOS Build Dependencies

macOS ships its C toolchain via the **Command Line Tools** (`cc`, `ld`, `make`):

```bash
xcode-select --install
```

Install the GUI stack with [Homebrew](https://brew.sh):

```bash
brew install pkg-config gtk4 libadwaita git curl
```

> **CLI-only on macOS?** Skip Homebrew — `xcode-select --install` alone is enough for `fyndra-cli`.
> The `.desktop` / app-menu integration is Linux-only; on macOS launch the GUI with `open ~/.local/bin/fyndra-gui`.

---

## 5. Build Fyndra

Clone the repository:

```bash
git clone https://github.com/0xPrince121/Fyndra.git
cd Fyndra
```

> Debug vs release: debug compiles fast but runs slower; release is optimized and recommended.

```bash
# Debug (fast compile, slower runtime)
cargo build

# Release (optimized — recommended)
cargo build --release

# Only one binary
cargo build --release -p fyndra-cli     # terminal tool only
cargo build --release -p fyndra-gui     # desktop app only
```

Outputs:

```text
target/release/fyndra-cli
target/release/fyndra-gui
```

> First build takes a few minutes (downloads + compiles ~300 crates). Later builds are incremental and fast.

---

## 6. Install Fyndra

### Option A — Automated installer (`install.sh`)

```bash
./install.sh
```

What it does:

1. Builds release binaries automatically if they're missing
2. Copies `fyndra-cli` + `fyndra-gui` into `~/.local/bin`
3. Installs `io.github.prince121.fyndra.desktop` into `~/.local/share/applications`
4. Refreshes the desktop database (skipped if `update-desktop-database` is unavailable)
5. Warns if `~/.local/bin` is not on your `PATH`

Add `~/.local/bin` to your `PATH` if needed:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

> The installer is fully automated — build, install, desktop entry, PATH check.

### Option B — Manual install

```bash
cargo build --release

mkdir -p ~/.local/bin
cp -f target/release/fyndra-cli ~/.local/bin/
cp -f target/release/fyndra-gui ~/.local/bin/
chmod +x ~/.local/bin/fyndra-*

# Desktop entry (Linux only — safe, no sed injection)
mkdir -p ~/.local/share/applications
{
  grep -v '^Exec=' fyndra.desktop || true
  printf 'Exec=%s\n' "$HOME/.local/bin/fyndra-gui"
} > ~/.local/share/applications/io.github.prince121.fyndra.desktop
update-desktop-database ~/.local/share/applications 2>/dev/null || true
```

---

## 7. Run Fyndra

### CLI

```bash
fyndra-cli --help

fyndra-cli "invoice"                          # search your home dir
fyndra-cli "report" -p ~/Docs,/tmp            # custom roots
fyndra-cli ".*\.rs$" -r -l 20                 # regex, top 20
fyndra-cli "photo*.jpg" -w --files            # wildcard, files only
fyndra-cli "test" --benchmark                 # scan + search timing
fyndra-cli "TODO" -c --json | jq              # JSON for jq/fzf
```

### GUI

```bash
fyndra-gui                                  # Linux launch
fyndra-gui &                                # background
open ~/.local/bin/fyndra-gui                # macOS (full path)
```

No install needed — run straight from the build directory:

```bash
./target/release/fyndra-cli "notes"
./target/release/fyndra-gui
```

---

## 8. Tests & Benchmarks

```bash
cargo test --workspace                  # unit + integration tests
cargo bench -p fyndra-core              # criterion benches: crawl + search
```

Quality gates (run before opening a PR):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## 9. Troubleshooting

| Problem | Fix |
|---------|-----|
| `error: package gtk4 requires rustc 1.92` | `rustup update stable` |
| `Package 'gtk4' not found` / `pkg-config: not found` | Install the GUI deps for your distro (see [§3](#3-linux-build-dependencies)) |
| `libadwaita-1 requires at least version 1.0` | Distro's libadwaita is too old — upgrade via the [§3](#3-linux-build-dependencies) commands |
| `cannot find -lbz2` / linker errors (macOS) | Command Line Tools missing/incomplete — rerun `xcode-select --install` |
| `os error 24: Too many open files` (indexing) | Raise the limit: `ulimit -n 8192` |
| `inotify: No space left on device` (watcher) | `sudo sysctl fs.inotify.max_user_watches=524288` |
| Installer says `~/.local/bin is not in your PATH` | `echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc` |
| `update-desktop-database: command not found` | Optional tool — install `desktop-file-utils` or ignore |

---

**All set?** See the [**README**](README.md) for features, GUI guide, CLI reference, and project details.