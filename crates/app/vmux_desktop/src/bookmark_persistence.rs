use bevy::prelude::*;
use bevy_world_serialization::WorldFilter;
use moonshine_save::prelude::*;
use std::collections::{HashMap, HashSet};
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
    pins: Query<(Entity, &PageMetadata), With<Pin>>,
    bookmarks: Query<(Entity, &PageMetadata), With<Bookmark>>,
    folders: Query<(Entity, &Name), With<Folder>>,
    manifests: Query<&vmux_core::page::PageManifest>,
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
        BookmarkDefaults::of(
            &settings.browser.bookmarks,
            &settings.browser.bookmark_folders,
        )
        .seed(
            &pins,
            &bookmarks,
            &folders,
            &manifests,
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
    pins: Query<(Entity, &PageMetadata), With<Pin>>,
    bookmarks: Query<(Entity, &PageMetadata), With<Bookmark>>,
    folders: Query<(Entity, &Name), With<Folder>>,
    manifests: Query<&vmux_core::page::PageManifest>,
    orders: Query<&BookmarkOrder, BookmarkFilter>,
    mut offered: ResMut<OfferedBookmarkDefaults>,
    mut auto: ResMut<BookmarkAutoSave>,
    mut commands: Commands,
) {
    if pending.is_none() {
        return;
    }
    BookmarkDefaults::of(
        &settings.browser.bookmarks,
        &settings.browser.bookmark_folders,
    )
    .seed(
        &pins,
        &bookmarks,
        &folders,
        &manifests,
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
    #[reflect(default)]
    folders: Vec<String>,
    #[reflect(default)]
    folder_bookmarks: Vec<String>,
}

impl OfferedBookmarkDefaults {
    fn claim_new_urls(&mut self, defaults: &[String]) -> Vec<String> {
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

    fn claim_new_folders(
        &mut self,
        defaults: &[vmux_setting::BookmarkFolderSettings],
    ) -> Vec<String> {
        let mut offered = self
            .folders
            .iter()
            .map(|name| BookmarkDefaults::key(name))
            .collect::<HashSet<_>>();
        let mut claimed = Vec::new();
        for folder in defaults {
            let key = BookmarkDefaults::key(&folder.name);
            if key.is_empty() || !offered.insert(key) {
                continue;
            }
            self.folders.push(folder.name.clone());
            claimed.push(folder.name.clone());
        }
        claimed
    }

    fn claim_new_folder_bookmarks(
        &mut self,
        defaults: &[vmux_setting::BookmarkFolderSettings],
    ) -> Vec<(String, String)> {
        let mut offered = self
            .folder_bookmarks
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        let mut claimed = Vec::new();
        for folder in defaults {
            for url in &folder.bookmarks {
                let key = BookmarkDefaults::folder_bookmark_key(&folder.name, url);
                if key.is_empty() || !offered.insert(key.clone()) {
                    continue;
                }
                self.folder_bookmarks.push(key);
                claimed.push((folder.name.clone(), url.clone()));
            }
        }
        claimed
    }
}

struct BookmarkDefaults<'a> {
    urls: &'a [String],
    folders: &'a [vmux_setting::BookmarkFolderSettings],
}

impl<'a> BookmarkDefaults<'a> {
    fn of(urls: &'a [String], folders: &'a [vmux_setting::BookmarkFolderSettings]) -> Self {
        Self { urls, folders }
    }

    fn key(url: &str) -> String {
        url.trim().trim_end_matches('/').to_ascii_lowercase()
    }

    fn folder_bookmark_key(folder: &str, url: &str) -> String {
        let folder = Self::key(folder);
        let url = Self::key(url);
        if folder.is_empty() || url.is_empty() {
            return String::new();
        }
        format!("{folder}\n{url}")
    }

    fn metadata(url: String, titles: &HashMap<String, &'static str>) -> PageMetadata {
        let title = titles
            .get(&Self::key(&url))
            .map_or_else(|| url.clone(), |title| (*title).to_string());
        PageMetadata {
            title,
            url,
            icon: vmux_core::PageIcon::None,
            bg_color: None,
        }
    }

    fn seed(
        self,
        pins: &Query<(Entity, &PageMetadata), With<Pin>>,
        bookmarks: &Query<(Entity, &PageMetadata), With<Bookmark>>,
        folders: &Query<(Entity, &Name), With<Folder>>,
        manifests: &Query<&vmux_core::page::PageManifest>,
        orders: &Query<&BookmarkOrder, BookmarkFilter>,
        offered: &mut OfferedBookmarkDefaults,
        auto: &mut BookmarkAutoSave,
        commands: &mut Commands,
    ) {
        let claimed_urls = offered.claim_new_urls(self.urls);
        let claimed_folders = offered.claim_new_folders(self.folders);
        let claimed_folder_bookmarks = offered.claim_new_folder_bookmarks(self.folders);
        if claimed_urls.is_empty()
            && claimed_folders.is_empty()
            && claimed_folder_bookmarks.is_empty()
        {
            return;
        }
        let titles = manifests
            .iter()
            .map(|manifest| (Self::key(&manifest.url()), manifest.title))
            .collect::<HashMap<_, _>>();
        let mut pinned = pins
            .iter()
            .map(|(entity, metadata)| {
                (
                    Self::key(&metadata.url),
                    (
                        entity,
                        metadata.title.is_empty() || metadata.title == metadata.url,
                    ),
                )
            })
            .collect::<HashMap<_, _>>();
        let mut bookmarked = bookmarks
            .iter()
            .map(|(_, metadata)| Self::key(&metadata.url))
            .collect::<HashSet<_>>();
        let mut next_order = orders
            .iter()
            .map(|order| order.0)
            .max()
            .map_or(0, |order| order.saturating_add(1));
        for url in claimed_urls {
            let key = Self::key(&url);
            if pinned.contains_key(&key) {
                continue;
            }
            let metadata = Self::metadata(url, &titles);
            let entity = commands
                .spawn((
                    Pin,
                    Uuid(uuid::Uuid::new_v4().to_string()),
                    metadata,
                    BookmarkOrder(next_order),
                ))
                .id();
            pinned.insert(key, (entity, false));
            next_order = next_order.saturating_add(1);
        }
        let mut existing_folders = folders
            .iter()
            .map(|(entity, name)| (Self::key(name.as_str()), entity))
            .collect::<HashMap<_, _>>();
        for name in claimed_folders {
            let key = Self::key(&name);
            if existing_folders.contains_key(&key) {
                continue;
            }
            let entity = commands
                .spawn((
                    Folder,
                    Uuid(uuid::Uuid::new_v4().to_string()),
                    Name::new(name),
                    BookmarkOrder(next_order),
                ))
                .id();
            existing_folders.insert(key, entity);
            next_order = next_order.saturating_add(1);
        }
        for (folder_name, url) in claimed_folder_bookmarks {
            let Some(folder) = existing_folders.get(&Self::key(&folder_name)) else {
                continue;
            };
            let key = Self::key(&url);
            if !bookmarked.insert(key.clone()) {
                continue;
            }
            let metadata = Self::metadata(url, &titles);
            if let Some((entity, fallback_title)) = pinned.get(&key) {
                let mut entity = commands.entity(*entity);
                entity.insert((Bookmark, ChildOf(*folder)));
                if *fallback_title {
                    entity.insert(metadata);
                }
            } else {
                commands.spawn((
                    Bookmark,
                    Uuid(uuid::Uuid::new_v4().to_string()),
                    metadata,
                    BookmarkOrder(next_order),
                    ChildOf(*folder),
                ));
                next_order = next_order.saturating_add(1);
            }
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
                ..default()
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
        assert!(
            load_app
                .world()
                .resource::<OfferedBookmarkDefaults>()
                .folders
                .is_empty()
        );
        assert!(
            load_app
                .world()
                .resource::<OfferedBookmarkDefaults>()
                .folder_bookmarks
                .is_empty()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn removed_default_bookmarks_are_not_offered_again() {
        let defaults = vec!["vmux://start/".into(), "vmux://terminal/".into()];
        let mut offered = OfferedBookmarkDefaults::default();

        assert_eq!(offered.claim_new_urls(&defaults), defaults);
        assert!(offered.claim_new_urls(&defaults).is_empty());
    }

    #[test]
    fn removed_default_folders_are_not_offered_again() {
        let defaults = vec![
            vmux_setting::BookmarkFolderSettings {
                name: "Projects".into(),
                bookmarks: Vec::new(),
            },
            vmux_setting::BookmarkFolderSettings {
                name: "Knowledge".into(),
                bookmarks: Vec::new(),
            },
        ];
        let mut offered = OfferedBookmarkDefaults::default();

        assert_eq!(
            offered.claim_new_folders(&defaults),
            ["Projects", "Knowledge"]
        );
        assert!(offered.claim_new_folders(&defaults).is_empty());
    }

    #[test]
    fn removed_default_folder_bookmarks_are_not_offered_again() {
        let defaults = vec![vmux_setting::BookmarkFolderSettings {
            name: "Projects".into(),
            bookmarks: vec!["vmux://projects/".into(), "https://example.com".into()],
        }];
        let mut offered = OfferedBookmarkDefaults::default();

        assert_eq!(
            offered.claim_new_folder_bookmarks(&defaults),
            [
                ("Projects".into(), "vmux://projects/".into()),
                ("Projects".into(), "https://example.com".into()),
            ]
        );
        assert!(offered.claim_new_folder_bookmarks(&defaults).is_empty());
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
        settings.browser.bookmark_folders = vec![
            vmux_setting::BookmarkFolderSettings {
                name: "Projects".into(),
                bookmarks: vec!["vmux://start/".into()],
            },
            vmux_setting::BookmarkFolderSettings {
                name: "Knowledge".into(),
                bookmarks: vec!["https://docs.example".into()],
            },
        ];
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
        let mut folders = load_app
            .world_mut()
            .query_filtered::<&Name, With<Folder>>()
            .iter(load_app.world())
            .map(|name| name.as_str().to_string())
            .collect::<Vec<_>>();
        folders.sort();
        assert_eq!(folders, ["Knowledge", "Projects"]);
        assert_eq!(
            load_app
                .world()
                .resource::<OfferedBookmarkDefaults>()
                .folders,
            ["Projects", "Knowledge"]
        );
        let mut assignments = load_app
            .world_mut()
            .query::<(&PageMetadata, &ChildOf)>()
            .iter(load_app.world())
            .map(|(metadata, parent)| {
                let folder = load_app
                    .world()
                    .get::<Name>(bevy::ecs::relationship::Relationship::get(parent))
                    .expect("bookmark folder");
                (metadata.url.clone(), folder.as_str().to_string())
            })
            .collect::<Vec<_>>();
        assignments.sort();
        assert_eq!(
            assignments,
            [
                ("https://docs.example".into(), "Knowledge".into()),
                ("vmux://start/".into(), "Projects".into()),
            ]
        );
        assert_eq!(
            load_app
                .world()
                .resource::<OfferedBookmarkDefaults>()
                .folder_bookmarks,
            ["projects\nvmux://start", "knowledge\nhttps://docs.example",]
        );
    }

    #[test]
    fn existing_empty_default_folder_receives_new_bookmarks() {
        let mut settings = vmux_setting::AppSettings::embedded();
        settings.browser.bookmarks.clear();
        settings.browser.bookmark_folders = vec![vmux_setting::BookmarkFolderSettings {
            name: "Projects".into(),
            bookmarks: vec!["vmux://projects/".into()],
        }];
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(vmux_core::CorePlugin)
            .insert_resource(settings)
            .insert_resource(OfferedBookmarkDefaults {
                urls: vec!["vmux://projects/".into()],
                folders: vec!["Projects".into()],
                ..default()
            })
            .init_resource::<BookmarkAutoSave>()
            .add_observer(seed_default_bookmarks_after_load);
        app.world_mut().insert_resource(BookmarkLoadPending);
        app.world_mut().spawn(vmux_core::page::PageManifest {
            host: "projects",
            title: "Projects",
            title_message_id: None,
            replaces_command: None,
            keywords: &[],
            icon: None,
            command_bar: true,
        });
        let pin = app
            .world_mut()
            .spawn((
                Pin,
                Uuid("project-pin".into()),
                PageMetadata {
                    title: "vmux://projects/".into(),
                    url: "vmux://projects/".into(),
                    ..default()
                },
                BookmarkOrder(0),
            ))
            .id();
        let folder = app
            .world_mut()
            .spawn((
                Folder,
                Uuid("projects-folder".into()),
                Name::new("Projects"),
                BookmarkOrder(1),
            ))
            .id();

        app.world_mut().trigger(Loaded {
            entity_map: bevy::ecs::entity::EntityHashMap::default(),
        });
        app.update();

        assert!(app.world().entity(pin).contains::<Bookmark>());
        assert_eq!(
            app.world()
                .get::<PageMetadata>(pin)
                .map(|meta| meta.title.as_str()),
            Some("Projects")
        );
        assert_eq!(
            app.world()
                .get::<ChildOf>(pin)
                .map(bevy::ecs::relationship::Relationship::get),
            Some(folder)
        );
        assert_eq!(
            app.world()
                .resource::<OfferedBookmarkDefaults>()
                .folder_bookmarks,
            ["projects\nvmux://projects"]
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
