# Contributing to Yacito

Thanks for helping make Yacito better.

## Development checks

Before opening a PR, run:

```bash
npm run check
cd src-tauri && cargo fmt && cargo test
```

## Principles

- Preserve `.http` and `httpyac` compatibility.
- Avoid introducing a proprietary collection format.
- Keep cross-platform behavior in mind, especially Windows path and command resolution.
- Put user-visible strings in `src/lib/i18n.ts`.
