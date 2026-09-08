use bevy::prelude::*;
use bevy_world_serialization::WorldFilter;
use moonshine_save::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;
use vmux_core::{Bookmark, BookmarkOrder, Collapsed, Folder, Order, PageMetadata, Pin, Uuid};
use vmux_layout::LayoutStartupSet;

pub(crate) struct BookmarkPersistencePlugin;

impl Plugin for BookmarkPersistencePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<OfferedBookmarkDefaults>()
            .init_resource::<OfferedBookmarkDefaults>()
            .init_resource::<BookmarkAutoSave>()
            .add_observer(save_on::<SaveWorld<BookmarkFilter>>)
            .add_observer(load_on::<LoadWorld<BookmarkFilter>>)
            .add_observer(seed_default_bookmarks_after_load)
            .add_systems(
                Startup,
                load_bookmarks_on_startup.after(LayoutStartupSet::Persistence),
            )
            .add_systems(
                PostUpdate,
                (
                    migrate_legacy_bookmark_order,
                    mark_bookmarks_dirty,
                    autosave_bookmarks,
                )
                    .chain(),
            );
    }
}

type BookmarkFilter = Or<(With<Pin>, With<Bookmark>, With<Folder>)>;

pub(crate) fn bookmarks_path() -> PathBuf {
    vmux_core::profile::profile_dir().join("bookmarks.ron")
}

fn bookmark_scene_filter() -> WorldFilter {
    WorldFilter::deny_all()
        .allow::<ChildOf>()
        .allow::<Children>()
        .allow::<Name>()
        .allow::<Pin>()
        .allow::<Bookmark>()
        .allow::<Folder>()
        .allow::<Collapsed>()
        .allow::<Uuid>()
        .allow::<BookmarkOrder>()
        .allow::<PageMetadata>()
}

fn bookmark_resource_filter() -> WorldFilter {
    WorldFilter::deny_all().allow::<OfferedBookmarkDefaults>()
}

fn save_bookmarks_to_path(commands: &mut Commands, path: PathBuf) {
    if vmux_core::profile::is_test_session() {
        return;
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut save = SaveWorld::<BookmarkFilter>::into_file(path);
    save.components = bookmark_scene_filter();
    save.resources = bookmark_resource_filter();
    commands.trigger_save(save);
}

fn load_bookmarks_on_startup(
    settings: Res<vmux_setting::AppSettings>,
    pins: Query<&PageMetadata, With<Pin>>,
    orders: Query<&BookmarkOrder, BookmarkFilter>,
    mut offered: ResMut<OfferedBookmarkDefaults>,
    mut auto: ResMut<BookmarkAutoSave>,
    mut commands: Commands,
) {
    if vmux_core::profile::is_test_session() {
        return;
    }
    let path = bookmarks_path();
    if !path.exists() {
        BookmarkDefaults::of(&settings.browser.bookmarks).seed(
            &pins,
            &orders,
            &mut offered,
            &mut auto,
            &mut commands,
        );
        return;
    }
    commands.insert_resource(BookmarkLoadPending);
    commands.trigger_load(LoadWorld::<BookmarkFilter>::from_file(path));
}

fn seed_default_bookmarks_after_load(
    _trigger: On<Loaded>,
    pending: Option<Res<BookmarkLoadPending>>,
    settings: Res<vmux_setting::AppSettings>,
    pins: Query<&PageMetadata, With<Pin>>,
    orders: Query<&BookmarkOrder, BookmarkFilter>,
    mut offered: ResMut<OfferedBookmarkDefaults>,
    mut auto: ResMut<BookmarkAutoSave>,
    mut commands: Commands,
) {
    if pending.is_none() {
        return;
    }
    BookmarkDefaults::of(&settings.browser.bookmarks).seed(
        &pins,
        &orders,
        &mut offered,
        &mut auto,
        &mut commands,
    );
    commands.remove_resource::<BookmarkLoadPending>();
}

#[derive(Resource)]
struct BookmarkLoadPending;

#[derive(Resource, Reflect, Default, Clone, Debug, PartialEq, Eq)]
#[reflect(Resource)]
struct OfferedBookmarkDefaults {
    urls: Vec<String>,
}

impl OfferedBookmarkDefaults {
    fn claim_new(&mut self, defaults: &[String]) -> Vec<String> {
        let mut offered = self
            .urls
            .iter()
            .map(|url| BookmarkDefaults::key(url))
            .collect::<HashSet<_>>();
        let mut claimed = Vec::new();
        for url in defaults {
            let key = BookmarkDefaults::key(url);
            if key.is_empty() || !offered.insert(key) {
                continue;
            }
            self.urls.push(url.clone());
            claimed.push(url.clone());
        }
        claimed
    }
}

struct BookmarkDefaults<'a> {
    urls: &'a [String],
}

