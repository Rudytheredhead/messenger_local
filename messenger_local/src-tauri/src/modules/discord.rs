//! Moduł Discord: izolowany WebView z oficjalną stroną `https://discord.com/app`.

use crate::modules::IslandModule;

pub const LABEL: &str = "discord";
pub const START_URL: &str = "https://discord.com/app";

pub struct Discord;

impl Discord {
    pub fn new() -> Self {
        Self
    }
}

impl IslandModule for Discord {
    fn id(&self) -> &'static str {
        LABEL
    }
    fn name(&self) -> &'static str {
        "Discord"
    }
    fn icon(&self) -> &'static str {
        "🎮"
    }
    fn priority(&self) -> u8 {
        95
    }
}
