# To-Do: Messenger Island (Dynamic Island dla Windows 11)

Poprzedni plan (wrapper Messengera, kroki 1–5) jest w historii git (commit `a5b08b9`) i w `PROGRESS.md`.
Ten plik to aktualny plan. Statusy: ✅ zrobione · 🔄 w toku · ⏳ do zrobienia · 🧪 wymaga weryfikacji ręcznej.

## Filar 1 — Tryb pełnoekranowy (zero kolizji z grami i filmami)
- ✅ Wątek w tle co 500 ms (`core/fullscreen_guard.rs`).
- ✅ Wykrycie: `SHQueryUserNotificationState` (QUNS_BUSY / D3D_FULL_SCREEN / PRESENTATION_MODE) oraz aktywne okno pokrywające monitor (`GetForegroundWindow` + `GetWindowRect` + `GetMonitorInfoW`); pulpit i pasek zadań wykluczone.
- ✅ Automatyczne ukrycie wyspy (`fullscreen_hidden`) i przywrócenie po wyjściu z gry/filmu.
- ✅ Ręczny przełącznik „Nie przeszkadzaj (ukryj wyspę)” w zasobniku (`user_hidden`); ma pierwszeństwo.
- ✅ Test na żywo: pełny ekran działa (potwierdzone przez użytkownika); borderless w grze do sprawdzenia.

## Filar 2 — Architektura modułowa
- ✅ Rdzeń: geometria, animacja, wykrywanie pełnoekranowości, skrót, zasobnik (`core/`).
- ✅ Trait `IslandModule` + `ModuleRegistry` (`modules/mod.rs`): `id, name, icon, priority, badge`.
- ✅ Pigułka pokazuje moduł o najwyższym priorytecie z badge > 0 (`island-core::pick_status`, testowane).
- ✅ Pasek zakładek w rozwiniętej wyspie; kontener na lekkie moduły HTML/JS (`main#content`).
- ✅ Messenger jako moduł (`modules/messenger.rs`).
- ✅ Lazy loading dla innych modułów: nowe moduły bez WebView (natywny Rust lub HTML/JS). Messenger ma jedyny WebView (wymagany do działania).

## Filar 3 — RAM (< 120–150 MB)
- ✅ Brak Electron; WebView2 przez Tauri v2.
- ✅ Przezroczyste tło pigułki (bez białego kwadratu).
- ✅ Interfejs wyspy: czysty HTML5 + CSS3 + Vanilla TypeScript (bez React/Vue).
- ✅ Profil `release`: `lto`, `strip`, `opt-level = "z"`, `codegen-units = 1`, `panic = "abort"` (w root workspace).
- ✅ Usypianie widoku zwiniętego: animacja licznika tylko gdy są nieprzeczytane i wyspa zwinięta; ukryty WebView Messengera nie renderuje.
- 🧪 Pomiar RAM po zalogowaniu (cel < 150 MB), Menedżer zadań, suma drzewa `msedgewebview2.exe`.

## Filar 4 — Bezpieczeństwo i zgodność z regulaminem Mety
- ✅ Filtr nawigacji (https, domeny Mety, bez loginu w URL, standardowy port) — `island-core::is_allowed_url`, testowane.
- ✅ Obce linki → przeglądarka systemowa (`tauri-plugin-opener`); nowe okna nie tworzą dodatkowych WebView.
- ✅ Licznik nieprzeczytanych z tytułu karty (`on_document_title_changed`), bez DOM i bez API.
- ✅ `withGlobalTauri: false`; WebView Messengera nie ma żadnych uprawnień IPC (capability tylko dla `island-ui`).
- ✅ Zero telemetrii; dane sesji w `data_directory` aplikacji.
- ⏳ Dedykowana sekcja w PROGRESS.md po weryfikacji (sieć, RAM).

## Krok 1 — Struktura
- ✅ `src-tauri/src/{lib.rs, commands.rs, core/{window_manager,fullscreen_guard,tray,state}.rs, modules/{mod,messenger}.rs}`
- ✅ `crates/island-core` — czysta logika (bez Tauri), `cargo test -p island-core`.
- ✅ `src/` — interfejs wyspy (TypeScript + styles.css), bundlowany do `dist/`.

