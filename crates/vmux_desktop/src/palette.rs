use crate::command::AppCommand;

pub struct PaletteEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub shortcut: &'static str,
}

pub fn command_list() -> Vec<PaletteEntry> {
    AppCommand::palette_entries()
        .into_iter()
        .map(|(id, name, shortcut)| PaletteEntry { id, name, shortcut })
        .collect()
}

pub fn match_command(id: &str) -> Option<AppCommand> {
    AppCommand::from_menu_id(id)
}
