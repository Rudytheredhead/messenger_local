//! Moduł Spotify: izolowany WebView z oficjalną stroną `https://open.spotify.com`.

use crate::modules::IslandModule;

pub const LABEL: &str = "spotify";
pub const START_URL: &str = "https://open.spotify.com";

pub struct Spotify;

impl Spotify {
    pub fn new() -> Self {
        Self
    }
}

impl IslandModule for Spotify {
    fn id(&self) -> &'static str {
        LABEL
    }
    fn name(&self) -> &'static str {
        "Spotify"
    }
    fn icon(&self) -> &'static str {
        "🎵"
    }
    fn priority(&self) -> u8 {
        90
    }
}
