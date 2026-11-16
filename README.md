# Marginalia — Your daily reading desk

A calm, personal RSS reader shipped as a **single portable binary**.

- **Windows**: one `Marginalia.exe` (~11 MB). Copy it anywhere and run it — no installer, no admin, no data folder next to it.
- **Linux**: one `Marginalia-<version>-linux-x64.AppImage`. `chmod +x` and run.
- The web app (SSR + static hosting) still builds from the same source.

## Download

Grab the current release from [GitHub Releases](https://github.com/kaidohex/marginalia/releases/latest).
Every release ships `SHA256SUMS.txt` so you can check the download:

```powershell
Get-FileHash Marginalia-<version>-windows-x64.exe -Algorithm SHA256
```

## How it works

```
┌─────────────────────────────────────────────┐
│ Marginalia.exe                              │
│  ├─ embedded SPA (the whole frontend)       │
│  ├─ loopback server on 127.0.0.1:43117      │
│  │    ├─ serves the app + /feeds.json       │
│  │    └─ /proxy?url=…  fetches RSS          │
│  ├─ WebView2 window (OS-provided)           │
│  └─ single-instance lock                    │
└─────────────────────────────────────────────┘
```

- **Everything is embedded.** `rust-embed` compiles the built SPA into the
  executable, so the binary is self-contained.
- **A loopback HTTP origin.** localStorage is keyed by origin, and the port is
  fixed on purpose — a random port would give the app a fresh empty profile
  every launch. Starting a second copy focuses the running window instead of
  fighting over the port.
- **Feeds are fetched by the Rust side** through `/proxy?url=...`, so no feed
  server needs to send CORS headers.
- **External links open in your browser.** Article links never load inside the app.
- **Your data lives in the webview profile** (`%LOCALAPPDATA%\dev.kaidohex.marginalia`
  on Windows). Use **Manage feeds → Export/Import** to move feeds between
  machines or between the web and desktop builds.
- Windows needs the **WebView2 runtime** (preinstalled on Windows 11 and most
  Windows 10 installs; Windows updates it automatically).

## Development

```powershell
npm install
npm run dev            # web app with SSR + hot reload
npm run desktop        # native window against the dev server (tauri dev)
npm run desktop:build  # release binary in src-tauri/target/release/
```

The first `desktop:build` compiles the Rust shell (several minutes); later
builds reuse the cache. Windows builds need the MSVC toolchain (Visual Studio
Build Tools with the C++ workload); Linux builds need the usual webkit
dependencies listed by the [Tauri docs](https://tauri.app/start/prerequisites/).

## Checks

```powershell
npm run typecheck   # tsc --noEmit
npm run lint        # eslint + prettier
npm test            # vitest
cargo fmt / cargo clippy in src-tauri
```

CI runs the typecheck, lint, tests, and both platform builds on every push and
pull request; tags (`v*`) publish a release after the checks pass.

## History

Marginalia started life as [rss-feeder](https://github.com/kaidohex/rss-feeder),
a Lovable-generated web app that was wrapped in Electron to go desktop. This
repo is the same reading experience rebuilt on Tauri so the whole app fits in
one portable binary.