impl<'a> BookmarkDefaults<'a> {
    fn of(urls: &'a [String]) -> Self {
        Self { urls }
    }

    fn key(url: &str) -> String {
        url.trim().trim_end_matches('/').to_ascii_lowercase()
    }

    fn seed(
        self,
        pins: &Query<&PageMetadata, With<Pin>>,
        orders: &Query<&BookmarkOrder, BookmarkFilter>,
        offered: &mut OfferedBookmarkDefaults,
        auto: &mut BookmarkAutoSave,
        commands: &mut Commands,
    ) {
        let claimed = offered.claim_new(self.urls);
        if claimed.is_empty() {
            return;
        }
        let mut pinned = pins
            .iter()
            .map(|metadata| Self::key(&metadata.url))
            .collect::<HashSet<_>>();
        let mut next_order = orders
            .iter()
            .map(|order| order.0)
            .max()
            .map_or(0, |order| order.saturating_add(1));
        for url in claimed {
            if !pinned.insert(Self::key(&url)) {
                continue;
            }
            commands.spawn((
                Pin,
                Uuid(uuid::Uuid::new_v4().to_string()),
                PageMetadata {
                    title: url.clone(),
                    url,
                    icon: vmux_core::PageIcon::None,
                    bg_color: None,
                },
                BookmarkOrder(next_order),
            ));
            next_order = next_order.saturating_add(1);
        }
        auto.dirty = true;
    }
}

#[derive(Resource, Default)]
struct BookmarkAutoSave {
    dirty: bool,
}

fn migrate_legacy_bookmark_order(
    legacy: Query<(Entity, &Order), (BookmarkFilter, Without<BookmarkOrder>)>,
    mut commands: Commands,
) {
    for (entity, order) in &legacy {
        commands
            .entity(entity)
            .insert(BookmarkOrder(order.0))
            .remove::<Order>()
            .remove::<Save>();
    }
}

fn mark_bookmarks_dirty(
    mut auto: ResMut<BookmarkAutoSave>,
    changed: Query<
        (),
        (
            BookmarkFilter,
            Or<(
                Added<Pin>,
                Added<Bookmark>,
                Added<Folder>,
                Added<Collapsed>,
                Changed<Name>,
                Changed<BookmarkOrder>,
                Changed<PageMetadata>,
                Changed<ChildOf>,
            )>,
        ),
    >,
    bookmark_items: Query<(), Or<(With<Bookmark>, With<Pin>, With<Folder>)>>,
    mut removed_pin: RemovedComponents<Pin>,
    mut removed_bookmark: RemovedComponents<Bookmark>,
    mut removed_folder: RemovedComponents<Folder>,
    mut removed_collapsed: RemovedComponents<Collapsed>,
    mut removed_child_of: RemovedComponents<ChildOf>,
) {
    let removed_child_of_bookmark = removed_child_of
        .read()
        .any(|entity| bookmark_items.get(entity).is_ok());
    let any_removed = removed_pin.read().next().is_some()
        | removed_bookmark.read().next().is_some()
        | removed_folder.read().next().is_some()
        | removed_collapsed.read().next().is_some()
        | removed_child_of_bookmark;
    if any_removed || !changed.is_empty() {
        auto.dirty = true;
    }
}

