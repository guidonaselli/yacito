# Yacito Smoke Test Checklist

Use this checklist before cutting a release or after changing request execution/import behavior.

## App startup

- [ ] App starts without console errors.
- [ ] Last selected `.http` folder is restored, or empty state is shown clearly.
- [ ] Language selector persists across restart.
- [ ] Last selected environment persists when it still exists.

## Folder and endpoint browsing

- [ ] Choose a folder containing `.http` files.
- [ ] Services/endpoints load in the sidebar.
- [ ] Search filters by service, method, endpoint name, and path.
- [ ] Selecting an endpoint loads its `.http` block in the editor.

## Request execution

- [ ] Run original request.
- [ ] Edit request in the editor and run edited request.
- [ ] Reset restores original request content.
- [ ] Response body is shown separately from HTTP trace.
- [ ] HTTP trace panel can be resized and keeps its height after restart.

## Sync and config

- [ ] If no generator is detected, Create Config writes a generic `yacito.config.json`.
- [ ] If generator is detected, Resync runs and writes generated `.http` files.
- [ ] Sync log appears in the empty/dashboard panel.

## Postman import

- [ ] Click Import Postman with a folder selected.
- [ ] Select a `*.postman_collection.json` file.
- [ ] A `.http` file is created in the selected folder.
- [ ] Sidebar refreshes and opens the imported endpoint when possible.
- [ ] Variables are declared as `@variable = TODO`.
- [ ] Basic Auth is represented via `@postmanBasicUsername`, `@postmanBasicPassword`, and `Authorization: Basic ...`.
- [ ] Postman scripts are preserved as `# postman-script:` comments and are not executed automatically.

## Platform checks

- [ ] Windows: run from a WSL/UNC path without `cmd.exe` UNC errors.
- [ ] Windows: build/install script closes old Yacito processes before rebuilding.
- [ ] Linux/WSL: installer completes and launches the built binary.
- [ ] CI: quality gates and Linux/Windows/macOS Tauri builds are green.
