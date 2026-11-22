# Progress: Messenger Local (Tauri v2)

Status na 2026-11-21: aplikacja kompletna pod względem kodu zgodnie z `To-Do.md` (Kroki 1–4).
Pozostają weryfikacje wymagające Twojego konta Messengera i monitora sieci (sekcja "Do zrobienia ręcznie").

## ✅ Zrobione

### 1. Toolchain
- Rust (rustup): toolchain GNU (MinGW/WinLibs UCRT) działa; MSVC wymaga VS BuildTools w PATH.
- Build: `cargo build --release` w `messenger_local/` → `target/release/messenger_local.exe` (≈3,1 MB).

### 2. Struktura projektu
```
messenger_local/              (repo git)
├── Cargo.toml                workspace; [profile.release] tylko tutaj
├── Cargo.lock                śledzony (aplikacja)
├── dist/index.html           placeholder wymagany przez frontendDist (okno ładuje zdalny URL)
└── src-tauri/
    ├── Cargo.toml
    ├── build.rs
    ├── tauri.conf.json
    ├── capabilities/default.json
    ├── icons/                icon.ico, icon.png, tray-icon.png
    └── src/{main.rs, lib.rs}
```
Usunięto: martwy `src/lib.rs` (szablon `add()`), duplikat `[profile.release]` z `src-tauri/Cargo.toml`.

### 3. Konfiguracja (`tauri.conf.json`)
- `app.windows: []` — okno tworzone w kodzie (potrzebne do `data_directory`, `on_navigation`, `on_new_window`).
- Usunięto `app.trayIcon` (powodowało drugą ikonę w zasobniku bez menu) i `devUrl`.
- Usunięto zduplikowany klucz `titleBarStyle`.
- `bundle.icon`: `icons/icon.ico` + `icons/icon.png` (bundler Windows wymaga `.ico`).
- `withGlobalTauri: false`, CSP bez zmian.

### 4. Logika (`src-tauri/src/lib.rs`)
- **Filtr nawigacji** (`on_navigation`): dozwolone tylko `https`, bez loginu/hasła w URL, bez niestandardowego portu, host = `facebook.com`/`messenger.com`/`fbcdn.net`/`fbsbx.com` lub ich subdomena. `about:blank` dozwolony.
- **Obce linki https** → blokowane w oknie i otwierane w przeglądarce systemowej (`tauri-plugin-opener`, `open_url`). Inne schematy (http, javascript, file, data) są po prostu blokowane.
- **Nowe okna** (`target=_blank`, `window.open`): domeny Mety nawigowane w głównym oknie, reszta do przeglądarki; zawsze `NewWindowResponse::Deny` (bez dodatkowych okien WebView).
- **Izolowany profil**: `data_directory = %LOCALAPPDATA%\com.messenger.local\webview-profile`.
- **Wylogowanie** (tray): `clear_all_browsing_data()` (ciasteczka, cache, local storage tego WebView), potem nawigacja na stronę Messengera (pokaże logowanie). Poprzednia wersja usuwała katalog `app_data_dir/webview`, którego WebView nie używał.
- **Zamknięcie okna** → ukrycie do zasobnika (bez `unwrap`, bo przy `panic = "abort"` każda panika zabija proces).
- **Zasobnik**: jedna ikona (`main-tray`), lewy klik pokazuje okno, prawy — menu (Otwórz / Wyloguj / Zakończ). Brak `unwrap` przy ikonie.
- `contentProtected(true)` ustawione w builderze okna.

