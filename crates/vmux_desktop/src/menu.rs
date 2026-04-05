use crate::command::{NewSpaceCommand, SplitHorizontallyCommand, SplitVerticallyCommand};
use bevy::prelude::*;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use parking_lot::Mutex;
use std::sync::LazyLock;

static PENDING_MENU_EVENTS: LazyLock<Mutex<Vec<String>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[allow(dead_code)]
struct NativeMenuResource(Menu);

pub struct NativeMenuPlugin;

impl Plugin for NativeMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, forward_menu_events);
    }
}

fn setup(world: &mut World) {
    let menu = Menu::new();

    let app_menu = Submenu::new("Vmux", true);
    app_menu
        .append_items(&[
            &PredefinedMenuItem::about(None, None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::quit(None),
        ])
        .unwrap();

    let space_menu = Submenu::new("Space", true);
    space_menu
        .append_items(&[&MenuItem::with_id("new_space", "New Space", true, None)])
        .unwrap();

    let pane_menu = Submenu::new("Pane", true);
    pane_menu
        .append_items(&[
            &MenuItem::with_id("split_vertically", "Split Vertically", true, None),
            &MenuItem::with_id("split_horizontally", "Split Horizontally", true, None),
        ])
        .unwrap();

    menu.append_items(&[&app_menu, &space_menu, &pane_menu])
        .unwrap();

    #[cfg(target_os = "macos")]
    menu.init_for_nsapp();

    MenuEvent::set_event_handler(Some(|event: MenuEvent| {
        PENDING_MENU_EVENTS.lock().push(event.id.0.clone());
    }));

    world.insert_non_send_resource(NativeMenuResource(menu));
}

fn forward_menu_events(world: &mut World) {
    let drained = {
        let mut events = PENDING_MENU_EVENTS.lock();
        if events.is_empty() {
            return;
        }
        std::mem::take(&mut *events)
    };

    for event_id in drained {
        match event_id.as_str() {
            "new_space" => world.trigger(NewSpaceCommand),
            "split_vertically" => world.trigger(SplitVerticallyCommand),
            "split_horizontally" => world.trigger(SplitHorizontallyCommand),
            _ => warn!(
                len = event_id.len(),
                "unknown native menu item"
            ),
        }
    }
}
