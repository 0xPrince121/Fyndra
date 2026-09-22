#!/usr/bin/env bash
set -e

: "${HOME:?HOME is not set}"

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ─────────────────────────────────────────────────────────────
# Fyndra Installer UI
# ─────────────────────────────────────────────────────────────

CYAN="\033[1;36m"
GREEN="\033[1;32m"
YELLOW="\033[1;33m"
DIM="\033[2m"
RESET="\033[0m"

printf "\n"

printf "${CYAN}"
printf "┌────────────────────────────────────────────────────────────┐\n"
printf "│                                                            │\n"
printf "│   ███████╗██╗   ██╗███╗   ██╗██████╗ ██████╗  █████╗       │\n"
printf "│   ██╔════╝╚██╗ ██╔╝████╗  ██║██╔══██╗██╔══██╗██╔══██╗      │\n"
printf "│   █████╗   ╚████╔╝ ██╔██╗ ██║██║  ██║██████╔╝███████║      │\n"
printf "│   ██╔══╝    ╚██╔╝  ██║╚██╗██║██║  ██║██╔══██╗██╔══██║      │\n"
printf "│   ██║        ██║   ██║ ╚████║██████╔╝██║  ██║██║  ██║      │\n"
printf "│   ╚═╝        ╚═╝   ╚═╝  ╚═══╝╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝      │\n"
printf "│                                                            │\n"
printf "└────────────────────────────────────────────────────────────┘\n"
printf "${RESET}"

printf "\n"
printf "${CYAN}Installing Fyndra...${RESET}\n"
printf "${DIM}Engineered from scratch by 0xPrince${RESET}\n\n"


# ─────────────────────────────────────────────────────────────
# Progress renderer
# ─────────────────────────────────────────────────────────────

progress() {
    local percent="$1"
    local status="$2"
    local width=40

    local filled=$((percent * width / 100))
    local empty=$((width - filled))

    printf "\r\033[2K"
    printf "${CYAN}["
    for ((i = 0; i < filled; i++)); do printf '█'; done
    for ((i = 0; i < empty; i++)); do printf '░'; done
    printf "]${RESET} ${CYAN}%3d%%${RESET} ${DIM}%s${RESET}" "$percent" "$status"
}


# ─────────────────────────────────────────────────────────────
# Build if binaries are missing
# ─────────────────────────────────────────────────────────────

if [ ! -f "$DIR/target/release/fyndra-gui" ] || \
   [ ! -f "$DIR/target/release/fyndra-cli" ]; then

    progress 1 "Preparing build environment..."

    progress 10 "Compiling Fyndra..."
    cargo build --release --manifest-path "$DIR/Cargo.toml"

    progress 65 "Release binaries built."
else
    progress 65 "Release binaries already available."
fi


# ─────────────────────────────────────────────────────────────
# Install binaries
# ─────────────────────────────────────────────────────────────

progress 72 "Installing Fyndra binaries..."

mkdir -p "$HOME/.local/bin"

cp -f "$DIR/target/release/fyndra-gui" \
      "$HOME/.local/bin/"

cp -f "$DIR/target/release/fyndra-cli" \
      "$HOME/.local/bin/"

chmod +x \
    "$HOME/.local/bin/fyndra-gui" \
    "$HOME/.local/bin/fyndra-cli"

progress 82 "Binaries installed."


# ─────────────────────────────────────────────────────────────
# Install desktop shortcut
# ─────────────────────────────────────────────────────────────

progress 86 "Installing desktop integration..."

mkdir -p "$HOME/.local/share/applications"

APP_DESKTOP="io.github.prince121.fyndra.desktop"

{
    grep -v '^Exec=' "$DIR/fyndra.desktop" || true
    printf 'Exec=%s\n' "$HOME/.local/bin/fyndra-gui"
} > "$HOME/.local/share/applications/$APP_DESKTOP"

chmod +x "$HOME/.local/share/applications/$APP_DESKTOP"

progress 93 "Desktop entry installed."


# ─────────────────────────────────────────────────────────────
# Update desktop database
# ─────────────────────────────────────────────────────────────

progress 97 "Updating application database..."

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database \
        "$HOME/.local/share/applications" \
        2>/dev/null || true
fi


# ─────────────────────────────────────────────────────────────
# Check PATH
# ─────────────────────────────────────────────────────────────

case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) PATH_WARNING=1 ;;
esac


# ─────────────────────────────────────────────────────────────
# Complete
# ─────────────────────────────────────────────────────────────

progress 100 "Installation complete."

printf "\n\n"
printf "${GREEN}✓ Fyndra installed successfully!${RESET}\n"

if [ -n "${PATH_WARNING:-}" ]; then
    printf "${YELLOW}⚠  %s/.local/bin is not in your PATH.${RESET}\n" "$HOME"
    printf "${YELLOW}   Add it with: echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc${RESET}\n"
    printf "${YELLOW}   You can still launch 'Fyndra' from your app menu immediately.${RESET}\n"
else
    printf "${DIM}Run 'fyndra-cli' in your terminal or launch 'Fyndra' from your app menu.${RESET}\n"
fi

printf "\n"
