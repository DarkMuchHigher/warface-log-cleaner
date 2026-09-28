# Warface Log Cleaner

Windows-only Tauri 2 / Rust 1.96.0 / React 19 / Node 24.15.0 / pnpm 10.33.1.

Scope: Warface logs, known profile cache folders, game-root crash dumps, explicitly listed GameCenter logs/caches/manifests, launcher packages and opt-in account data (game `user_*.cfg`/`Profiles`, launcher `GameCenter.ini`/`configPlays*.dat`). Everything outside `logs` is opt-in. Never add global Temp, browser sign-in storage (cookies/Network/Local Storage/IndexedDB/Session Storage), the `AV` FFmpeg component, launcher binaries or other games' data. Extension overrides live in `catalog::extension_override` and must stay per-target. Keep preview read-only and require explicit confirmation with a backend-held plan. Do not test cleanup on real user files.

Verification: `pnpm build`; `pnpm exec playwright install chromium`; `pnpm test`; `cargo fmt --manifest-path src-tauri/Cargo.toml --check`; `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`; `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`; `pnpm tauri build --ci`.

Playwright config: `.devin/playwright.config.ts`. Browser tests mock IPC. Rust tests only delete their own unique fixtures. Release artifacts live under `src-tauri/target/release/`; do not commit build outputs. User-supplied fonts are third-party assets; do not claim they are MIT licensed.
