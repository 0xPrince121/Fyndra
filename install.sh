#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Installing Fyndra..."

# Build if binaries missing
if [ ! -f "$DIR/target/release/fyndra-gui" ] || [ ! -f "$DIR/target/release/fyndra-cli" ]; then
    echo "Building release binaries..."
    cargo build --release --manifest-path "$DIR/Cargo.toml"
fi

# Install binaries to ~/.local/bin
mkdir -p "$HOME/.local/bin"
cp -f "$DIR/target/release/fyndra-gui" "$HOME/.local/bin/"
cp -f "$DIR/target/release/fyndra-cli" "$HOME/.local/bin/"
chmod +x "$HOME/.local/bin/fyndra-gui" "$HOME/.local/bin/fyndra-cli"

# Install desktop shortcut (reverse-DNS ID)
mkdir -p "$HOME/.local/share/applications"
APP_DESKTOP="io.github.prince121.fyndra.desktop"
sed "s|Exec=.*|Exec=$HOME/.local/bin/fyndra-gui|g" "$DIR/fyndra.desktop" > "$HOME/.local/share/applications/$APP_DESKTOP"
chmod +x "$HOME/.local/share/applications/$APP_DESKTOP"

# Update desktop database
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true

echo "Installation complete!"
echo "Run 'fyndra-cli' in your terminal or launch 'Fyndra' from your app menu."
