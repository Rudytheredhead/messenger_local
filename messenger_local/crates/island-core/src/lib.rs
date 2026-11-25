//! Czysta logika Dynamic Island. Bez zależności od Tauri i Win32,
//! dzięki temu testuje się ją zwykłym `cargo test -p island-core`.

use url::Url;

/// Dozwolone domeny dla aplikacji wyspy (Messenger, Spotify, Discord, Gemini; tylko https).
pub const ALLOWED_DOMAINS: &[&str] = &[
    "facebook.com",
    "messenger.com",
    "fbcdn.net",
    "fbsbx.com",
    "spotify.com",
    "scdn.co",
    "spotifycdn.com",
    "discord.com",
    "discordapp.com",
    "discord.gg",
    "discordapp.net",
    "discord.media",
    "gemini.google.com",
    "accounts.google.com",
    "accounts.youtube.com",
    "myaccount.google.com",
    "consent.google.com",
    "gds.google.com",
    "ogs.google.com",
    "gstatic.com",
    "googleusercontent.com",
    "googleapis.com",
    "gvt1.com",
];

/// Rozmiar zwiniętego paska przy górnej krawędzi ekranu (logiczne px).
pub const COLLAPSED_SIZE: (f64, f64) = (120.0, 18.0);
/// Rozmiar rozwiniętej wyspy (logiczne px).
pub const EXPANDED_SIZE: (f64, f64) = (420.0, 620.0);
/// Rozmiar powiększonej wyspy (logiczne px).
pub const LARGE_SIZE: (f64, f64) = (960.0, 760.0);
/// Wysokość paska z przyciskami aplikacji w rozwiniętym widoku.
pub const BAR_HEIGHT: f64 = 56.0;
/// Odstęp od górnej krawędzi ekranu (zwinięta wyspa przylega do góry).
pub const TOP_MARGIN: f64 = 0.0;
/// Odstęp rozwiniętej wyspy od górnej krawędzi ekranu (logiczne px).
pub const EXPANDED_TOP_MARGIN: f64 = 12.0;

/// Filtr nawigacji: https, bez danych logowania w URL, standardowy port, domena Mety lub jej subdomena.
pub fn is_allowed_url(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    if !url.username().is_empty() || url.password().is_some() || url.port().is_some() {
        return false;
    }
    match url.host_str() {
        Some(host) => ALLOWED_DOMAINS
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}"))),
        None => false,
    }
}

/// Prostokąt w pikselach ekranu (jak RECT z Win32).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Okno pokrywa cały monitor (tryb pełnoekranowy / borderless).
pub fn covers(window: Rect, monitor: Rect) -> bool {
    window.left <= monitor.left
        && window.top <= monitor.top
        && window.right >= monitor.right
        && window.bottom >= monitor.bottom
}

/// Pozycja X wyśrodkowanej wyspy (logiczne px).
pub fn centered_x(screen_width: f64, width: f64) -> f64 {
    ((screen_width - width) / 2.0).round()
}

/// Interpolacja liniowa.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Łagodne wyjście (smoothstep) dla animacji rozmiaru; t w [0, 1].
pub fn ease_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(s: &str) -> bool {
        Url::parse(s).map(|u| is_allowed_url(&u)).unwrap_or(false)
    }

    #[test]
    fn allows_meta_https_domains() {
        assert!(allowed("https://www.facebook.com/messages"));
        assert!(allowed("https://facebook.com/"));
        assert!(allowed("https://www.messenger.com/t/1"));
        assert!(allowed("https://scontent.xx.fbcdn.net/v/a.jpg"));
        assert!(allowed("https://video.fbsbx.com/x"));
        assert!(allowed("https://open.spotify.com/"));
        assert!(allowed("https://i.scdn.co/image/abc"));
        assert!(allowed("https://discord.com/app"));
        assert!(allowed("https://cdn.discordapp.com/icons/1.png"));
        assert!(allowed("https://gemini.google.com/app"));
        assert!(allowed("https://accounts.google.com/"));
    }

    #[test]
    fn blocks_http_and_other_schemes() {
        assert!(!allowed("http://www.facebook.com/messages"));
        assert!(!allowed("javascript:alert(1)"));
        assert!(!allowed("file:///C:/Windows/win.ini"));
        assert!(!allowed("data:text/html,hi"));
    }

    #[test]
    fn blocks_lookalike_and_foreign_hosts() {
        assert!(!allowed("https://evilfacebook.com/"));
        assert!(!allowed("https://facebook.com.evil.org/"));
        assert!(!allowed("https://example.com/"));
        assert!(!allowed("https://fbcdn.net.attacker.io/"));
    }

    #[test]
    fn blocks_credentials_and_nonstandard_ports() {
        assert!(!allowed("https://user:pass@www.facebook.com/"));
        assert!(!allowed("https://www.facebook.com:8443/"));
    }

    #[test]
    fn detects_monitor_coverage() {
        let mon = Rect { left: 0, top: 0, right: 1920, bottom: 1080 };
        assert!(covers(Rect { left: 0, top: 0, right: 1920, bottom: 1080 }, mon));
        assert!(covers(Rect { left: -1, top: 0, right: 1921, bottom: 1080 }, mon));
        assert!(!covers(Rect { left: 0, top: 30, right: 1920, bottom: 1080 }, mon));
        assert!(!covers(Rect { left: 100, top: 100, right: 800, bottom: 600 }, mon));
    }

    #[test]
    fn geometry_helpers() {
        assert_eq!(centered_x(1920.0, 420.0), 750.0);
        assert_eq!(lerp(210.0, 420.0, 0.0), 210.0);
        assert_eq!(lerp(210.0, 420.0, 1.0), 420.0);
        assert_eq!(ease_out(0.0), 0.0);
        assert_eq!(ease_out(1.0), 1.0);
        assert!(ease_out(0.5) > 0.4 && ease_out(0.5) < 0.6);
    }
}
