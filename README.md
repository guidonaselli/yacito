# 👶 Yacito

[![CI](https://github.com/guidonaselli/yacito/actions/workflows/ci.yml/badge.svg)](https://github.com/guidonaselli/yacito/actions/workflows/ci.yml)

<p align="center">
  <img src="static/yacito-logo.png" alt="Yacito logo" width="180" />
</p>

The adorable, interactive, and **baby-easy** GUI for [httpYac](https://httpyac.github.io/).

Yacito (from the Sanskrit *yācito*, meaning "requested") is a user-friendly desktop interface built on top of the powerful httpYac engine. It’s designed for developers who want the full power of text-based HTTP files with the comfort of a visual, interactive workspace. Making complex API requests is now child's play.

## 🍼 Quick Install

Get up and running from source in seconds. These scripts install Rust, system dependencies, httpYac, project requirements, build the production desktop app, and create a convenient launcher/shortcut when supported.

### Windows (PowerShell)
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/guidonaselli/yacito/main/scripts/install-windows.ps1?v=' + [guid]::NewGuid()))
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
4.  **Validate your environment**: `npm run doctor`
5.  **Run it in development**: `npm run dev:app`
6.  **Build the desktop app**: `npm run build:app`

## ✨ Features

- **Internal Generator**: Automatically generate `.http` files from OpenAPI specs. No external scripts required.
- **Postman Import**: Convert simple Postman Collection JSON files into runnable `.http` files in your selected folder. Postman scripts are preserved as comments for safe manual conversion.
- **Remembered Preferences**: Theme, language, selected environment, token visibility, and HTTP Trace height persist across restarts.
- **Visual Feedback**: Real-time response visualization.
- **Runes Powered**: Built with Svelte 5 for lightning-fast reactivity.
- **Configurable UI**: Style variables based on OKLCH for consistent, beautiful themes.
- **i18n Support**: Available in English and Spanish.
- **Baby-Easy**: No steep learning curves. Just open your `.http` files and start requesting.

## ⚙️ Internal Generator

To enable the internal generator, provide a `yacito.config.json` in your `.http` folder or its parent:

```json
{
  "services": [
    {
      "name": "example-service",
      "localPort": 8080,
      "dockerPort": 5000,
      "hostVar": "exampleService",
      "openapiPath": "/v3/api-docs"
    }
  ]
}
```

Yacito will fetch the OpenAPI specs and generate the corresponding `.http` files and `http-client.env.json` when you click **Sync**.

## 📮 Postman Collection Import

If a service does not expose Swagger/OpenAPI yet, you can import an exported Postman Collection instead. Select your `.http` folder in Yacito, click **Import Postman**, and choose a `*.postman_collection.json` file.

Yacito writes the generated `.http` file into the currently selected folder, then refreshes the sidebar. Basic Auth, headers, raw bodies, folders, and `{{variables}}` are converted. Postman `pre-request`/`test` scripts use Postman's `pm.*` API, so they are preserved as comments with a TODO instead of being executed automatically. Convert those scripts manually to httpYac scripting before enabling them.

## 📦 Builds and Artifacts

For stable builds, check the [GitHub Releases page](https://github.com/guidonaselli/yacito/releases). Release tags like `v0.1.0` create a draft release with Linux, Windows, and macOS bundles.

Every push and pull request runs the full CI pipeline: Svelte type-checking, frontend build, Rust formatting, Clippy, Rust tests, and real Tauri desktop builds for Linux, Windows, and macOS.

You can download temporary CI build artifacts from the [GitHub Actions CI workflow](https://github.com/guidonaselli/yacito/actions/workflows/ci.yml):

- `yacito-windows`
- `yacito-linux`
- `yacito-macos`

For a local production build:

```bash
npm run build:app
```

The local artifacts will be generated in `src-tauri/target/release/bundle/`.

## ✅ Release Smoke Test

Before publishing a release, run through [`docs/SMOKE_TEST.md`](docs/SMOKE_TEST.md).

## 👶 Why Yacito?

Apart from being the "baby" version of httpYac, the word **Yacito** (*yācito*) literally means "requested" or "asked" in ancient Sanskrit and Pali. It’s the perfect name for a tool dedicated to making HTTP requests!

---
*Yacito is not an official httpyac project. It is a companion GUI that uses the httpyac CLI as its request engine.*

Made with 🍼 and ☕ for the developer community.
