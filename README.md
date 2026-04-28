# 👶 Yacito

<p align="center">
  <img src="static/yacito-logo.png" alt="Yacito logo" width="180" />
</p>

The adorable, interactive, and **baby-easy** GUI for [httpYac](https://httpyac.github.io/).

Yacito (from the Sanskrit *yācito*, meaning "requested") is a user-friendly desktop interface built on top of the powerful httpYac engine. It’s designed for developers who want the full power of text-based HTTP files with the comfort of a visual, interactive workspace. Making complex API requests is now child's play.

## 🍼 Quick Install

Get up and running in seconds. These scripts install Rust, system dependencies, and project requirements.

### Windows (PowerShell)
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/guidonaselli/yacito/main/scripts/install-windows.ps1'))
```

### WSL / Linux (Ubuntu/Debian)
```bash
curl -sSL https://raw.githubusercontent.com/guidonaselli/yacito/main/scripts/install-wsl.sh | bash
```

## 🛠️ Manual Setup

If you prefer to do it yourself:

1.  **Clone the repo**: `git clone https://github.com/guidonaselli/yacito`
2.  **Install dependencies**: `npm install`
3.  **Setup httpYac**: `npm run setup:httpyac`
4.  **Run it**: `npm run dev:app`

## ✨ Features

- **Visual Feedback**: Real-time response visualization.
- **Runes Powered**: Built with Svelte 5 for lightning-fast reactivity.
- **Configurable UI**: Style variables based on OKLCH for consistent, beautiful themes.
- **i18n Support**: Available in English and Spanish.
- **Baby-Easy**: No steep learning curves. Just open your `.http` files and start requesting.

## 📦 Building for Production

To generate a standalone `.exe` or `.msi`:

```bash
npm run build:app
```
The artifacts will be generated in `src-tauri/target/release/bundle/`.

## 👶 Why Yacito?

Apart from being the "baby" version of httpYac, the word **Yacito** (*yācito*) literally means "requested" or "asked" in ancient Sanskrit and Pali. It’s the perfect name for a tool dedicated to making HTTP requests!

---
*Yacito is not an official httpyac project. It is a companion GUI that uses the httpyac CLI as its request engine.*

Made with 🍼 and ☕ for the developer community.