### 5. Testy
- Testy jednostkowe filtra URL dodane w `lib.rs` (`mod tests`): dozwolone domeny Mety, blokada http/javascript/file/data, domeny podobne (`evilfacebook.com`, `facebook.com.evil.org`), blokada loginu w URL i portów.
- Wykonane: skopiowana logika filtra w osobnym crate → **4/4 testy OK**.
- Uwaga: `cargo test` w tym projekcie kończy się `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139) jeszcze przed uruchomieniem testów — problem środowiska/toolchainu GNU, nie logiki. Do zbadania (np. toolchain MSVC z VS BuildTools).

### 6. Weryfikacja builda
- `cargo build --release` — OK. Jedyne ostrzeżenie: linker GNU `.rsrc merge failure: multiple non-default manifests` (nieblokujące, wynika z manifestu Tauri + GNU).
- Uruchomienie `messenger_local.exe` — proces żyje po 20 s.

### 7. Repo
- `.gitignore`: `/target`, `/src-tauri/gen`, pliki IDE/OS/logi.
- Commity: `43e3ce6` (porządki i konfiguracja), `3921258` (nawigacja, okno, wylogowanie, zasobnik).

## 🛡️ Raport bezpieczeństwa (Krok 5.1 / 5.3)

**Zależności bezpośrednie** (`cargo tree --depth 1`):
| Crate | Wersja | Uzasadnienie |
|---|---|---|
| `tauri` | 2.12.7 | Okno, WebView2, zasobnik, IPC (tylko okno lokalne, brak uprawnień dla zdalnej strony). |
| `tauri-plugin-opener` | 2.7.3 | Otwieranie obcych linków w przeglądarce systemowej. |
| `tauri-plugin-notification` | 2.8.1 | Powiadomienia systemowe (wymaga dalszej decyzji — patrz niżej). |
| `url` | 2.5.8 | Parsowanie i walidacja URL w filtrze nawigacji. |
| `serde_json` | 1.0.154 | Wymagany przez Tauri. |
Łącznie ~559 crate'ów w grafie (głównie transitive z Tauri/wry/tao). Dokładna lista: `cargo tree -p messenger_local`.

**Izolacja danych:**
- Sesja, ciasteczka i cache: wyłącznie w `data_directory` tej aplikacji.
- Brak `fs`, `shell`, `http` w capabilities; brak dostępu strony do API Rusta (`withGlobalTauri: false`).
- Aplikacja nie czyta ani nie zapisuje haseł/tokenów/treści wiadomości — nie ma kodu, który by to robił.

**Ograniczenia, które trzeba znać:**
- **CSP** w `tauri.conf.json` dotyczy zasobów lokalnych aplikacji. Zdalna strona `facebook.com` nie jest nią objęta — jej ochrona to filtr nawigacji i brak IPC dla zdalnej treści.
- **Brak capability `remote`** — zdalna strona nie ma dostępu do `opener`/`notification` przez IPC. Powiadomienia z Messengera (Web Notifications API) zależą od WebView2; osobne `notification:default` nie zadziała z poziomu strony.
- **Instalator Windows** używa `downloadBootstrapper` — podczas instalacji pobiera WebView2 z serwerów Microsoft (nie Mety, nie działa w czasie pracy aplikacji). Alternatywa: `embedBootstrapper` (większy instalator, zero pobierania).
- **Pliki `icons/create_icons.ps1`/`create_ico.py`** — generatory, nie wchodzą do builda.

## ⏳ Do zrobienia ręcznie (wymaga Twojego konta / monitora)

1. **Logowanie i sesja** — zaloguj się, zamknij okno (ma się schować), przywróć z zasobnika; sprawdź, że lewy klik pokazuje okno, a menu działa.
2. **Wylogowanie** — po „Wyloguj / Wyczyść sesję” sprawdź, że trzeba zalogować się ponownie (ciasteczka usunięte). Uwaga: `clear_all_browsing_data` jest asynchroniczne; jeśli po wylogowaniu sesja nadal działa, dodamy opóźnienie/ponowne czyszczenie.
3. **Linki zewnętrzne** — kliknij link spoza Mety w czacie → ma się otworzyć w przeglądarce, a okno zostać na Messengerze.
4. **RAM** — po zalogowaniu i zminimalizowaniu do zasobnika (cel < 150 MB; Menedżer zadań → Szczegóły → `messenger_local.exe` i procesy `msedgewebview2.exe`; suma całego drzewa procesów).
5. **Ruch sieciowy** — Resource Monitor → Sieć: połączenia tylko do adresów IP Mety (np. `whois` na IP z listy połączeń). Wymaga działającego logowania.
6. **Instalator** — `cargo tauri build` (potrzebny `tauri-cli`): sprawdzić, czy powstaje bundle MSI/NSIS.

## ⚠️ Znane problemy / dług
- `cargo test` (binarka testowa) — `STATUS_ENTRYPOINT_NOT_FOUND` na toolchainie GNU; testy filtra przeszły w osobnym crate. Do sprawdzenia na MSVC.
- Brak testów E2E (WebView wymaga GUI i konta).
- Ikona tray i okna: tylko `icon.ico`/`icon.png`; brak `icon.icns` (macOS nie jest celem na razie).


---

## 🏝️ Dynamic Island (Messenger Island) — 2026-11-21

Plan i statusy: `To-Do.md`. Kod: `messenger_local/src-tauri/src/` (core, modules), `messenger_local/crates/island-core/`, interfejs: `messenger_local/src/` (TypeScript → `dist/`).

### Zrobione
- **island-core** (crate bez Tauri): filtr URL, licznik nieprzeczytanych z tytułu, wybór modułu do pigułki, detekcja pokrycia monitora, geometria. `cargo test -p island-core`: 8/8 OK.
- **Okno wyspy**: bez ramki, przezroczyste, bez cienia, zawsze na wierzchu, bez paska zadań, góra-środek; animacja pigułka 210×36 ⇄ 420×620.
- **Messenger jako moduł** (`modules/messenger.rs`): child webview pod paskiem zakładek, filtr nawigacji, obce linki do przeglądarki, nowe okna bez dodatkowych WebView, licznik z tytułu karty, wylogowanie przez `clear_all_browsing_data`.
- **Rejestr modułów** (`modules/mod.rs`): trait `IslandModule`, pigułka pokazuje moduł o najwyższym priorytecie z badge > 0.
- **Pełny ekran** (`core/fullscreen_guard.rs`): `SHQueryUserNotificationState` + aktywne okno pokrywające monitor; wyspa ukrywa się w grach i filmach. Potwierdzone przez użytkownika na żywo.
- **Zasobnik**: Pokaż wyspę · Nie przeszkadzaj (ukryj) · Wyloguj · Zakończ. Skrót globalny `Ctrl+Alt+I`.
- **Frontend**: Vanilla TypeScript + esbuild, HTML/CSS, bez frameworków.

### Poprawki po teście użytkownika
- **Okno PowerShell/konsoli przy uruchomieniu release**: brakowało `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` w `main.rs`. Dodano; nagłówek PE to teraz `Windows GUI` (sprawdzone `objdump`).
- **Biały kwadrat wokół pigułki**: domyślne białe tło WebView i cień okna. Ustawiono `background_color(0,0,0,0)` dla WebView wyspy oraz `shadow(false)` na oknie.

### Odchylenia od spec
- „Auto-hide przy Focused(false)” wdrożono jako **zwinięcie do pigułki**, nie ukrycie (ukrywanie po każdym kliknięciu obok uniemożliwiałoby użycie wyspy). Ukrycie działa przez „Nie przeszkadzaj” i w grach.
- Messenger ładuje się od razu (nie leniwie), żeby licznik nieprzeczytanych działał przy zwiniętej wyspie. Pozostałe moduły (przyszłe) mają nie tworzyć WebView.

### Weryfikacja
- `cargo check` / `cargo build --release`: OK (jedyne ostrzeżenie: linker GNU o wielu manifestach).
- `npm run build` (typecheck + bundle): OK.
- Uruchomienie release: proces żyje, okno „Messenger Island”, odpowiada; ~27 MB w procesie głównym.
- Użytkownik: pełny ekran działa, pigułka wyświetla się poprawnie (po poprawkach konsoli i tła — do potwierdzenia).

### Do zrobienia / znane
- Ręcznie: logowanie w Messengerze w child webview, animacja rozwijania, skrót, zasobnik, pomiar RAM z WebView2, ruch sieciowy.
- `cargo test` binarki Tauri: `STATUS_ENTRYPOINT_NOT_FOUND` na toolchainie GNU (znane).
- Instalator (`cargo tauri build`) jeszcze nie uruchamiany.


### RAM — redukcja (2026-11-21)
- Pomiar bazowy (zwinięta wyspa, Messenger załadowany): proces aplikacji ~25 MB; procesy WebView2 z profilu aplikacji ~737 MB (Working Set), 9 procesów.
- Próby flag Chromium przez `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` (`--renderer-process-limit=1`, `--disable-gpu`): brak wyraźnej poprawy w szumie pomiarowym — odrzucone.
- **Usypianie strony Messengera** (`core/suspend.rs`, WebView2 `TrySuspend`/`Resume`): po 5 min zwinięcia (domyślnie) strona jest uśpiona; po rozwinięciu wznawiana. Wynik po 100 s: Working Set **737 → 416 MB (−44%)**, Private 445 → 419 MB (−6%). Sama aplikacja nie ulega zmianie.
- Uśpienie jest **opcjonalne i domyślnie wyłączone** (przełącznik „Uśpij Messengera po 5 min” w zasobniku). Powód: uśpiona strona nie odbiera wiadomości, więc licznik nieprzeczytanych jest zamrożony. Ustawienia środowiskowe do strojenia: `MESSENGER_ISLAND_AUTO_SUSPEND=1`, `MESSENGER_ISLAND_SUSPEND_SECS=<n>`.
- Animacje CSS licznika działają tylko przy zwiniętej wyspie z nieprzeczytanymi (spec: usypianie animacji).
- Weryfikacja: `cargo build --release` OK; log WebView2 potwierdził `TrySuspend success=true` i `Resume` przy rozwinięciu (diagnostyka usunięta z kodu).
- Pomiary: zrzut z PowerShell (`Get-CimInstance` + `Get-Process`), jeden przebieg na konfigurację, zależne od stanu strony (zalogowany/niezalogowany) — traktować jako orientacyjne.


### Wygląd wyspy (2026-11-22)
- Zwinięta: cienki pasek przy górnej krawędzi (szewron + licznik), zamiast pigułki z ikoną i nazwą.
- Rozwinięta: rząd przycisków aplikacji (na razie Messenger), przycisk zwijania, pod spodem okno aplikacji.
- Weryfikacja: build release i testy `island-core` (8/8) OK. Zrzuty ekranu i `PrintWindow` nie odzwierciedlają treści WebView2 (widoczne tylko w rzeczywistym oglądzie), więc wygląd wymaga potwierdzenia na ekranie.
- Wypychanie na GitHub: zwykłe `git push` wiesza się na tej maszynie; działa z helperem `gh` (patrz pamięć agenta).

### Poprawki wyglądu — druga runda (2026-11-22)
- Przyczyna czarnego artefaktu i braku przycisków aplikacji: zwinięty pasek `.tab` (flex:1) nie był ukrywany w stanie rozwiniętym, więc pasek z przyciskami spadał na środek i był przykrywany przez WebView Messengera (y=56).
- Naprawa: `.expanded .tab {display:none}`, nieprzezroczyste tło wyspy, grubszy szewron SVG wycentrowany.
- Weryfikacja: zrzuty samego WebView wyspy przez CDP (`--remote-debugging-port`, tylko na czas testu). Geometria: zwinięty szewron 47–73 px w pasku 120 px; rozwinięty pasek y=0, przycisk aplikacji wycentrowany.
- Uwaga bezpieczeństwa: `--remote-debugging-port` daje lokalnym procesom dostęp do WebView (w tym sesji Messengera). Używać wyłącznie do testów, nigdy w buildzie użytkownika.
- Narzędzie diagnostyczne: skrypt CDP w `C:/Users/podwo/AppData/Local/hermes/cache/scratch/` (nie w repo).

### Wygląd wyspy (2026-11-22)
- Zwinięta: cienki pasek przy górnej krawędzi (szewron + licznik), zamiast pigułki z ikoną i nazwą.
- Rozwinięta: rząd przycisków aplikacji (na razie Messenger), przycisk zwijania, pod spodem okno aplikacji.
- Weryfikacja: build release i testy `island-core` (8/8) OK. Zrzuty ekranu i `PrintWindow` nie odzwierciedlają treści WebView2 (widoczne tylko w rzeczywistym oglądzie), więc wygląd wymaga potwierdzenia na ekranie.
- Wypychanie na GitHub: zwykłe `git push` wiesza się na tej maszynie; działa z helperem `gh` (patrz pamięć agenta).

### Poprawki wyglądu — druga runda (2026-11-22)
- Przyczyna czarnego artefaktu i braku przycisków aplikacji: zwinięty pasek `.tab` (flex:1) nie był ukrywany w stanie rozwiniętym, więc pasek z przyciskami spadał na środek i był przykrywany przez WebView Messengera (y=56).
- Naprawa: `.expanded .tab {display:none}`, nieprzezroczyste tło wyspy, grubszy szewron SVG wycentrowany.
- Weryfikacja: zrzuty samego WebView wyspy przez CDP (`--remote-debugging-port`, tylko na czas testu). Geometria: zwinięty szewron 47–73 px w pasku 120 px; rozwinięty pasek y=0, przycisk aplikacji wycentrowany.
- Uwaga bezpieczeństwa: `--remote-debugging-port` daje lokalnym procesom dostęp do WebView (w tym sesji Messengera). Używać wyłącznie do testów, nigdy w buildzie użytkownika.
- Narzędzie diagnostyczne: skrypt CDP w `C:/Users/podwo/AppData/Local/hermes/cache/scratch/` (nie w repo).

### Przezroczysty pasek i zaokrąglone rogi (2026-11-22)
- Usunięte tło wyspy w stanie rozwiniętym; przyciski aplikacji są samodzielnymi chipami.
- Zaokrąglenie dolnych rogów realizowane regionem Win32 (`SetWindowRgn`), bo CSS nie obejmuje child WebView Messengera. Region aktualizowany w każdej klatce animacji. Górna krawędź rozciągnięta poza okno, żeby górne rogi nie były zaokrąglone.
- Weryfikacja: build release, DOM (przezroczyste tła). Wizualnie niepotwierdzone przez agenta (zrzuty nie łapią WebView2).

---

## 🧠 Natychmiastowe uśpienie pamięci (2026-11-22)

### Zmiany
- **Usunięto**: mechanizm 5-minutowego opóźnienia (`DEFAULT_DELAY_SECS = 300`), zmienne środowiskowe `MESSENGER_ISLAND_AUTO_SUSPEND` / `MESSENGER_ISLAND_SUSPEND_SECS`, przełącznik „Uśpij Messengera po 5 min” z menu zasobnika.
- **Dodano**: automatyczne zwalnianie pamięci **zawsze** po zwinięciu wyspy (i na starcie):
  1. Ukrycie WebView `messenger` (`hide()`)
  2. Nawigacja na `about:blank` (`Navigate(PCWSTR("about:blank"))`) — niszczy DOM, V8 heap, WebSockety
  3. Po 200 ms (generation-based guard): `ICoreWebView2_3::TrySuspend` + czyszczenie Working Set
     - `SetProcessWorkingSetSizeEx(usize::MIN, usize::MAX, SETPROCESSWORKINGSETSIZEEX_FLAGS::default())` dla bieżącego procesu
     - To samo dla procesów potomnych `msedgewebview2.exe` (ToolHelp32 snapshot + `OpenProcess`)
- **Przy rozwinięciu**: `ICoreWebView2_3::Resume` + nawigacja na `START_URL` + pokazanie WebView po animacji
- **Niski profil pamięci WebView2** (`ICoreWebView2_19::SetMemoryUsageTargetLevel(LOW)`)
- **Wyłączone funkcje WebView2** (`ICoreWebView2Settings`): DevTools, status bar, zoom control, built-in error page

### Pliki zmienione
- `src-tauri/src/core/suspend.rs` — pełna przepisana logika (nowa struct `SuspendState`)
- `src-tauri/src/core/tray.rs` — usunięty przełącznik „Uśpij Messengera po 5 min”
- `src-tauri/src/core/window_manager.rs` — używa `SuspendState`, nawigacja `about:blank`/`START_URL` w `set_expanded`
- `src-tauri/src/modules/messenger.rs` — `pub configure_webview2_low_memory()` wywoływane w `window_manager::create()`
- `src-tauri/Cargo.toml` — dodane feature'y Windows: `Win32_System_Diagnostics_ToolHelp`, `Win32_System_Memory`, `Win32_System_Threading`
- `src-tauri/src/lib.rs` — `SuspendState::new()` zamiast `SuspendPolicy::from_env()`

### Weryfikacja
- `cargo test -p island-core` → 6 testów OK (geometria, filtry URL, fullscreen)
- `cargo build --release` → binarka ~3.1 MB, bez błędów/ostrzeżeń o nieużywanym kodzie
- `npm run build` → frontend kompiluje się bez błędów
