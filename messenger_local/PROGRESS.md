# Messenger Local - Progress

## 2026-11-22

### Zrealizowane etapy

#### 1. Podstawowa architektura (Tauri v2)
- Workspace Cargo z dwoma crate'ami: `messenger_local` (app) + `island-core` (pure logic)
- Frontend Vanilla TypeScript + esbuild
- Dynamic Island na Windows: zwijany pasek (120×18) + rozwinięta wyspa (420×620)
- Globalny skrót `Ctrl+Alt+I`, ikona w zasobniku systemowym

#### 2. Messenger WebView
- Izolowany profil WebView2 (`webview-profile/`) — ciasteczka/sesje oddzielone od przeglądarki
- Strict CSP: tylko domeny Meta (`facebook.com`, `messenger.com`, `fbcdn.net`, `fbsbx.com` + WSS)
- Nawigacja: domeny Mety w WebView, reszta → systemowa przeglądarka (`opener`)
- Nowe okna (`target=_blank`): Meta w tym samym WebView, inne zewnętrznie

#### 3. Stan i animacje
- `IslandState`: `expanded`, `user_hidden`, `fullscreen_hidden`
- Animacja rozmiaru (ease-out, 10 kroków, 16ms/klatka)
- Win32 region (zaokrąglone dolne rogi) — obcina też child WebView
- Auto-zwijanie przy utracie fokusu, fullscreen guard (ukrywa w grach/filmach)

#### 4. Czyszczenie kodu (usunięto)
- `tauri-plugin-notification` ( nieużywane)
- Licznik nieprzeczytanych (`unread`, `badge`, `Status`, `pick_status`, `unread_from_title`) z backend + frontend
- `StatusDto` z `IslandState`, element `#tab-badge` z HTML, style `.badge` + `@keyframes pulse`
- Testy jednostkowe `island-core` dla usuniętych funkcji

#### 5. Zarządzanie pamięcią (Natychmiastowe uśpienie)
- **Usunięto**: opóźnienie 5 min (`DEFAULT_DELAY_SECS`), zmienne środowiskowe `MESSENGER_ISLAND_AUTO_SUSPEND` / `MESSENGER_ISLAND_SUSPEND_SECS`, przełącznik z menu tray
- **Dodano**: automatyczne zwalnianie pamięci **zawsze** po zwinięciu wyspy:
  1. Ukrycie WebView `messenger`
  2. Nawigacja na `about:blank` (zniszczy DOM, V8 heap, WebSockety)
  3. Po 200 ms: `ICoreWebView2_3::TrySuspend` + czyszczenie Working Set
     - `SetProcessWorkingSetSizeEx(usize::MIN, usize::MAX)` dla bieżącego procesu
     - To samo dla procesów potomnych `msedgewebview2.exe` (ToolHelp32 snapshot)
- **Przy rozwinięciu**: `Resume` + nawigacja na `START_URL` + pokazanie WebView
- **Niski profil pamięci WebView2** (`ICoreWebView2_19::SetMemoryUsageTargetLevel(LOW)`)
- **Wyłączone funkcje WebView2**: DevTools, status bar, zoom control, built-in error page

#### 6. Weryfikacja
- `cargo test -p island-core` → 6 testów OK (geometria, filtry URL, fullscreen)
- `cargo build --release` → binarka ~3.1 MB, bez błędów/ostrzeżeń o nieużywanym kodzie
- `npm run build` → frontend kompiluje się bez błędów

---

## Następne kroki (backlog)

- [ ] Testy manualne: RAM przy zwiniętej/rozwiniętej wyspie (Task Manager)
- [ ] Persistence: zapamiętywanie `user_hidden` między restartami
- [ ] Ikona aplikacji (`.ico` / `.png` w `src-tauri/icons/`)
- [ ] Podpisanie instalatora (code signing)
- [ ] CI/CD: GitHub Actions (build + test na Windows runner)
- [ ] README: sekcja "Jak działa uśpienie pamięci"