## Krok 2 — Okno wyspy (`core/window_manager.rs`)
- ✅ Bez ramki, przezroczyste, zawsze na wierzchu, bez paska zadań, pozycja: góra-środek (DPI-aware, z uwzględnieniem monitora).
- ✅ Animacja rozmiaru pigułka `210×36` ⇄ rozwinięta `420×620` (10 klatek, smoothstep).
- ✅ Messenger jako dziecko okna (`unstable` multiwebview) pod paskiem 40 px.
- ⚠️ Odchylenie od spec: spec mówi „auto-hide przy Focused(false)”. Wdrożone jest **zwinięcie do pigułki** (nie ukrycie), bo ukrywanie wyspy po każdym kliknięciu poza nią uniemożliwiałoby jej używanie. Ukrycie działa przez „Nie przeszkadzaj” i w grach.

## Krok 3 — Skróty i zasobnik
- ✅ Skrót globalny `Ctrl+Alt+I` (rozwija/zwija, pokazuje jeśli ukryta).
- ✅ Zasobnik: Pokaż wyspę · Nie przeszkadzaj (ukryj) · Wyloguj / Wyczyść sesję · Zakończ.

## Krok 4 — Profil release i build
- ✅ Profil release w root workspace (bez duplikatu w podprojekcie).
- ✅ `frontendDist` = `dist/`, generowany przez `npm run build` (`beforeBuildCommand`).
- ✅ Wyłączona konsola w release (`windows_subsystem = "windows"`) — brak okna PowerShell.
- ⏳ Instalator (`cargo tauri build`, MSI/NSIS) — nie uruchamiany.

## Krok 5 — Audyt i weryfikacja
- ✅ Testy czystej logiki: `cargo test -p island-core` (8/8).
- ⚠️ `cargo test` dla binarki Tauri: `STATUS_ENTRYPOINT_NOT_FOUND` na toolchainie GNU (znane, patrz PROGRESS.md).
- ✅ Build release i start procesu (patrz PROGRESS.md, sekcja weryfikacji).
- 🧪 Ręcznie: logowanie, animacja, skrót, zasobnik, gra/film, RAM, ruch sieciowy.


## Zmiana wyglądu (2026-11-22)
- ✅ Zwinięta wyspa: cienki pasek przy górnej krawędzi ekranu (120×18 px, szewron ⌄, licznik nieprzeczytanych w rogu).
- ✅ Rozwinięta wyspa: rząd okrągłych przycisków aplikacji u góry (na razie tylko Messenger), przycisk zwijania ⌃, pod spodem okno aplikacji (420×620 px).
- ✅ Wymiary w `island-core` (`COLLAPSED_SIZE`, `BAR_HEIGHT = 56`, `TOP_MARGIN = 0`).
- 🧪 Wizualnie niepotwierdzone po stronie agenta: zrzuty ekranu i `PrintWindow` nie pokazują treści WebView2 (wyszła czerń/brak). Wymaga oglądu na ekranie.

### Poprawki wyglądu (2026-11-22, druga runda)
- ✅ Strzałka zwinięta: gruby szewron SVG (stroke 3.5), wycentrowany w pasku 120×18.
- ✅ Przycisk zwijania w rozwiniętym pasku: ten sam szewron, skierowany w górę.
- ✅ Czarny artefakt usunięty: zwinięty pasek `.tab` był ukrywany tylko przy zwinięciu; teraz ukrywany także przy rozwinięciu (`.expanded .tab`). Przy `flex:1` pasek aplikacji spadał na środek i był zasłonięty przez WebView Messengera.
- ✅ Tło wyspy nieprzezroczyste (`#17171c`), bez `backdrop-filter` — przezroczyste okno z blurem dawało czarne artefakty.
- ✅ Weryfikacja przez CDP WebView wyspy (zrzut samego interfejsu): pasek na y=0, przycisk aplikacji wycentrowany, brak artefaktu.
- ⚠️ Instancja testowa z `--remote-debugging-port` była zatrzymana po weryfikacji; port nie jest częścią normalnego uruchomienia.

### Przezroczysty pasek i zaokrąglone rogi (2026-11-22, trzecia runda)
- ✅ Usunięte tło wyspy w stanie rozwiniętym (`rgba(0,0,0,0)`): przyciski aplikacji wiszą w powietrzu, każdy we własnym chipie z cieniem.
- ✅ Zaokrąglone dolne rogi okna regionem Win32 (`SetWindowRgn` + `CreateRoundRectRgn`), promień 10 px zwinięty / 18 px rozwinięty. Region obcina też WebView Messengera, którego CSS by nie objął. Górna krawędź przylega do ekranu.
- ✅ Build release przechodzi; w DOM potwierdzone przezroczyste tło.
- 🧪 Nie potwierdzone wizualnie przez agenta: zrzut ekranu nie łapie treści WebView2 (jak wcześniej). Do potwierdzenia na ekranie.