fn autosave_bookmarks(mut auto: ResMut<BookmarkAutoSave>, mut commands: Commands) {
    if !auto.dirty {
        return;
    }
    save_bookmarks_to_path(&mut commands, bookmarks_path());
    auto.dirty = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_round_trips_bookmarks_and_excludes_save_entities() {
        let dir = std::env::temp_dir().join(format!("vmux-bm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bookmarks.ron");

        let mut save_app = App::new();
        save_app
            .add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(vmux_core::CorePlugin)
            .register_type::<OfferedBookmarkDefaults>()
            .insert_resource(OfferedBookmarkDefaults {
                urls: vec!["vmux://start/".into()],
            })
            .add_observer(save_on::<SaveWorld<BookmarkFilter>>);
        save_app.world_mut().spawn((
            Folder,
            Uuid("f1".into()),
            Name::new("PRs"),
            BookmarkOrder(0),
        ));
        save_app.world_mut().spawn((
            Bookmark,
            Uuid("b1".into()),
            PageMetadata {
                title: "A".into(),
                url: "https://a.test".into(),
                icon: vmux_core::icon::PageIcon::default(),
                bg_color: None,
            },
            BookmarkOrder(1),
        ));
        save_app
            .world_mut()
            .spawn((Save, Name::new("excluded-save-entity")));
        let p = path.clone();
        save_app.add_systems(Update, move |mut c: Commands| {
            let mut s = SaveWorld::<BookmarkFilter>::into_file(p.clone());
            s.components = bookmark_scene_filter();
            s.resources = bookmark_resource_filter();
            c.trigger_save(s);
        });
        save_app.update();
        save_app.update();

        assert!(path.exists(), "bookmarks.ron written");
        let ron = std::fs::read_to_string(&path).unwrap();
        assert!(ron.contains("b1"), "bookmark uuid persisted");
        assert!(ron.contains("PRs"), "folder name persisted");

        let mut load_app = App::new();
        load_app
            .add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(vmux_core::CorePlugin)
            .register_type::<OfferedBookmarkDefaults>()
            .init_resource::<OfferedBookmarkDefaults>()
            .add_observer(load_on::<LoadWorld<BookmarkFilter>>);
        let p2 = path.clone();
        load_app.add_systems(Update, move |mut c: Commands| {
            c.trigger_load(LoadWorld::<BookmarkFilter>::from_file(p2.clone()));
        });
        load_app.update();
        load_app.update();

        let bookmarks = load_app
            .world_mut()
            .query_filtered::<Entity, With<Bookmark>>()
            .iter(load_app.world())
            .count();
        let folders = load_app
            .world_mut()
            .query_filtered::<Entity, With<Folder>>()
            .iter(load_app.world())
            .count();
        assert_eq!(bookmarks, 1, "bookmark rebuilt");
        assert_eq!(folders, 1, "folder rebuilt");
        let excluded = load_app
            .world_mut()
            .query::<&Name>()
            .iter(load_app.world())
            .any(|name| name.as_str() == "excluded-save-entity");
        assert!(!excluded, "Save-only entity excluded");
        assert_eq!(
            load_app.world().resource::<OfferedBookmarkDefaults>().urls,
            ["vmux://start/"]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn removed_default_bookmarks_are_not_offered_again() {
        let defaults = vec!["vmux://start/".into(), "vmux://terminal/".into()];
        let mut offered = OfferedBookmarkDefaults::default();

        assert_eq!(offered.claim_new(&defaults), defaults);
        assert!(offered.claim_new(&defaults).is_empty());
    }

    #[test]
    fn existing_bookmark_store_receives_missing_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("bookmarks.ron");
        let mut save_app = App::new();
        save_app
            .add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(vmux_core::CorePlugin)
            .add_observer(save_on::<SaveWorld<BookmarkFilter>>);
        save_app.world_mut().spawn((
            Pin,
            Uuid("existing".into()),
            PageMetadata {
                title: "Existing".into(),
                url: "https://example.com".into(),
                icon: vmux_core::PageIcon::None,
                bg_color: None,
            },
            BookmarkOrder(4),
        ));
        let save_path = path.clone();
        save_app.add_systems(Update, move |mut commands: Commands| {
            let mut save = SaveWorld::<BookmarkFilter>::into_file(save_path.clone());
            save.components = bookmark_scene_filter();
            commands.trigger_save(save);
        });
        save_app.update();
        save_app.update();

        let mut settings = vmux_setting::AppSettings::embedded();
        settings.browser.bookmarks = vec!["vmux://start/".into(), "vmux://terminal/".into()];
        let mut load_app = App::new();
        load_app
            .add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(vmux_core::CorePlugin)
            .insert_resource(settings)
            .register_type::<OfferedBookmarkDefaults>()
            .init_resource::<OfferedBookmarkDefaults>()
            .init_resource::<BookmarkAutoSave>()
            .add_observer(load_on::<LoadWorld<BookmarkFilter>>)
            .add_observer(seed_default_bookmarks_after_load);
        load_app.world_mut().insert_resource(BookmarkLoadPending);
        load_app
            .world_mut()
            .commands()
            .trigger_load(LoadWorld::<BookmarkFilter>::from_file(path));
        load_app.update();
        load_app.update();

        let mut pins = load_app
            .world_mut()
            .query_filtered::<(&PageMetadata, &BookmarkOrder), With<Pin>>()
            .iter(load_app.world())
            .map(|(metadata, order)| (metadata.url.clone(), order.0))
            .collect::<Vec<_>>();
        pins.sort();
        assert_eq!(
            pins,
            [
                ("https://example.com".into(), 4),
                ("vmux://start/".into(), 5),
                ("vmux://terminal/".into(), 6),
            ]
        );
        assert_eq!(
            load_app.world().resource::<OfferedBookmarkDefaults>().urls,
            ["vmux://start/", "vmux://terminal/"]
        );
    }

    #[test]
    fn legacy_bookmark_order_migration_removes_space_save_marker() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(vmux_core::CorePlugin)
            .add_systems(Update, migrate_legacy_bookmark_order);
        let entity = app
            .world_mut()
            .spawn((Bookmark, Uuid("b1".into()), Order(3)))
            .id();
        assert!(app.world().get::<Save>(entity).is_some());

        app.update();

        assert_eq!(
            app.world().get::<BookmarkOrder>(entity),
            Some(&BookmarkOrder(3))
        );
        assert!(app.world().get::<Order>(entity).is_none());
        assert!(app.world().get::<Save>(entity).is_none());
    }
}
