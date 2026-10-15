Tech stack: Yacito standalone Tauri v2 + Svelte 5 + Rust backend.
Architecture: Yacito is a local desktop utility over user-selected folders containing httpyac .http files.
Testing: Svelte type checks via npm run check; Rust compile/test via cargo; no app-specific unit/e2e suite currently detected for Yacito.
Style: Minimal scoped changes, preserve .http/httpyac as source-of-truth, follow open-source, cross-platform, minimal-change rules.
