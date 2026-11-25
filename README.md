# TopIsland

Lekka, pływająca wyspa u góry ekranu dla Windows 10/11 (zbudowana w **Tauri v2** + **WebView2** + **Rust**) w stylistyce *Dark Glassmorphism*. Skupia komunikatory, muzykę i asystentów AI w jednym wysuwanym oknie, utrzymując niskie zużycie pamięci RAM.

## Funkcje i aplikacje

- **Messenger** — natywny WebView z izolowanymi sesjami i bezpiecznym filtrowaniem domen (`island-core`).
- **Discord** — pełne wsparcie dla połączeń głosowych, udostępniania ekranu (ochrona okna podczas wybierania ekranu) oraz trybu pełnoekranowego wideo/streamów.
- **Spotify** — odtwarzacz webowy z obsługą DRM Widevine i autoplay.
- **Gemini** — szybki dostęp do `gemini.google.com/app` jako osobny moduł na pasku.
- **Antigravity** *(opcjonalnie, wykrywany automatycznie)* — natywny czat bez WebView2 oparty na lokalnym CLI `agy.exe`, z oddzielnym wyborem **Modelu** i **Thinking effort** (`Low` / `Medium` / `High` / `XHigh`), historią wątków i załączaniem plików/folderów (przeciągnij i upuść lub okna dialogowe). Jeśli na komputerze nie ma `agy.exe`, moduł jest automatycznie ukrywany.

## Zarządzanie pamięcią

- Wszystkie aplikacje webowe startują wyłączone (`about:blank` + stan uśpienia `TrySuspend`), dzięki czemu wyspa nie zużywa zbędnego RAM-u w tle.
- Każdą aplikację można uruchomić kliknięciem lub klawiszem `Enter`, a zamknąć przyciskiem `×` na jej ikonie.
- Aplikacja automatycznie chowa zwiniętą pigułkę podczas korzystania z gier i filmów na pełnym ekranie.

## Skróty klawiszowe

| Skrót | Działanie |
|---|---|
| `Ctrl` + `~` (tylda pod `Esc`) / `Ctrl` + `Alt` + `I` | Wysuwa lub chowa wyspę (działa także w aplikacjach pełnoekranowych) |
| `Tab` / `Shift` + `Tab` | Przełącza pomiędzy aplikacjami na pasku wyspy |
| `Enter` | Uruchamia aktualnie wybraną, wyłączoną aplikację |
| `Esc` | Zamyka otwarte menu lub zwija wyspę (w trybie pełnoekranowym wychodzi z pełnego ekranu wideo) |

## Udostępnianie znajomym

Aplikacja nie zawiera żadnych haseł ani tokenów — wszystkie sesje logowania oraz historia czatów są przechowywane wyłącznie lokalnie na komputerze użytkownika w `%LOCALAPPDATA%\com.messenger.local`.

Aby uruchomić **TopIsland** na innym komputerze z Windows 10/11 (z zainstalowanym WebView2 Runtime):
1. Skopiuj do jednego folderu oba pliki z `messenger_local\target\release\`:
   - `TopIsland.exe`
   - `WebView2Loader.dll`
2. Przy pierwszym uruchomieniu Windows SmartScreen może wyświetlić ostrzeżenie dla niepodpisanego pliku — wybierz **Więcej informacji → Uruchom mimo to**.

## Budowanie ze źródeł

Wymagania: **Node.js** (20+), **Rust** (`stable-x86_64-pc-windows-gnu` lub MSVC) oraz **WebView2 Runtime**.

```powershell
cd messenger_local
npm install
npm run build
cargo test -p island-core
cargo build --release
```

Gotowe pliki wyjściowe znajdują się w `messenger_local\target\release\TopIsland.exe` oraz `messenger_local\target\release\WebView2Loader.dll`.
