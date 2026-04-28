#!/bin/bash
set -e

echo "🍼 Yacito WSL Quick Installer"
echo "----------------------------"

# System Dependencies for Tauri
echo "📦 Installing system dependencies..."
sudo apt update
sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

# Rust
if ! command -v rustc &> /dev/null; then
    echo "🦀 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "✓ Rust already installed"
fi

# Node.js check
if ! command -v node &> /dev/null; then
    echo "❌ Node.js not found. Please install it using your preferred method (e.g., nvm)."
    exit 1
fi

# Project setup
echo "🚀 Building Yacito (Production)..."
npm install
npm run setup:httpyac
npm run build:app

# Add alias to the production binary
BINARY_PATH="$PWD/src-tauri/target/release/yacito"
if ! grep -q "alias yacito=" ~/.bashrc; then
    echo "alias yacito='$BINARY_PATH'" >> ~/.bashrc
    echo "✅ Alias 'yacito' added to ~/.bashrc"
fi

echo ""
echo "✨ Installation complete!"
echo "The productive version of Yacito is ready."
echo "Please RESTART your terminal or run 'source ~/.bashrc'."
echo "Then, you can start the app by simply typing: yacito"
