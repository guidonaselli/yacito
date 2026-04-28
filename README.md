# Yacito

<p align="center">
  <img src="static/yacito-logo.png" alt="Yacito logo" width="180" />
</p>


Yacito is a tiny desktop workbench for [`httpyac`](https://httpyac.github.io/): pick a folder of `.http` files, browse requests, edit a request temporarily, and send it without leaving the app.

Yacito is not an official httpyac project. It is a companion GUI that uses the httpyac CLI as its request engine when available.

## Goals

- Keep `.http` files and `httpyac` as the source of truth.
- Work on Linux, Windows, and macOS through Tauri.
- Provide a friendly GUI for request discovery, temporary edits, environment selection, tokens, and response output.
- Stay lightweight: no proprietary collection format.
- Support i18n from the start.

## Current features

- Select and persist a folder containing httpyac `.http` files.
- Load environments from `http-client.env.json`.
- Browse endpoints grouped by `.http` file.
- Edit the selected `.http` block temporarily before sending.
- Send edited or original requests through `httpyac`.
- Optional Resync integration when the selected folder belongs to a workspace with `scripts/generate-http-files.py`.
- English/Spanish UI dictionary.

## Install from source

```bash
git clone https://github.com/<your-org>/yacito.git
cd yacito
npm install
npm run doctor
```

If `doctor` reports that `httpyac` is missing, install it explicitly:

```bash
npm run setup:httpyac
npm run doctor
```

Yacito does **not** install `httpyac` automatically during `npm install`, because global installs should be an explicit user choice.

## Run from source

```bash
npm run dev:app
```

Useful checks:

```bash
npm run check
cd src-tauri && cargo fmt && cargo test
```

## Build from source

```bash
npm run build:app
```

The generated installer/bundle is written by Tauri under `src-tauri/target/release/bundle/`.

## Windows notes

Recommended baseline:

1. Install Node.js LTS.
2. Install Rust from <https://rustup.rs/>.
3. Install the Microsoft C++ Build Tools if Tauri/Rust requests them.
4. Install dependencies and run:

```powershell
npm install
npm run setup:httpyac
npm run doctor
npm run dev:app
```

Yacito searches for `httpyac` in `PATH`, including Windows `.cmd` shims such as `httpyac.cmd` from global npm installs.

## Linux notes

Tauri requires WebKit/GTK system packages. The exact package names depend on the distro. After the system packages are installed:

```bash
npm install
npm run setup:httpyac
npm run doctor
npm run dev:app
```

## Project structure

```text
src/                 Svelte UI
src/lib/i18n.ts      Minimal i18n dictionary
src-tauri/           Tauri/Rust backend
scripts/doctor.mjs   Source-install diagnostics
openspec/            SDD specs and active change docs
```

## Attribution

Yacito uses the `httpyac` CLI as its request engine when installed. httpyac is an MIT-licensed project by Andreas Weber. See <https://httpyac.github.io/> and <https://github.com/AnWeber/httpyac>.

## License

MIT
