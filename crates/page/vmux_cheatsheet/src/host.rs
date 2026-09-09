use crate::{CheatsheetEvent, EVENT, PAGE_URL, ShortcutEntry, ShortcutGroup};
use bevy::prelude::*;
use bevy_cef::prelude::{BinHostEmitEvent, BinReceive, Browsers};
use std::collections::{BTreeMap, HashMap};
use vmux_command::shortcut::Keymap;
use vmux_command::{AppCommand, ResolvedLocale, localized_command_name};
use vmux_core::page::PageReady;
use vmux_layout::native_open::{HostedPage, HostedPagePlugin};
use vmux_ui::i18n::Locale;

pub struct CheatsheetPlugin;

impl Plugin for CheatsheetPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().spawn(PAGE_MANIFEST);
        app.add_plugins(HostedPagePlugin::<Cheatsheet>::default())
            .add_observer(send_cheatsheet);
    }
}

pub const PAGE_MANIFEST: vmux_core::page::PageManifest = vmux_core::page::PageManifest {
    host: "cheatsheet",
    title: "Keyboard Shortcuts",
    title_message_id: Some("cheatsheet-title"),
    replaces_command: None,
    keywords: &["keyboard", "shortcut", "keymap", "cheatsheet"],
    icon: Some(vmux_core::BuiltinIcon::Keyboard),
    command_bar: true,
};

#[derive(Component, Default)]
struct Cheatsheet;

impl HostedPage for Cheatsheet {
    const HOST: &'static str = "cheatsheet";
    const URL: &'static str = PAGE_URL;
    const TITLE: &'static str = "Keyboard Shortcuts";
}

fn send_cheatsheet(
    trigger: On<BinReceive<PageReady>>,
    views: Query<(), With<Cheatsheet>>,
    keymap: Res<Keymap>,
    locale: Option<Res<ResolvedLocale>>,
    browsers: NonSend<Browsers>,
    mut commands: Commands,
) {
    let webview = trigger.event().webview;
    if !views.contains(webview) || !browsers.can_emit_to(&webview) {
        return;
    }
    let locale = locale
        .as_deref()
        .map(|locale| locale.0.clone())
        .unwrap_or_else(Locale::preferred);
    let payload = CheatsheetEvent::of(&keymap, &locale);
    commands.trigger(BinHostEmitEvent::from_rkyv(webview, EVENT, &payload));
}

impl CheatsheetEvent {
    fn of(keymap: &Keymap, locale: &Locale) -> Self {
        let mut labels = HashMap::new();
        for (id, fallback) in AppCommand::shortcut_labels() {
            labels.insert(id, localized_command_name(locale.as_str(), id, fallback));
        }

        let mut grouped: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        for binding in keymap.bindings() {
            if AppCommand::from_shortcut_id(&binding.command).is_none() {
                continue;
            }
            let Some(label) = labels.get(binding.command.as_str()) else {
                continue;
            };
            let (group, name) = label
                .split_once(" > ")
                .map(|(group, name)| (group.to_string(), name.to_string()))
                .unwrap_or_else(|| (locale.translate("cheatsheet-general"), label.clone()));
            let shortcuts = grouped.entry(group).or_default().entry(name).or_default();
            let shortcut = binding.shortcut.display();
            if !shortcuts.contains(&shortcut) {
                shortcuts.push(shortcut);
            }
        }

        let groups = grouped
            .into_iter()
            .map(|(name, entries)| ShortcutGroup {
                name,
                entries: entries
                    .into_iter()
                    .map(|(name, shortcuts)| ShortcutEntry { name, shortcuts })
                    .collect(),
            })
            .collect();
        Self { groups }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vmux_core::{PageMetadata, PageOpenId, PageOpenTask};
    use vmux_layout::native_open::NativeOpenPlugin;

    #[test]
    fn event_lists_hidden_and_visible_shortcuts() {
        let event = CheatsheetEvent::of(&Keymap::defaults(), &Locale::from("en-US"));
        let names = event
            .groups
            .iter()
            .flat_map(|group| group.entries.iter())
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.iter().any(|name| name.contains("Rotate Forward")));
        assert!(names.iter().any(|name| name.contains("Open in New Tab")));
    }

    #[test]
    fn page_open_spawns_cheatsheet_view() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(NativeOpenPlugin)
            .add_plugins(HostedPagePlugin::<Cheatsheet>::default());
        let stack = app.world_mut().spawn_empty().id();
        app.world_mut().spawn(PageOpenTask {
            id: PageOpenId::new(),
            stack,
            url: PAGE_URL.to_string(),
            request_id: None,
        });

        app.update();

        let title = app
            .world_mut()
            .query_filtered::<&PageMetadata, With<Cheatsheet>>()
            .single(app.world())
            .expect("cheatsheet webview spawned")
            .title
            .clone();
        assert_eq!(title, "Keyboard Shortcuts");
    }
}
