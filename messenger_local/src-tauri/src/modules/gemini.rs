//! Moduł Gemini: izolowany WebView z oficjalną stroną `https://gemini.google.com/app`.

use crate::modules::IslandModule;

pub const LABEL: &str = "gemini";
pub const START_URL: &str = "https://gemini.google.com/app";

pub struct Gemini;

impl Gemini {
    pub fn new() -> Self {
        Self
    }
}

impl IslandModule for Gemini {
    fn id(&self) -> &'static str {
        LABEL
    }
    fn name(&self) -> &'static str {
        "Gemini"
    }
    fn icon(&self) -> &'static str {
        "✦"
    }
    fn priority(&self) -> u8 {
        96
    }
}
