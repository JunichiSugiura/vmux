pub const PAGE_URL: &str = "vmux://cheatsheet/";
pub const EVENT: &str = "cheatsheet";

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct CheatsheetEvent {
    pub groups: Vec<ShortcutGroup>,
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct ShortcutGroup {
    pub name: String,
    pub entries: Vec<ShortcutEntry>,
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct ShortcutEntry {
    pub name: String,
    pub shortcuts: Vec<String>,
}

#[cfg(host)]
mod host;
#[cfg(host)]
pub use host::CheatsheetPlugin;

#[cfg(ui)]
pub mod page;
