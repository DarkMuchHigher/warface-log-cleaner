# Warface Log Cleaner

Windows utility for reviewing and removing Warface logs, caches, crash dumps and update packages,
plus explicitly listed VK Play launcher logs and caches.
Built with Tauri 2, Rust, React 19 and TypeScript. Russian and English interfaces.
Unofficial project; not affiliated with Warface or its publisher.

## Download

Open this repository's **Releases** and choose:

- **Warface-Log-Cleaner-<version>-windows-x64-setup.exe** — recommended per-user installer.
- **Warface-Log-Cleaner-<version>-windows-x64-portable.exe** — run without installing the app.
- **SHA256SUMS.txt** — SHA-256 checksums for both files.

Windows 10/11 x64 and Microsoft Edge WebView2 Runtime are required. The installer can download
WebView2 when missing; the portable executable requires an already installed runtime. Portable
refers to the application executable, not a bundled browser or a zero-write runtime.
Builds are not Authenticode-signed; Windows may show an unknown-publisher warning.

## Cleanup scope

| Category | Files |
|---|---|
| Game logs | `Game.log`, `action_history.log`, `server_profile.txt`, `process.txt`, `-gup-/install.log`, `LogBackups/*.log` |
| Optional caches | Files inside `modelscache`, `BannersCache`, `headscache`, `QueryCache`, `Shaders` in the Warface profile |
| Optional crash dumps | `*.dmp` directly inside the validated Warface installation |
| Optional VK Play launcher | `main.log`, `Chrome.log`, `sdump*.dmp`, named catalog manifests; image/common/notification/description/update-list caches; browser `Cache`, `Code Cache`, `GPUCache` and `DawnCache` subfolders |
| Optional Warface updates | `DownloadPath/packages/warface` |

Launcher data is shared across games. The launcher category is explicitly opt-in. Browser sign-in
storage (`Network`, cookies, local/session storage), `GameCenter.ini`, launcher binaries and the
entire `Chrome` runtime folder are not cleanup targets.

**Never selected:** global Temp folders, unrelated crash reports, other games' update packages,
game settings or key bindings. Cache, dump, launcher and update categories are off by default. Empty directories are kept. Clearing caches may slow the next game launch; crash
dumps may be useful to support.

## Usage and safeguards

1. Scan files. If auto-detection fails, enter the installation root containing `Bin64Release`,
   `Game` and `Engine` by expanding **Game folder**.
2. Select categories or individual targets.
3. Press **Clean** to open the file list. This is a read-only preview with paths and sizes; it never terminates
   processes or removes files.
4. Close the game and launcher yourself, then explicitly confirm deletion.

Deletion is permanent, without the Recycle Bin. Review the list carefully. The backend accepts
only a recent, server-held preview ID, never arbitrary file paths. Empty/unknown selections fail
closed. New files are not added to a confirmed plan; changed or replaced files are skipped.
Junctions, symbolic links and hard-linked files are excluded. Parent-directory handles are held
while deleting by file handle, preventing ancestor renames during deletion. Locked/inaccessible
files are reported and skipped. The app does not disable protection, force-close processes or
modify game binaries. It cannot guarantee how the game will behave after any cache removal.

## Development

Windows with MSVC Build Tools, Rust **1.96.0**, Node **24.15.0**, pnpm **10.33.1**, and WebView2:

```sh
pnpm install --frozen-lockfile
pnpm tauri dev
```

Verification:

```sh
pnpm build
pnpm exec playwright install chromium
pnpm test
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
pnpm tauri build --ci
```

Rust deletion tests create their own isolated fixtures. Browser tests mock Tauri IPC and never
access game files. Plain browser preview disables filesystem actions.

## CI and releases

The Windows workflow checks formatting, Rust tests, Clippy, frontend build and browser tests,
then builds an NSIS installer and uploads both binaries as a workflow artifact.

A `v<version>` tag triggers a release after all checks pass. The tag must match `package.json`,
`src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`. A draft release receives both files and
checksums before publication. Branch pushes, pull requests and manual workflow runs produce
artifacts only, not public releases. Releases are immutable by convention: publish a new version
rather than reusing an existing tag. The release job alone receives `contents: write`.

## Fonts

The supplied `warface-ru.ttf` and `warface-en.ttf` are used locally, without external font requests.
They are third-party assets and are not covered by a project source-code license. Verify the font
owners' redistribution terms before publishing builds that include them.
