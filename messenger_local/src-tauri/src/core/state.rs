use std::sync::Mutex;

use crate::modules::{antigravity, discord, gemini, messenger, spotify};

/// Stan wyspy. `user_hidden` = ręczny „Nie przeszkadzać”, `fullscreen_hidden` = automatyczne ukrycie w grze/filmie,
/// `webview_fullscreen` = pełny ekran elementu wewnątrz WebView (np. Discord stream), `*_running` = czy dana aplikacja jest aktualnie uruchomiona.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IslandFlags {
    pub expanded: bool,
    pub large: bool,
    pub dialog_open: bool,
    pub webview_fullscreen: bool,
    pub user_hidden: bool,
    pub fullscreen_hidden: bool,
    pub messenger_running: bool,
    pub spotify_running: bool,
    pub discord_running: bool,
    pub gemini_running: bool,
    pub antigravity_running: bool,
    pub active_module: &'static str,
}

impl Default for IslandFlags {
    fn default() -> Self {
        Self {
            expanded: false,
            large: true,
            dialog_open: false,
            webview_fullscreen: false,
            user_hidden: false,
            fullscreen_hidden: false,
            messenger_running: false,
            spotify_running: false,
            discord_running: false,
            gemini_running: false,
            antigravity_running: false,
            active_module: messenger::LABEL,
        }
    }
}

impl IslandFlags {
    pub fn is_module_running(&self, id: &str) -> bool {
        match id {
            messenger::LABEL => self.messenger_running,
            spotify::LABEL => self.spotify_running,
            discord::LABEL => self.discord_running,
            gemini::LABEL => self.gemini_running,
            antigravity::LABEL => self.antigravity_running,
            _ => false,
        }
    }

    pub fn set_module_running(&mut self, id: &str, running: bool) -> bool {
        match id {
            messenger::LABEL => {
                let changed = self.messenger_running != running;
                self.messenger_running = running;
                changed
            }
            spotify::LABEL => {
                let changed = self.spotify_running != running;
                self.spotify_running = running;
                changed
            }
            discord::LABEL => {
                let changed = self.discord_running != running;
                self.discord_running = running;
                changed
            }
            gemini::LABEL => {
                let changed = self.gemini_running != running;
                self.gemini_running = running;
                changed
            }
            antigravity::LABEL => {
                let changed = self.antigravity_running != running;
                self.antigravity_running = running;
                changed
            }
            _ => false,
        }
    }
}

#[derive(Default)]
pub struct IslandStore(pub Mutex<IslandFlags>);
