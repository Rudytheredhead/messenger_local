# Progress: Messenger Local (Tauri v2)

## ✅ Completed

### 1. Rust Toolchain
- Installed Rust via rustup (MSVC toolchain + **GNU/MinGW toolchain installing...**)
- `cargo` and `rustc` available at `~/.cargo/bin/`

### 2. Project Structure Created
```
messenger_local/
├── Cargo.toml (root workspace)
├── src/
│   └── lib.rs
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/
    │   └── default.json
    ├── src/
    │   ├── main.rs
    │   └── lib.rs
    └── icons/ ✅ **Created** (`icon.ico`, `icon.png`, `icon.icns`, `tray-icon.png`)
```

### 3. Configuration Files

**Root `Cargo.toml`** - Workspace with release profile optimized for size/RAM:
- `lto = true`, `strip = true`, `opt-level = "z"`, `codegen-units = 1`, `panic = "abort"`

**`src-tauri/Cargo.toml`** - Dependencies:
- `tauri` v2.0 with features: `tray-icon`
- `tauri-plugin-opener` v2.0
- `tauri-plugin-notification` v2.0
- `url` v2.5 (for URL parsing in navigation filter)

**`tauri.conf.json`** - Fully configured:
- Window URL: `https://www.facebook.com/messages`
- `withGlobalTauri: false` (complete isolation from Rust backend)
- Strict CSP allowing only Meta domains (`facebook.com`, `messenger.com`, `fbcdn.net`, `fbsbx.com`)
- `contentProtection: true` (prevents screenshots/recording)
- Tray icon menu: "Otwórz", "Wyloguj / Wyczyść sesję", "Zakończ"
- Opener plugin configured for external links

**`capabilities/default.json`** - Least privilege:
- Only: `core:default`, window controls, `webview:allow-on-navigation`, `opener:default`, `notification:default`, `tray:default`
- **NO** `fs`, **NO** `shell`, **NO** filesystem access

### 4. Rust Logic (`src-tauri/src/lib.rs`)

- **Navigation filter** (`on_navigation`): Allows only `https://` + subdomains of allowed Meta domains; everything else opens in system browser via `tauri-plugin-opener`
- **Tray menu** with three actions:
  - "Otwórz" → shows/focuses main window
  - "Wyloguj / Wyczyść sesję" → navigates to Facebook logout + deletes `app_data_dir/webview` (clears cookies/cache)
  - "Zakończ" → exits app
- **Window close behavior**: Hide to tray instead of closing (prevents accidental logout)

### 5. Entry Point (`src-tauri/src/main.rs`)
```rust
use messenger_local::run;
fn main() { run(); }
```

---

## ⏳ In Progress

| Task | Status |
|------|--------|
| Build & test compile (`cargo build --release`) | 🔄 **Waiting for MinGW install → building with GNU toolchain** |
| Verify RAM usage < 150 MB after login + minimize to tray | ⏳ Pending |
| Network verification (Wireshark/Resource Monitor → only Meta IPs) | ⏳ Pending |
| Security Report (dependencies justification, isolation proof) | ⏳ Pending |

---

## Next Steps

1. **Finish build** - MinGW installing, then `cargo build --release` with GNU toolchain
2. **Test run** - Verify navigation filter, tray, logout/clear-session works
3. **Security audit** - Confirm zero external connections, RAM target met