//! Moduł Messenger: izolowany WebView z oficjalną stroną `https://www.facebook.com/messages`.

use crate::modules::IslandModule;

pub const LABEL: &str = "messenger";
pub const START_URL: &str = "https://www.facebook.com/messages";

pub struct Messenger;

impl Messenger {
    pub fn new() -> Self {
        Self
    }
}

impl IslandModule for Messenger {
    fn id(&self) -> &'static str {
        LABEL
    }
    fn name(&self) -> &'static str {
        "Messenger"
    }
    fn icon(&self) -> &'static str {
        "💬"
    }
    fn priority(&self) -> u8 {
        100
    }
}
