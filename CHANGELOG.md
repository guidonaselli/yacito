# Changelog

## v0.1.0 - Initial public preview

Yacito is now ready as a baby-easy desktop GUI companion for httpYac.

### Highlights

- Standalone Tauri + Svelte desktop app with Yacito branding.
- Browse and run `.http` files with editable in-memory request overrides.
- Postman-like response view that separates HTTP trace from response body.
- Resizable HTTP trace panel with persisted UI preferences.
- Sidebar endpoint search.
- Internal OpenAPI generator for workspaces with `yacito.config.json`.
- Simple Postman Collection import into `.http` files, preserving Postman scripts as TODO comments.
- Windows/WSL compatibility fixes for UNC paths and global npm CLI wrappers.
- Source installers for Windows and WSL/Linux.
- CI quality gates and desktop builds for Linux, Windows, and macOS.

### Notes

- Postman scripts use the `pm.*` API and are not executed automatically after import. Convert preserved `# postman-script:` comments to httpYac scripting before enabling them.
- Windows/macOS artifacts are currently unsigned.
