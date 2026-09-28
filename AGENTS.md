# Warface Log Cleaner

Windows-only Tauri 2 / Rust 1.96.0 / React 19 / Node 24.15.0 / pnpm 10.33.1.

Scope: Warface logs, known profile cache folders, game-root crash dumps, explicitly listed GameCenter logs/caches/manifests, and packages/warface. Shared launcher targets are opt-in and identified as shared. Never add global Temp, account/browser sign-in data, launcher binaries or full-profile reset targets. Keep preview read-only and require explicit confirmation with a backend-held plan. Do not test cleanup on real user files.

Verification: `pnpm build`; `pnpm exec playwright install chromium`; `pnpm test`; `cargo fmt --manifest-path src-tauri/Cargo.toml --check`; `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`; `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings`; `pnpm tauri build --ci`.

Playwright config: `.devin/playwright.config.ts`. Browser tests mock IPC. Rust tests only delete their own unique fixtures. Release artifacts live under `src-tauri/target/release/`; do not commit build outputs. User-supplied fonts are third-party assets; do not claim they are MIT licensed.
