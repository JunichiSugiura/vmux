use crate::command::{AppCommand, WriteAppCommands};
use crate::confirm_close;
use crate::settings::AppSettings;
use crate::terminal::{PtyExited, Terminal};
use bevy::app::AppExit;
use bevy::ecs::message::Messages;
use bevy::prelude::*;
use muda::{Menu, MenuEvent};
use parking_lot::Mutex;
use std::sync::LazyLock;

static PENDING_MENU_EVENTS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[allow(dead_code)]
struct OsMenuResource(Menu);

pub struct OsMenuPlugin;

impl Plugin for OsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, forward_menu_events.in_set(WriteAppCommands));
    }
}

fn setup(world: &mut World) {
    let mut menu = Menu::new();
    AppCommand::build_native_root_menu(&mut menu).unwrap();

    #[cfg(target_os = "macos")]
    menu.init_for_nsapp();

    MenuEvent::set_event_handler(Some(|event: MenuEvent| {
        PENDING_MENU_EVENTS.lock().push(event.id.0.clone());
    }));

    world.insert_non_send_resource(OsMenuResource(menu));
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
        if event_id == "app_quit" {
            handle_quit_request(world);
        } else if let Some(cmd) = AppCommand::from_menu_id(event_id.as_str()) {
            world.resource_mut::<Messages<AppCommand>>().write(cmd);
        } else {
            warn!(len = event_id.len(), "unknown native menu item");
        }
    }
}

fn handle_quit_request(world: &mut World) {
    let should_confirm = world
        .get_resource::<AppSettings>()
        .and_then(|s| s.terminal.as_ref())
        .map_or(true, |t| t.confirm_close);

    if should_confirm {
        let mut query = world.query_filtered::<(), (With<Terminal>, Without<PtyExited>)>();
        let count = query.iter(world).count();

        if count > 0 && !confirm_close::confirm_quit_dialog(count) {
            return;
        }
    }

    world.resource_mut::<Messages<AppExit>>().write(AppExit::Success);
}
