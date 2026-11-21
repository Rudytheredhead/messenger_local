# Messenger Local

Prywatny, lekki wrapper **Messengera** dla Windows 11 — zbudowany na **Tauri v2**, bez Electrona, bez nieoficjalnych API, bez telemetrii.

## ✨ Cechy

- **Dynamic Island na Windows** — rozwijane okno w stylu Dynamic Island (skrót `Ctrl+Alt+I`), ikona w zasobniku systemowym.
- **Zero telemetrii** — żadne dane nie opuszczają Twojej maszyny.
- **Izolowany profil** — dedykowany WebView2 data folder, ciasteczka i sesje oddzielone od przeglądarki.
- **Strict CSP** — polityka bezpieczeństwa ograniczająca połączenia wyłącznie do domen Facebook/Messenger.
- **System WebView2** — korzysta z WebView2 zainstalowanego w systemie (Tauri v2), bez dołączania Chromium.
- **Globalny skrót** — `Ctrl+Alt+I` otwiera/zwija wyspę z dowolnego miejsca.
- **Zasobnik systemowy** — aplikacja działa w tle, zamknięcie okna nie kończy procesu.

## 🏗 Architektura

```
messenger_local/
├── src-tauri/          # Aplikacja Rust (Tauri v2)
│   ├── src/
│   │   ├── main.rs     # Entry point
│   │   ├── lib.rs      # Setup Tauri, moduły, skróty, tray
│   │   ├── commands.rs # Komendy invoke z frontendu
│   │   ├── core/       # Logika okna, tray, stan, fullscreen guard
│   │   └── modules/    # Moduły wyspy (Messenger)
│   ├── Cargo.toml
│   └── tauri.conf.json # Konfiguracja Tauri, CSP, bundling
├── crates/
│   └── island-core/    # Czysta logika Dynamic Island (bez Tauri), testowalna osobno
├── src/                # Frontend TypeScript (Vanilla, esbuild)
│   └── main.ts         # Interfejs wyspy, komunikacja z backendem
├── dist/               # Zbuildowany frontend (commitowany)
├── Cargo.toml          # Workspace Cargo
└── package.json        # Skrypty frontendu
```

## 🚀 Wymagania

- **Windows 10/11** z **WebView2 Runtime** (Evergreen lub Fixed Version)
- **Rust** (toolchain `stable-x86_64-pc-windows-gnu` zalecany — MinGW UCRT)
- **Node.js** 18+ (do buildu frontendu)

### Toolchain Rust (Windows)

```bash
# Zainstaluj MinGW UCRT (WinLibs)
winget install BrechtSanders.WinLibs.POSIX.UCRT

# Przełącz toolchain na GNU (działa out-of-the-box)
rustup default stable-x86_64-pc-windows-gnu
```

> Toolchain MSVC też działa, ale wymaga `link.exe` z Visual Studio BuildTools w PATH.

## 🛠 Budowanie

```bash
cd messenger_local

# Zbuduj frontend (TypeScript → dist/)
npm run build

# Zbuduj aplikację Rust (release)
cargo build --release

# Binarka: target/release/messenger_local.exe
```

## 🏃 Uruchamianie

```bash
./target/release/messenger_local.exe
```

Aplikacja startuje w zasobniku systemowym. Użyj **`Ctrl+Alt+I`** by rozwinąć wyspę.

## 🔒 Bezpieczeństwo i prywatność

| Mechanizm | Szczegóły |
|-----------|-----------|
| **CSP** | `default-src 'self' https://www.facebook.com https://*.facebook.com ...` — brak inline skryptów, brak zewnętrznych CDN. |
| **Profil** | `webview2-com` + dedykowany `userDataFolder` — ciasteczka, localStorage, sesje w izolacji. |
| **Sieć** | `connect-src` ograniczony do `facebook.com`, `messenger.com`, `fbcdn.net`, `fbsbx.com` + WSS. |
| **Ramki** | `frame-ancestors 'none'` — brak embeddingowania, brak clickjackingu. |
| **Telemetria** | Żadna. Żadne `tauri-plugin-telemetry`, żadne analytics. |

## ⌨ Skróty klawiszowe

| Skrót | Akcja |
|-------|-------|
| `Ctrl+Alt+I` | Przełącz rozwijanie/zwijanie wyspy |
| Klik poza wyspą | Automatyczne zwinięcie (utracona fokus) |
| PPM na ikonie w tray | Menu: Otwórz, Zakończ |

## 📦 Rozpakowywanie / Dystrybucja

```bash
cargo tauri build
```

Tworzy instalator (MSI/EXE) w `src-tauri/target/release/bundle/`. WebView2 bootstrapper jest dołączany automatycznie (`downloadBootstrapper`).

## 🧪 Testowanie logiki wyspy (izolowane)

```bash
cd crates/island-core
cargo test
```

`island-core` nie zależy od Tauri — czysta logika stanowa, łatwa do testowania jednostkowego.

## 📄 Licencja

MIT — patrz [LICENSE](LICENSE) (jeśli plik istnieje) lub nagłówki plików źródłowych.

## 🙋 Autor

Projekt prywatny, open-source. PR-y i issue mile widziane.