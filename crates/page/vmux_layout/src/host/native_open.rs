use bevy::prelude::*;
use vmux_core::{PageMetadata, PageOpenError, PageOpenHandled, PageOpenSet, PageOpenTask};

use vmux_core::host::page::NativelyHosted;

use crate::cef::Browser;
use crate::warm_page::clear_stack_children;

pub trait HostedPage: Component + Default {
    const HOST: &'static str;
    const URL: &'static str;
    const TITLE: &'static str;
}

pub struct HostedPagePlugin<M: HostedPage>(std::marker::PhantomData<fn() -> M>);

impl<M: HostedPage> Default for HostedPagePlugin<M> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<M: HostedPage> Plugin for HostedPagePlugin<M> {
    fn build(&self, app: &mut App) {
        vmux_core::register_host_spawn(app, M::HOST);
        app.world_mut()
            .spawn(NativelyHosted::page(M::URL, M::TITLE));
        app.add_systems(
            Update,
            mark_hosted_view::<M>.after(PageOpenSet::HandleKnownPages),
        );
    }
}

fn mark_hosted_view<M: HostedPage>(
    views: Query<(Entity, &PageMetadata), (With<vmux_core::host::page::HostsPage>, Without<M>)>,
    mut commands: Commands,
) {
    for (entity, page) in &views {
        if page.url == M::URL {
            commands.entity(entity).try_insert(M::default());
        }
    }
}

pub struct NativeOpenPlugin;

impl Plugin for NativeOpenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            handle_native_page_open.in_set(PageOpenSet::HandleKnownPages),
        );
    }
}

type PendingPageOpen = (Without<PageOpenHandled>, Without<PageOpenError>);

fn handle_native_page_open(
    pages: Query<&NativelyHosted>,
    tasks: Query<(Entity, &PageOpenTask), PendingPageOpen>,
    children_q: Query<&Children>,
    mut commands: Commands,
) {
    let mut opened = std::collections::HashSet::new();

    for (task_entity, task) in &tasks {
        let Some(page) = pages.iter().find(|page| page.answers_for(&task.url)) else {
            continue;
        };
        if opened.insert(task.stack) {
            clear_stack_children(task.stack, &children_q, &mut commands);
            commands.entity(task.stack).insert(PageMetadata {
                url: page.url.to_string(),
                title: page.title.to_string(),
                ..default()
            });
            commands.spawn((
                Browser::native_page(&task.url, page.title),
                ChildOf(task.stack),
            ));
        }
        commands.entity(task_entity).insert(PageOpenHandled);
    }
}

#[cfg(test)]
mod tests {
    use vmux_core::host::page::NativelyHosted;

    #[test]
    fn a_trailing_slash_does_not_decide_which_page_was_asked_for() {
        let page = NativelyHosted::page("vmux://debug/", "Debug");

        assert!(page.answers_for("vmux://debug/"));
        assert!(page.answers_for("vmux://debug"));
    }

    #[test]
    fn a_longer_url_is_a_different_page() {
        let page = NativelyHosted::page("vmux://debug/", "Debug");

        assert!(!page.answers_for("vmux://debugger/"));
        assert!(!page.answers_for("vmux://debug/panel"));
    }

    #[test]
    fn a_subtree_page_claims_descendants_without_claiming_a_sibling() {
        let page = NativelyHosted::subtree("vmux://debug/", "Debug");

        assert!(page.answers_for("vmux://debug/panel"));
        assert!(!page.answers_for("vmux://debugger/"));
    }
}
