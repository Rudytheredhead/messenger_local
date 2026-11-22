//! Rejestr modułów wyspy. Nowy widżet = nowy plik w `modules/` + wpis w `lib.rs`.

pub mod messenger;

use std::sync::Arc;

use serde::Serialize;

/// Wspólny interfejs modułu. Moduły z natywnym WebView (Messenger) i moduły lekkie (HTML/JS, Rust)
/// implementują ten sam trait; rdzeń nie zna ich szczegółów.
pub trait IslandModule: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    /// Wyższy priorytet wygrywa w pigułce, gdy kilka modułów ma badge > 0.
    fn priority(&self) -> u8;
}

#[derive(Serialize, Clone, Debug)]
pub struct ModuleInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub priority: u8,
}

pub struct ModuleRegistry {
    modules: Vec<Arc<dyn IslandModule>>,
}

impl ModuleRegistry {
    pub fn new(modules: Vec<Arc<dyn IslandModule>>) -> Self {
        Self { modules }
    }

    pub fn list(&self) -> Vec<ModuleInfo> {
        self.modules
            .iter()
            .map(|m| ModuleInfo {
                id: m.id(),
                name: m.name(),
                icon: m.icon(),
                priority: m.priority(),
            })
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn IslandModule>> {
        self.modules.iter().find(|m| m.id() == id)
    }
}