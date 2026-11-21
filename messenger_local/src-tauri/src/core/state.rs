use std::sync::Mutex;

/// Stan wyspy. `user_hidden` = ręczny „Nie przeszkadzać”, `fullscreen_hidden` = automatyczne ukrycie w grze/filmie.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IslandFlags {
    pub expanded: bool,
    pub user_hidden: bool,
    pub fullscreen_hidden: bool,
}

#[derive(Default)]
pub struct IslandStore(pub Mutex<IslandFlags>);
