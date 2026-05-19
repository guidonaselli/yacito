# 👶 Yacito

[![CI](https://github.com/guidonaselli/yacito/actions/workflows/ci.yml/badge.svg)](https://github.com/guidonaselli/yacito/actions/workflows/ci.yml)

<p align="center">
  <img src="static/yacito-logo.png" alt="Yacito logo" width="180" />
</p>

The adorable, interactive, and **baby-easy** GUI for [httpYac](https://httpyac.github.io/).

Yacito (from the Sanskrit *yācito*, meaning "requested") is a user-friendly desktop interface built on top of the powerful httpYac engine. It’s designed for developers who want the full power of text-based HTTP files with the comfort of a visual, interactive workspace. Making complex API requests is now child's play.

## 🍼 Quick Install

Get up and running from source in seconds. These scripts install Rust, system dependencies, project requirements, build the production desktop app, and create a convenient launcher/shortcut when supported.

### Windows (PowerShell)
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/guidonaselli/yacito/main/scripts/install-windows.ps1?v=' + [guid]::NewGuid()))
```

### Linux
```bash
curl -sSL https://raw.githubusercontent.com/guidonaselli/yacito/main/scripts/install-linux.sh | bash
```

The Linux installer currently supports `apt`, `pacman`, `dnf`, and `zypper`. By default it clones/updates Yacito in `~/work/yacito`, builds Linux bundles locally, installs a launcher in `~/.local/bin/yacito`, registers a `.desktop` app entry, and installs icons into the local hicolor theme so app launchers can discover it cleanly.

To preconfigure a workspace such as `plataformamonitoreo/api-http` from the installer:

```bash
curl -sSL https://raw.githubusercontent.com/guidonaselli/yacito/main/scripts/install-linux.sh | bash -s -- \
  --api-http-dir /absolute/path/to/api-http
```

Useful options:

```bash
curl -sSL https://raw.githubusercontent.com/guidonaselli/yacito/main/scripts/install-linux.sh | bash -s -- --help
```

- `--dir /custom/path` → install somewhere else
- `--api-http-dir /path/to/api-http` → leave the launcher preconfigured for your workspace
- `--skip-build` → install/update dependencies now and build later

> If `~/.local/bin` is not in your `PATH`, the installer adds it to `~/.bashrc`.

## 🛠️ Manual Setup

If you prefer to do it yourself:

1.  **Clone the repo**: `git clone https://github.com/guidonaselli/yacito`
2.  **Install dependencies**: `npm install`
3.  **Validate your environment**: `npm run doctor`
4.  **Optional: install httpYac globally**: `npm run setup:httpyac`
5.  **Run it in development**: `npm run dev:app`
6.  **Build the desktop app**:
    - Cross-platform/default: `npm run build:app`
    - Linux local bundles (deb/rpm): `npm run build:app:linux`

## ✨ Features

- **Workspace Generators**: Sync `.http` files either from built-in OpenAPI config or from your own generator command/script.
- **Recursive Workspace Loading**: Open a top-level `httpyac` folder and Yacito discovers `.http` files in nested directories too.
- **Postman Import**: Convert simple Postman Collection JSON files into runnable `.http` files in your selected folder. Postman scripts are preserved as comments for safe manual conversion.
- **Remembered Preferences**: Theme, language, selected environment, token visibility, and HTTP Trace height persist across restarts.
- **Login Quality of Life**: Successful `/login` responses automatically populate the bearer token field when the API returns a JWT.
- **Visual Feedback**: Real-time response visualization.
- **Runes Powered**: Built with Svelte 5 for lightning-fast reactivity.
- **Configurable UI**: Style variables based on OKLCH for consistent, beautiful themes.
- **i18n Support**: Available in English and Spanish.
- **Baby-Easy**: No steep learning curves. Just open your `.http` files and start requesting.

## ⚙️ Workspace Generators

Yacito is meant to be plug-and-play with any `httpyac` workspace. If your folder already contains `.http` files and `http-client.env.json`, you can just open it and work.

For a more complete contract/reference for reusable workspaces, see [`docs/WORKSPACE_GENERATORS.md`](docs/WORKSPACE_GENERATORS.md).

If you also want Yacito to regenerate those files, place a `yacito.config.json` in the `api-http` folder or its parent. There are two supported models:

### 1) Built-in OpenAPI sync

Use this when each service exposes an OpenAPI endpoint and you want Yacito to generate `.http` files itself:

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

### 2) Custom workspace generator

Use this when your project already has its own script, task runner, or scaffolding process:

```json
{
  "generator": {
    "command": "python3",
    "args": ["scripts/generate-http-files.py", "--env", "{{env}}", "{{service}}"],
    "cwd": ".."
  }
}
```

Supported placeholders in `command`, `args`, and `cwd`:

- `{{env}}` → selected environment in the UI
- `{{service}}` → selected service name, if any
- `{{apiHttpDir}}` → absolute path to the current `.http` workspace
- `{{repoRoot}}` → parent directory of the `api-http` folder

Yacito also exports these environment variables when running a configured generator:

- `YACITO_ENV`
- `YACITO_SERVICE`
- `YACITO_API_HTTP_DIR`
- `YACITO_REPO_ROOT`

That makes it easy to reuse existing scripts without coupling the app to one repo layout.

### 3) Script auto-discovery (legacy-friendly)

If you do not want a configured command, Yacito can still auto-discover these scripts with compatibility across both styles:

- directly inside the selected `api-http` folder
- inside `api-http/scripts/`
- inside `api-http/.yacito/`
- inside repo-level `scripts/`
- inside repo-level `.yacito/`

- `generate-http-files.py`
- `generate-http-files.sh`
- `generate-http-files.js`
- `generate-http-files.mjs`
- `scripts/generate-http-files.py`
- `scripts/generate-http-files.sh`
- `scripts/generate-http-files.js`
- `scripts/generate-http-files.mjs`
- `.yacito/generate-http-files.py`
- `.yacito/generate-http-files.sh`
- `.yacito/generate-http-files.js`
- `.yacito/generate-http-files.mjs`

If `httpyac` is not installed globally but `npx` is available, Yacito automatically falls back to `npx httpyac` when you click **Send**.

## 🐧 Linux Notes

- The recommended local Linux build is `npm run build:app:linux`, which generates `deb` and `rpm` bundles.
- `npm run build:app` still exists for the full default Tauri bundle flow, but some local environments may fail specifically on AppImage tooling (`linuxdeploy`) even if the native binary, `.deb`, and `.rpm` build correctly.
- The launcher created by `install-linux.sh` can predefine `HTTPYAC_API_HTTP_DIR`, which is useful for workspaces like `plataformamonitoreo/api-http`.

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

For Linux-only local bundles without AppImage:

```bash
npm run build:app:linux
```

The local artifacts will be generated in `src-tauri/target/release/bundle/`.

## ✅ Release Smoke Test

Before publishing a release, run through [`docs/SMOKE_TEST.md`](docs/SMOKE_TEST.md).

## 👶 Why Yacito?

Apart from being the "baby" version of httpYac, the word **Yacito** (*yācito*) literally means "requested" or "asked" in ancient Sanskrit and Pali. It’s the perfect name for a tool dedicated to making HTTP requests!

---
*Yacito is not an official httpyac project. It is a companion GUI that uses the httpyac CLI as its request engine.*

Made with 🍼 and ☕ for the developer community.
