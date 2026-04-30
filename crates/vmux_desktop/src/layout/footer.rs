use super::Open;
use crate::command::{AppCommand, FooterCommand, ReadAppCommands};
use bevy::prelude::*;
use vmux_footer::{FOOTER_HEIGHT_PX, Footer};

pub(crate) struct FooterLayoutPlugin;

impl Plugin for FooterLayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_footer_toggle.in_set(ReadAppCommands))
            .add_systems(
                PostUpdate,
                sync_footer_visibility.before(bevy::ui::UiSystems::Layout),
            );
    }
}

fn handle_footer_toggle(
    mut reader: MessageReader<AppCommand>,
    footer_q: Query<(Entity, Has<Open>), With<Footer>>,
    mut commands: Commands,
) {
    for cmd in reader.read() {
        if matches!(cmd, AppCommand::Footer(FooterCommand::Toggle)) {
            for (entity, is_open) in &footer_q {
                if is_open {
                    commands.entity(entity).remove::<Open>();
                } else {
                    commands.entity(entity).insert(Open);
                }
            }
        }
    }
}

fn sync_footer_visibility(
    mut footer_q: Query<(&mut Visibility, &mut Node), With<Footer>>,
    added: Query<Entity, (With<Footer>, Added<Open>)>,
    mut removed: RemovedComponents<Open>,
) {
    for entity in &added {
        if let Ok((mut vis, mut node)) = footer_q.get_mut(entity) {
            *vis = Visibility::Inherited;
            node.display = Display::Flex;
            node.height = Val::Px(FOOTER_HEIGHT_PX);
        }
    }

    for entity in removed.read() {
        if let Ok((mut vis, mut node)) = footer_q.get_mut(entity) {
            *vis = Visibility::Hidden;
            node.display = Display::None;
            node.height = Val::Px(0.0);
        }
    }
}
