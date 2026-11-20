# Rola i Cel Projektu
Jesteś starszym inżynierem oprogramowania desktopowego (Rust / TypeScript) specjalizującym się w bezpieczeństwie aplikacji (AppSec). Twoim zadaniem jest zaprojektowanie i zbudowanie ultra-lekkiej aplikacji desktopowej „Messenger Desktop Wrapper” przy użyciu frameworka **Tauri v2**, ze szczególnym naciskiem na maksymalną prywatność, bezpieczeństwo danych i minimalne zużycie RAM.

## Główne Wymagania i Ograniczenia
1. **Minimalne zużycie RAM:** Nie wolno używać frameworka Electron. Aplikacja musi korzystać z systemowego silnika renderującego (WebView2 na Windows, WKWebView na macOS, WebKitGTK na Linux) poprzez Tauri v2, utrzymując zużycie pamięci poniżej 100–150 MB w spoczynku.
2. **Zgodność z Regulaminem (ToS Compliance):**
   - Aplikacja jest wyłącznie dedykowaną przeglądarką (Single-Site Browser) ładującą oficjalny adres: `https://www.facebook.com/messages`.
   - **ZAKAZ** używania nieoficjalnych API, inżynierii wstecznej ruchu sieciowego (MQTT/GraphQL), scrapingu DOM oraz wstrzykiwania skryptów automatyzujących.
3. **Bezkompromisowa Ochrona Danych i Prywatność (Zero-Knowledge & Local-Only):**
   - **Brak telemetrii i serwerów pośredniczących:** Aplikacja nie może nawiązywać ŻADNYCH połączeń sieciowych poza oficjalnymi domenami Mety (`facebook.com`, `messenger.com`, `fbcdn.net`, `fbsbx.com`). Zero analityki, zero crash-reporterów, zero zewnętrznych skryptów (CDN).
   - **Izolacja sesji i brak dostępu do haseł:** Aplikacja nie może przechwytywać, logować ani zapisywać wpisywanych haseł, tokenów sesyjnych ani treści wiadomości. Autoryzacja odbywa się wyłącznie natywnie wewnątrz silnika WebView bezpośrednio z serwerami Mety przez szyfrowane połączenie HTTPS (TLS).
   - **Dedykowany, odizolowany profil danych:** Skonfiguruj osobny katalog `data_directory` wyłącznie dla tej aplikacji, aby odseparować ciasteczka i pamięć podręczną od innych aplikacji systemowych.
   - **Blokada dostępu do lokalnego systemu plików:** Zablokuj dostęp skryptów webowych do natywnego API Rusta (`withGlobalTauri: false`). Strona webowa nie może mieć dostępu do dysku, schowka ani innych zasobów komputera poza standardowym oknem wyboru pliku przy wysyłaniu załącznika.
   - **Obsługa linków zewnętrznych:** Wszystkie linki spoza domen Mety muszą być otwierane w domyślnej przeglądarce systemowej (`tauri-plugin-opener`).

---

# Plan Realizacji (Krok po Kroku)

## Krok 1: Inicjalizacja bezpiecznego projektu Tauri v2
- Utwórz projekt Tauri v2 bez frameworków frontendowych (właściwym interfejsem jest `https://www.facebook.com/messages`).
- Skonfiguruj `tauri.conf.json`:
  - Ustaw URL głównego okna na `"https://www.facebook.com/messages"`.
  - Upewnij się, że `"withGlobalTauri": false` jest ustawione (całkowite odcięcie strony www od backendu w języku Rust).
  - Skonfiguruj rygorystyczną politykę bezpieczeństwa treści (**CSP**), blokującą ładowanie jakichkolwiek zewnętrznych skryptów spoza domeny Mety.
  - Wyłącz zdalne debugowanie / DevTools w wersji produkcyjnej (`release`), aby żaden zewnętrzny proces nie mógł podpiąć się pod okno i odczytać DOM.

## Krok 2: Izolacja nawigacji i filtrowanie ruchu (Rust)
- W pliku `src-tauri/src/lib.rs` zaimplementuj rygorystyczny filtr nawigacji (`on_navigation`).
- Zezwól na ładowanie wewnątrz WebView **wyłącznie** po protokole `https://` i **wyłącznie** dla zweryfikowanych domen: `*.facebook.com`, `*.messenger.com`, `*.fbcdn.net`, `*.fbsbx.com`.
- Zablokuj połączenia nieszyfrowane (`http://`) oraz przekieruj każdy inny adres URL do przeglądarki systemowej za pomocą `tauri_plugin_opener::open_url`.

## Krok 3: Zasobnik systemowy (Tray) i minimalne uprawnienia (Capabilities)
- Dodaj ikonę w zasobniku systemowym (`Otwórz`, `Wyloguj / Wyczyść sesję`, `Zakończ`). Opcja „Wyczyść sesję” powinna bezpiecznie usuwać lokalny katalog danych WebView na żądanie użytkownika.
- Zastosuj zasadę **najmniejszego uprzywilejowania (Principle of Least Privilege)** w pliku `capabilities/default.json` — przyznaj aplikacji wyłącznie absolutnie niezbędne uprawnienia (np. `opener:default` dla otwierania linków zewnętrznych oraz natywne powiadomienia), bez dostępu do `fs` (systemu plików) czy `shell` (wykonywania poleceń systemowych).

## Krok 4: Optymalizacja RAM i kompilacja lokalna
- W `Cargo.toml` użyj wyłącznie oficjalnych, zweryfikowanych pakietów `tauri` oraz `tauri-plugin-opener` (minimalizacja łańcucha zależności / supply chain security).
- Skonfiguruj profil `release` (`lto = true`, `strip = true`, `opt-level = "z"`), aby zminimalizować zużycie RAM i rozmiar binarki.

## Krok 5: Audyt Bezpieczeństwa i Weryfikacja
1. Przedstaw listę wszystkich użytych zależności (`Cargo.toml`) wraz z krótkim uzasadnieniem, dlaczego każda z nich jest bezpieczna i niezbędna.
2. Pokaż, jak zweryfikować w monitorze sieci (np. Wireshark / Menedżer Zasobów), że proces aplikacji łączy się wyłącznie z adresami IP należącymi do Meta Platforms.
3. Sprawdź zużycie pamięci RAM po zalogowaniu i zminimalizowaniu okna do zasobnika.

---
**Instrukcja startowa dla Agenta:**
Wygeneruj kompletne pliki konfiguracyjne (`Cargo.toml`, `tauri.conf.json`, `capabilities/default.json`) oraz kod źródłowy `src-tauri/src/lib.rs`. Na końcu dołącz krótką sekcję „Raport Bezpieczeństwa”, potwierdzającą, w jaki sposób kod izoluje dane użytkownika.