#![allow(non_snake_case)]

use std::collections::HashMap;
use std::rc::Rc;

use crate::event::{
    BOOKMARK_MENU_ACTION_EVENT, BOOKMARKS_EVENT, BookmarkContextMenuEvent, BookmarkMenuActionEvent,
    BookmarkNode, BookmarkRow, BookmarkTextInputEvent, BookmarksCommandEvent, BookmarksHostEvent,
    FolderRow, HeaderCommandEvent, LAYOUT_STATE_EVENT, LayoutStateEvent, PANE_TREE_EVENT, PaneNode,
    PaneTreeEvent, RELOAD_EVENT, REMOTE_STATE_EVENT, ReloadEvent, RemoteCommandEvent,
    RemoteCopyEvent, RemotePhase, RemoteStateEvent, STACKS_EVENT, StackNode, StackRow,
    StacksHostEvent, TABS_EVENT, TabRow, TabsCommandEvent, TabsHostEvent, WindowDragRegionEvent,
};
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use vmux_command::panel::CommandBarPanel;
use vmux_core::event::team::{TEAM_EVENT, TeamCommandEvent, TeamEvent, TeamMemberRow};
use vmux_core::event::{
    EXTENSIONS_LIST_EVENT, ExtActionRequest, ExtListRequest, ExtOpenManagerRequest, ExtRow,
    ExtensionsEvent,
};
use vmux_core::knowledge::{
    KNOWLEDGE_TREE_EVENT, KnowledgeEntry, KnowledgeGitStatus, KnowledgeTreeEvent,
};
use vmux_core::tools::{TOOLS_SNAPSHOT_EVENT, ToolCategory, ToolItem, ToolStatus, ToolsSnapshot};
use vmux_core::{PageIcon, PageMetadata};
use vmux_ui::components::avatar::Avatar;
use vmux_ui::components::context_menu::{
    ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger,
};
use vmux_ui::components::icon::Icon;
use vmux_ui::components::progress::{Progress, ProgressIndicator};
use vmux_ui::components::tree_row::{
    SIDEBAR_CARD_CHEVRON_CLOSED, SIDEBAR_CARD_CHEVRON_OPEN, SIDEBAR_TREE_CHEVRON_CLOSED,
    SIDEBAR_TREE_CHEVRON_OPEN, SIDEBAR_TREE_COLUMN, SIDEBAR_TREE_SCROLLER, SidebarTreeChildren,
    SidebarTreeRow, SidebarTreeRowGroup,
};
use vmux_ui::favicon::favicon_src_for_url;
use vmux_ui::hooks::{send, use_event, use_listener, use_theme};
use vmux_ui::i18n::{TranslationValue, translate, translate_with};
use vmux_ui::icon::PageIconView;
use vmux_ui::platform::sleep_ms;
use vmux_ui::scroll::ScrollIntoView;
use vmux_ui::util::cn;

#[component]
pub fn Page() -> Element {
    use_theme();

    let mut layout_state = use_signal(LayoutStateEvent::default);
    let mut layout_state_received = use_signal(|| false);
    let layout_listener = use_listener::<LayoutStateEvent, _>(LAYOUT_STATE_EVENT, move |data| {
        layout_state_received.set(true);
        layout_state.set(data);
    });

    let mut stacks_state = use_signal(StacksHostEvent::default);
    let mut stacks_state_received = use_signal(|| false);
    let stacks_listener = use_listener::<StacksHostEvent, _>(STACKS_EVENT, move |data| {
        stacks_state_received.set(true);
        stacks_state.set(data);
    });

    let mut tabs_state = use_signal(TabsHostEvent::default);
    let mut tabs_state_received = use_signal(|| false);
    let tabs_listener = use_listener::<TabsHostEvent, _>(TABS_EVENT, move |data| {
        tabs_state_received.set(true);
        tabs_state.set(data);
    });

    let mut bookmarks_state = use_signal(BookmarksHostEvent::default);
    let _bookmarks_listener = use_listener::<BookmarksHostEvent, _>(BOOKMARKS_EVENT, move |data| {
        bookmarks_state.set(data);
    });
    let bookmark_menu_action = use_event::<BookmarkMenuActionEvent>(
        BOOKMARK_MENU_ACTION_EVENT,
        BookmarkMenuActionEvent::default,
    );
    use_context_provider(|| bookmark_menu_action);

    let mut reload_key = use_signal(|| 0u32);
    let _reload_listener = use_listener::<ReloadEvent, _>(RELOAD_EVENT, move |_| {
        reload_key.set(reload_key() + 1);
    });

    let mut pane_tree_state = use_signal(PaneTreeEvent::default);
    let mut pane_tree_state_received = use_signal(|| false);
    let pane_tree_listener = use_listener::<PaneTreeEvent, _>(PANE_TREE_EVENT, move |data| {
        pane_tree_state_received.set(true);
        pane_tree_state.set(data);
    });

    let mut spaces_state = use_signal(vmux_core::event::space::SpacesListEvent::default);
    let mut spaces_state_received = use_signal(|| false);
    let spaces_listener = use_listener::<vmux_core::event::space::SpacesListEvent, _>(
        vmux_core::event::space::SPACES_LIST_EVENT,
        move |data| {
            spaces_state_received.set(true);
            spaces_state.set(data);
        },
    );

    let projects_state = use_event::<crate::event::TabBoundaryEvent>(
        crate::event::TAB_BOUNDARY_EVENT,
        crate::event::TabBoundaryEvent::default,
    );

    let mut knowledge_state = use_signal(KnowledgeTreeEvent::default);
    let mut knowledge_state_received = use_signal(|| false);
    let _knowledge_listener =
        use_listener::<KnowledgeTreeEvent, _>(KNOWLEDGE_TREE_EVENT, move |data| {
            knowledge_state_received.set(true);
            knowledge_state.set(data);
        });

    let mut tools_state = use_signal(ToolsSnapshot::default);
    let mut tools_state_received = use_signal(|| false);
    let _tools_listener = use_listener::<ToolsSnapshot, _>(TOOLS_SNAPSHOT_EVENT, move |data| {
        tools_state_received.set(true);
        tools_state.set(data);
    });

    let team_state = use_event::<TeamEvent>(TEAM_EVENT, TeamEvent::default);
    let remote_state = use_event::<RemoteStateEvent>(REMOTE_STATE_EVENT, RemoteStateEvent::default);

    let extensions_state =
        use_event::<ExtensionsEvent>(EXTENSIONS_LIST_EVENT, ExtensionsEvent::default);
    use_effect(move || {
        let _ = send(&ExtListRequest);
    });

    let mut update_phase = use_signal(|| None::<UpdatePhase>);
    let _update_progress_listener = use_listener::<crate::event::UpdateProgressEvent, _>(
        crate::event::UPDATE_PROGRESS_EVENT,
        move |evt| {
            update_phase.set(Some(if evt.installing {
                UpdatePhase::Installing {
                    version: evt.version,
                }
            } else {
                UpdatePhase::Downloading {
                    version: evt.version,
                    downloaded: evt.downloaded,
                    total: evt.total,
                }
            }));
        },
    );
    let _update_ready_listener = use_listener::<crate::event::UpdateReadyEvent, _>(
        crate::event::UPDATE_READY_EVENT,
        move |evt| {
            update_phase.set(Some(UpdatePhase::Ready {
                version: evt.version,
            }))
        },
    );
    let _update_cleared_listener = use_listener::<crate::event::UpdateClearedEvent, _>(
        crate::event::UPDATE_CLEARED_EVENT,
        move |_| update_phase.set(None),
    );

    let state = layout_state();
    let stacks = stacks_state();
    let tabs = tabs_state();
    let PaneTreeEvent { panes } = pane_tree_state();
    let active_space = spaces_state().spaces.into_iter().find(|s| s.is_active);
    let layout_error = (layout_listener.error)();
    let stacks_error = (stacks_listener.error)();
    let tabs_error = (tabs_listener.error)();
    let pane_tree_error = (pane_tree_listener.error)();
    let spaces_error = (spaces_listener.error)();
    let overlay_ready = layout_overlay_ready(
        &state,
        listener_ready(layout_state_received(), &layout_error),
        listener_ready(stacks_state_received(), &stacks_error),
        listener_ready(tabs_state_received(), &tabs_error),
        listener_ready(pane_tree_state_received(), &pane_tree_error),
        listener_ready(spaces_state_received(), &spaces_error),
    );
    let radius_px = state.radius;
    let reveal = StackReveal::side_sheet(use_signal(|| None::<(u64, u64)>));
    use_effect(move || set_root_radius_px(radius_px));
    use_effect(move || {
        if !layout_state().side_sheet_open {
            reveal.forget();
            return;
        }
        let PaneTreeEvent { panes } = pane_tree_state();
        let Some(target) = ActiveStack::of(&panes) else {
            return;
        };
        reveal.follow(target);
    });
    let host_sheet_width = state.side_sheet_width;
    let sheet_left = state.window_pad_left;
    let mut sheet_width = use_signal(|| host_sheet_width);
    let mut sheet_resizing = use_signal(|| false);
    use_effect(use_reactive!(|host_sheet_width| {
        if !*sheet_resizing.peek() {
            sheet_width.set(host_sheet_width);
        }
    }));
    let side_sheet_vars = format!(
        "--vmux-side-sheet-width:{}px;--vmux-side-sheet-left:{}px;--vmux-side-sheet-top:{}px;--vmux-side-sheet-bottom:{}px;--vmux-side-sheet-pad-top:{}px;",
        sheet_width(),
        state.window_pad_left,
        state.window_pad_top,
        state.window_pad_bottom,
        crate::event::url_bar_top(),
    );
    let header_vars = format!(
        "--vmux-header-top:{}px;--vmux-header-left:{}px;--vmux-header-right:{}px;--vmux-header-height:{}px;--vmux-tab-row-pad-left:{}px;",
        state.header_top(),
        state.header_left(),
        state.header_right(),
        state.header_height,
        state.tab_row_pad_left(),
    );

    rsx! {
        div { class: "fixed inset-0 pointer-events-none text-foreground",
            if overlay_ready && state.side_sheet_open {
                aside {
                    id: "vmux-side-sheet",
                    class: "pointer-events-auto fixed left-[var(--vmux-side-sheet-left)] top-[var(--vmux-side-sheet-top)] bottom-[var(--vmux-side-sheet-bottom)] min-h-0 overflow-visible w-[var(--vmux-side-sheet-width)] pt-[var(--vmux-side-sheet-pad-top)]",
                    style: "{side_sheet_vars}",
                    SideSheetGrab { resizing: sheet_resizing }
                    div { class: "flex h-full min-h-0 flex-col",
                        SideSheetView {
                            panes,
                            active_space,
                            remote: remote_state(),
                            bookmarks: bookmarks_state(),
                            projects: projects_state().projects,
                            knowledge: knowledge_state(),
                            knowledge_loaded: knowledge_state_received(),
                            tools: tools_state(),
                            tools_loaded: tools_state_received(),
                            pane_tree_error: pane_tree_error.clone(),
                        }
                        if let Some(phase) = update_phase() {
                            UpdateNoticeFooter { phase }
                        }
                    }
                }
            }
            if overlay_ready && state.header_visible() {
                div {
                    class: "pointer-events-auto fixed top-[var(--vmux-header-top)] left-[var(--vmux-header-left)] right-[var(--vmux-header-right)] h-[var(--vmux-header-height)]",
                    style: "{header_vars}",
                    HeaderView {
                        stacks_state: stacks,
                        tabs_state: tabs,
                        bookmarks: bookmarks_state(),
                        team: team_state().members,
                        extensions: extensions_state().extensions,
                        reload_key: reload_key(),
                        stacks_error: stacks_error.clone(),
                        tabs_error: tabs_error.clone(),
                    }
                }
            }
            CommandBarPanel {}
            if sheet_resizing() {
                div {
                    class: "pointer-events-auto fixed inset-0 z-[900] cursor-col-resize",
                    onmousemove: move |event: Event<MouseData>| {
                        let x = event.client_coordinates().x as f32 - sheet_left;
                        sheet_width.set(crate::event::SideSheetResizeEvent { width: x }.clamped());
                    },
                    onmouseup: move |_| {
                        sheet_resizing.set(false);
                        let _ = send(
                            &crate::event::SideSheetResizeEvent {
                                width: sheet_width(),
                            },
                        );
                    },
                }
            }
        }
    }
}

#[component]
fn SideSheetGrab(mut resizing: Signal<bool>) -> Element {
    rsx! {
        div {
            class: "absolute inset-y-0 -right-1 z-10 w-2 cursor-col-resize",
            onmousedown: move |event: Event<MouseData>| {
                event.prevent_default();
                resizing.set(true);
            },
            div { class: "mx-auto h-full w-px bg-transparent transition-colors duration-150 hover:bg-cyan-400/40" }
        }
    }
}

struct ActiveStack;

impl ActiveStack {
    fn of(panes: &[PaneNode]) -> Option<(u64, u64)> {
        let mut fallback = None;
        for pane in panes {
            for stack in &pane.stacks {
                if !stack.is_active {
                    continue;
                }
                if pane.is_active {
                    return Some((pane.id, stack.id));
                }
                if fallback.is_none() {
                    fallback = Some((pane.id, stack.id));
                }
            }
        }
        fallback
    }
}

#[derive(Clone, Copy)]
struct StackReveal {
    settled: Signal<Option<(u64, u64)>>,
    prefix: &'static str,
}

impl StackReveal {
    fn side_sheet(settled: Signal<Option<(u64, u64)>>) -> Self {
        Self {
            settled,
            prefix: "sidesheet-stack",
        }
    }

    fn forget(mut self) {
        if (self.settled)().is_some() {
            self.settled.set(None);
        }
    }

    fn follow(mut self, target: (u64, u64)) {
        let Some(settled) = (self.settled)() else {
            self.settled.set(Some(target));
            return;
        };
        if settled == target {
            return;
        }
        let (pane_id, stack_id) = target;
        if ScrollIntoView::nearest(&format!("{}-{pane_id}-{stack_id}", self.prefix)) {
            self.settled.set(Some(target));
        }
    }
}

#[component]
fn SideSheetView(
    panes: Vec<PaneNode>,
    active_space: Option<vmux_core::event::space::SpaceRow>,
    remote: RemoteStateEvent,
    bookmarks: BookmarksHostEvent,
    projects: Vec<vmux_core::event::ProjectRow>,
    knowledge: KnowledgeTreeEvent,
    knowledge_loaded: bool,
    tools: ToolsSnapshot,
    tools_loaded: bool,
    pane_tree_error: Option<String>,
) -> Element {
    let active_pane = panes
        .iter()
        .find(|pane| pane.is_active)
        .or_else(|| panes.first())
        .cloned();
    let active_page = active_pane
        .as_ref()
        .and_then(|pane| pane.stacks.iter().find(|stack| stack.is_active))
        .filter(|stack| !stack.url.is_empty())
        .cloned();
    let folders = bookmark_folder_choices(&bookmarks.roots);
    let initial_folders = folders.clone();
    let mut folder_context = use_signal(|| initial_folders);
    let drag_state = use_signal(|| None::<BookmarkDragState>);
    use_context_provider(|| folder_context);
    use_context_provider(|| drag_state);
    use_effect(move || folder_context.set(folders.clone()));
    use_drop(move || {
        remove_bookmark_drag_ghost();
        set_bookmark_context_menu_active(false);
    });
    rsx! {
        div {
            class: "flex min-h-0 flex-1 flex-col overflow-x-hidden overflow-y-auto px-2 pb-3 pt-2 text-foreground [scrollbar-gutter:stable]",
            ..BookmarkDragState::listeners(drag_state),
            if let Some(space) = active_space {
                div { class: "glass mb-2 flex shrink-0 flex-col overflow-hidden rounded-lg",
                    SideSheetSpaceRow { key: "{space.id}", space: space.clone() }
                    RemotePanel { remote: remote.clone() }
                }
            }
            if let Some(pane) = active_pane {
                BookmarksSection {
                    bookmarks: bookmarks.clone(),
                    active_page,
                    pane_id: pane.id,
                    expanded: pane.bookmarks_expanded,
                    smart: SmartBookmarkContent {
                        pane_id: pane.id,
                        projects: projects.clone(),
                        knowledge: knowledge.clone(),
                        knowledge_loaded,
                        tools: tools.clone(),
                        tools_loaded,
                    },
                }
            }
            if let Some(err) = pane_tree_error {
                div { class: "flex shrink-0 items-center px-2 py-1",
                    span { class: "text-ui text-destructive", "{err}" }
                }
            } else if panes.is_empty() {
                div { class: "flex shrink-0 items-center px-2 py-1",
                    span { class: "text-ui text-muted-foreground", {translate("layout-no-stacks")} }
                }
            } else {
                for (i, pane) in panes.iter().enumerate() {
                    PaneSection { key: "{pane.id}", pane: pane.clone(), index: i }
                }
            }
        }
    }
}

fn listener_ready(received: bool, error: &Option<String>) -> bool {
    received || error.is_some()
}

fn layout_overlay_ready(
    state: &LayoutStateEvent,
    layout_ready: bool,
    stacks_ready: bool,
    tabs_ready: bool,
    pane_tree_ready: bool,
    spaces_ready: bool,
) -> bool {
    layout_ready
        && (!state.header_visible() || (stacks_ready && tabs_ready))
        && (!state.side_sheet_open || (pane_tree_ready && spaces_ready))
}

#[component]
fn UpdateNoticeFooter(phase: UpdatePhase) -> Element {
    let (label, version) = match &phase {
        UpdatePhase::Downloading { version, .. } => {
            (translate("layout-update-downloading"), version.clone())
        }
        UpdatePhase::Installing { version } => {
            (translate("layout-update-installing"), version.clone())
        }
        UpdatePhase::Ready { version } => (translate("layout-update-ready"), version.clone()),
    };
    rsx! {
        div {
            class: "shrink-0 mx-2 mb-2 mt-2 flex flex-col gap-2 rounded-md glass px-3 py-2 text-foreground",
            div { class: "flex items-center gap-2",
                span { class: "inline-block h-2 w-2 shrink-0 rounded-full bg-success" }
                span { class: "min-w-0 flex-1 text-ui font-medium", "{label}" }
                span { class: "shrink-0 text-xs text-muted-foreground", "{version}" }
            }
            {match phase {
                UpdatePhase::Downloading { downloaded, total, .. } => rsx! {
                    UpdateProgressBar { downloaded, total }
                },
                UpdatePhase::Installing { .. } => rsx! {
                    UpdateProgressBar { downloaded: 0, total: 0 }
                },
                UpdatePhase::Ready { .. } => rsx! {
                    button {
                        r#type: "button",
                        class: "w-full cursor-pointer rounded-md bg-primary px-2.5 py-1.5 text-ui font-medium text-primary-foreground hover:opacity-90",
                        onclick: move |_| {
                            let _ = send(&crate::event::RestartRequestEvent);
                        },
                        {translate("layout-restart-update")}
                    }
                },
            }}
        }
    }
}

#[component]
fn HeaderView(
    stacks_state: StacksHostEvent,
    tabs_state: TabsHostEvent,
    bookmarks: BookmarksHostEvent,
    team: Vec<TeamMemberRow>,
    extensions: Vec<ExtRow>,
    reload_key: u32,
    stacks_error: Option<String>,
    tabs_error: Option<String>,
) -> Element {
    let StacksHostEvent {
        stacks,
        can_go_back,
        can_go_forward,
        is_zoomed: _,
    } = stacks_state;
    let TabsHostEvent { tabs } = tabs_state;
    let active_row = stacks.iter().find(|t| t.is_active).cloned();
    let active_bg_color = active_row.as_ref().and_then(|r| r.bg_color.clone());
    let active_url = active_row
        .as_ref()
        .map(|r| r.url.clone())
        .unwrap_or_default();
    let show_bookmark = !active_url.is_empty();
    let is_bookmarked = show_bookmark
        && (bookmark_nodes_contain_url(&bookmarks.roots, &active_url)
            || bookmarks
                .pins
                .iter()
                .any(|pin| pin.metadata.url == active_url && pin.bookmarked));
    let pinned_uuid = bookmarks
        .pins
        .iter()
        .find(|pin| pin.metadata.url == active_url)
        .map(|pin| pin.uuid.clone());
    let is_pinned = pinned_uuid.is_some();
    let active_metadata = active_row.as_ref().map(|row| PageMetadata {
        title: row.title.clone(),
        url: row.url.clone(),
        icon: row.icon.clone(),
        bg_color: row.bg_color.clone(),
    });

    let (url_row_style, url_row_class) = url_row_cef(active_bg_color.as_deref());

    rsx! {
        div {
            class: "flex h-full min-h-0 min-w-0 flex-col text-foreground",
            div { class: "flex min-w-0 shrink-0 items-center gap-1 pl-[var(--vmux-tab-row-pad-left)] pr-2",
                if let Some(err) = tabs_error {
                    span { class: "text-ui text-destructive", "{err}" }
                } else {
                    div { class: "flex min-w-0 flex-1 items-center gap-1 overflow-x-auto overflow-y-hidden pl-2",
                        for tab in tabs.iter() {
                            {
                                let mut tab = tab.clone();
                                if tab.is_active {
                                    tab.bg_color = active_bg_color.clone();
                                }
                                rsx! { Tab { key: "{tab.id}", tab } }
                            }
                        }
                        NewTabButton {}
                        WindowDragRegion {}
                    }
                }
            }
            div {
                class: "{url_row_class}",
                style: "{url_row_style}",
                if let Some(err) = stacks_error {
                    span { class: "text-ui text-destructive", "{err}" }
                } else {
                    NavButton { label: translate("layout-back"), command: "prev_page", disabled: !can_go_back,
                        Icon { class: "h-4 w-4",
                            path { d: "M19 12H5" }
                            path { d: "M12 19l-7-7 7-7" }
                        }
                    }
                    NavButton { label: translate("layout-forward"), command: "next_page", disabled: !can_go_forward,
                        Icon { class: "h-4 w-4",
                            path { d: "M5 12h14" }
                            path { d: "M12 5l7 7-7 7" }
                        }
                    }
                    NavButton { label: translate("layout-reload"), command: "reload", disabled: active_row.as_ref().is_none_or(|t| t.url.is_empty()),
                        span {
                            key: "{reload_key}",
                            class: if reload_key > 0 { "inline-flex animate-spin-once" } else { "inline-flex" },
                            Icon { class: "h-4 w-4",
                                path { d: "M21 12a9 9 0 11-3-6.7L21 8" }
                                path { d: "M21 3v5h-5" }
                            }
                        }
                    }
                    HeaderAddressBar {
                        active_row: active_row.clone(),
                        bg_color: active_bg_color.clone(),
                    }
                    if show_bookmark {
                        button {
                            r#type: "button",
                            aria_label: if is_bookmarked { translate("layout-remove-bookmark") } else { translate("layout-bookmark-page") },
                            title: if is_bookmarked { format!("{} (\u{2318}D)", translate("layout-remove-bookmark")) } else { format!("{} (\u{2318}D)", translate("layout-bookmark-page")) },
                            class: if is_bookmarked {
                                "flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-md text-foreground transition-colors hover:bg-glass-hover"
                            } else {
                                "flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-glass-hover hover:text-foreground"
                            },
                            onclick: move |_| {
                                let _ = send(&BookmarksCommandEvent {
                                    command: "toggle_active".into(),
                                    uuid: None,
                                    name: None,
                                    url: None,
                                    metadata: None,
                                    folder: None,
                                });
                            },
                            Icon { class: "h-4 w-4",
                                path {
                                    d: "M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z",
                                    fill: if is_bookmarked { "currentColor" } else { "none" },
                                }
                            }
                        }
                        button {
                            r#type: "button",
                            aria_label: if is_pinned { translate("layout-unpin-page") } else { translate("layout-pin-page") },
                            title: if is_pinned { translate("layout-unpin-page") } else { translate("layout-pin-page") },
                            class: if is_pinned {
                                "flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-md text-foreground transition-colors hover:bg-glass-hover"
                            } else {
                                "flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-glass-hover hover:text-foreground"
                            },
                            onclick: move |_| {
                                if let Some(uuid) = pinned_uuid.clone() {
                                    bookmark_cmd("unpin", Some(uuid));
                                } else if let Some(metadata) = active_metadata.clone() {
                                    add_to_bookmarks("pin_url", metadata, None);
                                }
                            },
                            Icon { class: "h-4 w-4",
                                path { d: "M12 17v5" }
                                path { d: "M5 17h14" }
                                path { d: "M6 3h12" }
                                path {
                                    d: "M8 3v5a6 6 0 0 1-2 4v1h12v-1a6 6 0 0 1-2-4V3",
                                    fill: if is_pinned { "currentColor" } else { "none" },
                                }
                            }
                        }
                    }
                    TeamFacepile { members: team }
                    ExtensionBar { extensions }
                }
            }
        }
    }
}

fn url_row_cef(_bg_color: Option<&str>) -> (String, String) {
    (
        String::new(),
        "flex min-w-0 flex-1 shrink-0 items-center gap-1 rounded-t-[var(--radius)] px-2 bg-glass backdrop-blur-xl backdrop-saturate-150 text-foreground".to_string(),
    )
}

#[component]
fn SideSheetSpaceRow(space: vmux_core::event::space::SpaceRow) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "group flex w-full cursor-pointer items-center gap-2 px-2 py-1.5 text-foreground hover:bg-foreground/5",
            onclick: move |_| {
                let _ = send(&vmux_core::event::space::SpaceCommandEvent {
                    command: "open_page".to_string(),
                    space_id: Some(space.id.clone()),
                    name: None,
                });
            },
            Icon { class: "h-4 w-4 shrink-0",
                path { d: "M3 3h7v7H3z" }
                path { d: "M14 3h7v7h-7z" }
                path { d: "M3 14h7v7H3z" }
                path { d: "M14 14h7v7h-7z" }
            }
            span {
                class: "min-w-0 flex-1 truncate text-ui font-medium text-foreground text-left",
                "{space.name}"
            }
        }
    }
}

fn dir_truncate_class(title: &str) -> &'static str {
    if title.contains('/') {
        "truncate-start"
    } else {
        "truncate"
    }
}

#[component]
fn RemotePanel(remote: RemoteStateEvent) -> Element {
    let mut show_pairing = use_signal(|| false);
    let mut pairing_generation = use_signal(|| 0_u64);
    let mut pairing_started_paired = use_signal(|| false);
    let mut copied = use_signal(|| false);
    let active = remote.phase == RemotePhase::Enabled;
    let transitioning = remote.phase == RemotePhase::Starting;
    let status = match remote.phase {
        RemotePhase::Disabled | RemotePhase::Enabled => None,
        RemotePhase::Starting if remote.enabled => Some("Starting…"),
        RemotePhase::Starting => Some("Stopping…"),
        RemotePhase::Error => Some("Needs attention"),
    };
    let qr = if active
        && show_pairing()
        && (!remote.paired || pairing_started_paired())
        && !remote.pairing_deep_link.is_empty()
    {
        pairing_qr_svg(&remote.pairing_deep_link)
    } else {
        None
    };
    rsx! {
        div {
            class: if remote.enabled {
                "border-t border-success/30 bg-success/10 px-2.5 py-2.5"
            } else {
                "border-t border-foreground/10 px-2.5 py-2.5"
            },
            div { class: "flex items-center gap-2",
                div {
                    class: if remote.enabled {
                        "flex size-7 shrink-0 items-center justify-center rounded-md bg-success/15 text-success"
                    } else {
                        "flex size-7 shrink-0 items-center justify-center rounded-md bg-foreground/5 text-muted-foreground"
                    },
                    Icon { class: "size-4",
                        path { d: "M12 2a10 10 0 1 0 10 10" }
                        path { d: "M12 12 22 2" }
                        path { d: "M15 2h7v7" }
                    }
                }
                div { class: "min-w-0 flex-1",
                    div { class: "text-ui font-semibold", "Live" }
                    if let Some(status) = status {
                        div {
                            class: if remote.phase == RemotePhase::Error {
                                "mt-0.5 truncate text-[10px] text-destructive"
                            } else {
                                "mt-0.5 text-[10px] text-muted-foreground"
                            },
                            "{status}"
                        }
                    }
                }
                button {
                    r#type: "button",
                    class: if remote.enabled {
                        "relative h-5 w-9 shrink-0 rounded-full bg-success transition-colors"
                    } else {
                        "relative h-5 w-9 shrink-0 rounded-full bg-foreground/15 transition-colors"
                    },
                    aria_label: "Toggle Live",
                    aria_pressed: remote.enabled,
                    onclick: move |_| {
                        if remote.enabled {
                            pairing_generation.set(pairing_generation().wrapping_add(1));
                            show_pairing.set(false);
                        }
                        let _ = send(&RemoteCommandEvent {
                            enabled: !remote.enabled,
                        });
                    },
                    span {
                        class: if remote.enabled {
                            "absolute left-[18px] top-0.5 size-4 rounded-full bg-white shadow-sm transition-all"
                        } else {
                            "absolute left-0.5 top-0.5 size-4 rounded-full bg-white shadow-sm transition-all"
                        }
                    }
                }
            }
            if remote.phase == RemotePhase::Error {
                div { class: "mt-2 rounded-md border border-destructive/20 bg-destructive/5 p-2",
                    div { class: "break-words text-[10px] leading-4 text-destructive", "{remote.error}" }
                    button {
                        r#type: "button",
                        class: "mt-1.5 text-[10px] font-semibold text-foreground hover:opacity-70",
                        onclick: move |_| {
                            let _ = send(&RemoteCommandEvent {
                                enabled: remote.enabled,
                            });
                        },
                        "Retry"
                    }
                }
            } else if transitioning {
                div { class: "mt-2 h-1 overflow-hidden rounded-full bg-foreground/10",
                    div { class: "h-full w-full rounded-full bg-success" }
                }
            } else if active {
                if let Some(svg) = qr {
                    div { class: "mt-2 flex items-center justify-between gap-2",
                        div { class: "text-[10px] font-semibold text-foreground", "Connect a device" }
                        button {
                            r#type: "button",
                            class: "rounded px-1.5 py-1 text-[9px] font-semibold text-muted-foreground hover:bg-foreground/10 hover:text-foreground",
                            onclick: move |_| {
                                pairing_generation.set(pairing_generation().wrapping_add(1));
                                show_pairing.set(false);
                            },
                            "Close"
                        }
                    }
                    div { class: "mt-2 flex flex-col items-center rounded-lg bg-white p-2.5 text-zinc-950",
                        div {
                            class: "w-full rounded-sm [&>svg]:block [&>svg]:aspect-square [&>svg]:h-auto [&>svg]:w-full",
                            dangerous_inner_html: "{svg}",
                        }
                        div { class: "mt-1.5 text-center text-[10px] font-semibold", "Scan with your phone" }
                        div { class: "mt-0.5 text-center text-[9px] text-zinc-500", "Opens Vmux and pairs automatically" }
                    }
                    div { class: "mt-2 flex items-center gap-1.5 rounded-md bg-foreground/5 py-1 pl-2 pr-1",
                        div {
                            class: "min-w-0 flex-1 truncate font-mono text-[9px] text-muted-foreground",
                            title: "{remote.pairing_url}",
                            "{remote.pairing_url}"
                        }
                        button {
                            r#type: "button",
                            class: "shrink-0 rounded px-1.5 py-1 text-[9px] font-semibold text-foreground hover:bg-foreground/10",
                            onclick: move |_| {
                                let _ = send(&RemoteCopyEvent);
                                copied.set(true);
                            },
                            if copied() { "Copied" } else { "Copy" }
                        }
                    }
                    div { class: "mt-1.5 text-[9px] leading-4 text-muted-foreground",
                        "Pairing details hide automatically after 2 minutes."
                    }
                } else {
                    div { class: "mt-2 flex items-center gap-2",
                        div { class: if remote.paired { "flex min-w-0 flex-1 items-center gap-1.5 text-[10px] text-success" } else { "flex min-w-0 flex-1 items-center gap-1.5 text-[10px] text-muted-foreground" },
                            span { class: if remote.paired { "size-1.5 rounded-full bg-success" } else { "size-1.5 rounded-full bg-foreground/25" } }
                            if remote.paired { "Phone paired" } else { "No phone paired" }
                        }
                        button {
                            r#type: "button",
                            class: "text-[10px] font-semibold text-foreground hover:opacity-70",
                            onclick: move |_| {
                                copied.set(false);
                                pairing_started_paired.set(remote.paired);
                                let generation = pairing_generation().wrapping_add(1);
                                pairing_generation.set(generation);
                                show_pairing.set(true);
                                spawn(async move {
                                    sleep_ms(120_000).await;
                                    if pairing_generation() == generation {
                                        show_pairing.set(false);
                                    }
                                });
                            },
                            "Connect device"
                        }
                    }
                }
            }
        }
    }
}

fn pairing_qr_svg(value: &str) -> Option<String> {
    use qrcode::QrCode;
    use qrcode::render::svg;

    let code = QrCode::new(value).ok()?;
    Some(
        code.render::<svg::Color>()
            .min_dimensions(148, 148)
            .dark_color(svg::Color("#09090b"))
            .light_color(svg::Color("#ffffff"))
            .build(),
    )
}

#[derive(Clone, PartialEq)]
struct SmartBookmarkContent {
    pane_id: u64,
    projects: Vec<vmux_core::event::ProjectRow>,
    knowledge: KnowledgeTreeEvent,
    knowledge_loaded: bool,
    tools: ToolsSnapshot,
    tools_loaded: bool,
}

impl SmartBookmarkContent {
    fn count(&self, kind: vmux_core::SmartBookmarkFolder) -> usize {
        match kind {
            vmux_core::SmartBookmarkFolder::Projects => self.projects.len(),
            vmux_core::SmartBookmarkFolder::Knowledge => self.knowledge.entries.len(),
            vmux_core::SmartBookmarkFolder::Tools => self
                .tools
                .categories
                .iter()
                .map(|category| category.items.len())
                .sum(),
        }
    }
}

#[component]
fn BookmarksSection(
    bookmarks: BookmarksHostEvent,
    active_page: Option<StackNode>,
    pane_id: u64,
    expanded: bool,
    smart: SmartBookmarkContent,
) -> Element {
    let BookmarksHostEvent { pins, roots } = bookmarks;
    let drag_state: Signal<Option<BookmarkDragState>> = use_context();
    let mut creating_folder = use_signal(|| false);
    let new_folder_draft = use_signal(|| translate("layout-new-folder"));
    let bookmark_menu_action: Signal<BookmarkMenuActionEvent> = use_context();
    let initial_menu_action = bookmark_menu_action.peek().sequence;
    let mut handled_menu_action = use_signal(|| initial_menu_action);
    use_effect(move || {
        let action = bookmark_menu_action();
        if action.sequence == handled_menu_action() {
            return;
        }
        handled_menu_action.set(action.sequence);
        if action.action == "new_folder" && action.uuid.is_none() {
            begin_new_folder(creating_folder, new_folder_draft);
        }
    });
    let folders = bookmark_folder_choices(&roots);
    let folder_rows = bookmark_folder_rows(&roots);
    let root_targeted = bookmark_drop_targeted(drag_state, &BookmarkDropTarget::Root);
    let root_drop_label = drag_state()
        .filter(|drag| drag.active)
        .map(|drag| match drag.item {
            BookmarkDragItem::Page { .. } => translate("layout-add-to-bookmarks"),
            BookmarkDragItem::Bookmark { .. }
            | BookmarkDragItem::Pin { .. }
            | BookmarkDragItem::Folder { .. } => translate("layout-move-to-bookmarks"),
        });
    let bookmarks_title = translate("layout-bookmarks");
    let new_folder_title = translate("layout-new-folder");

    rsx! {
        div {
            "data-bookmark-drop": "root",
            class: "glass group relative z-30 mb-2 flex shrink-0 flex-col overflow-hidden rounded-lg",
            oncontextmenu: move |e: Event<MouseData>| {
                e.prevent_default();
                request_bookmark_menu("menu_root", None, None);
            },
            div {
                "data-bookmark-drop": "root",
                class: if root_targeted {
                    "flex items-center bg-foreground/10 ring-1 ring-inset ring-ring"
                } else {
                    "flex items-center transition-colors hover:bg-glass-hover"
                },
                div { class: "flex min-w-0 flex-1 items-center gap-2 px-2.5 py-2",
                    div { class: "grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-foreground/[0.07] text-foreground ring-1 ring-inset ring-foreground/10",
                        Icon { class: "h-3.5 w-3.5",
                            path { d: "M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z" }
                        }
                    }
                    if let Some(label) = root_drop_label {
                        span { class: "min-w-0 flex-1 text-ui font-semibold text-foreground", "{label}" }
                    } else {
                        span { class: "min-w-0 flex-1 text-ui font-semibold text-foreground", "{bookmarks_title}" }
                        button {
                            r#type: "button",
                            aria_label: "{new_folder_title}",
                            title: "{new_folder_title}",
                            class: "flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-sm text-muted-foreground hover:bg-foreground/10 hover:text-foreground",
                            onclick: move |event| {
                                event.prevent_default();
                                event.stop_propagation();
                                begin_new_folder(creating_folder, new_folder_draft);
                            },
                            Icon { class: "h-3.5 w-3.5 pointer-events-none",
                                path { d: "M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" }
                                path { d: "M12 10v6" }
                                path { d: "M9 13h6" }
                            }
                        }
                    }
                }
                button {
                    r#type: "button",
                    aria_label: "{bookmarks_title}",
                    title: "{bookmarks_title}",
                    class: if expanded {
                        "mr-2 flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-sm text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 hover:bg-foreground/10 hover:text-foreground"
                    } else {
                        "mr-2 flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-sm bg-foreground/10 text-foreground"
                    },
                    onclick: move |_| set_side_sheet_section(pane_id, "bookmarks", !expanded),
                    Icon {
                        class: if expanded { SIDEBAR_CARD_CHEVRON_OPEN } else { SIDEBAR_CARD_CHEVRON_CLOSED },
                        path { d: "m9 18 6-6-6-6" }
                    }
                }
            }
            div { class: if expanded {
                    "grid grid-rows-[1fr] opacity-100 transition-[grid-template-rows,opacity] duration-200 ease-out"
                } else {
                    "grid grid-rows-[0fr] opacity-0 transition-[grid-template-rows,opacity] duration-200 ease-out"
                },
                div { class: "overflow-hidden",
                    div { class: "border-t border-foreground/10 p-1.5",
                        if !pins.is_empty() {
                            div {
                                "data-bookmark-drop": "",
                                class: "mb-1 grid grid-cols-4 gap-1.5 p-1",
                                for p in pins.iter() {
                                    PinTile { key: "{p.uuid}", row: p.clone() }
                                }
                            }
                        }
                        if creating_folder() {
                            div { class: "flex h-9 items-center gap-2 rounded-md border border-transparent px-2",
                                Icon { class: "h-4 w-4 shrink-0 text-muted-foreground",
                                    path { d: "M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" }
                                }
                                BookmarkNameInput {
                                    draft: new_folder_draft,
                                    class: "min-w-0 flex-1 bg-transparent text-ui font-medium text-foreground outline-none".to_string(),
                                    placeholder: translate("layout-folder-name"),
                                    on_commit: move |name| {
                                        creating_folder.set(false);
                                        create_bookmark_folder(name, None);
                                    },
                                    on_cancel: move |_| creating_folder.set(false),
                                }
                            }
                        }
                        if pins.is_empty() && roots.is_empty() && !creating_folder() {
                            div { class: "px-2 py-2 text-ui-xs text-muted-foreground", {translate("layout-no-pins-bookmarks")} }
                        } else {
                            div { class: SIDEBAR_TREE_SCROLLER,
                            div { class: "{SIDEBAR_TREE_COLUMN} gap-1",
                                for node in roots.iter() {
                                    match node {
                                        BookmarkNode::Folder(f) if f.parent.is_none() => rsx! {
                                            BookmarkFolder {
                                                key: "{f.uuid}",
                                                folder: f.clone(),
                                                parent_uuid: None,
                                                folders: folders.clone(),
                                                folder_rows: folder_rows.clone(),
                                                active_page: active_page.clone(),
                                                smart: smart.clone(),
                                            }
                                        },
                                        BookmarkNode::Folder(_) => rsx! {},
                                        BookmarkNode::Entry(b) => rsx! {
                                            BookmarkEntry {
                                                key: "{b.uuid}",
                                                row: b.clone(),
                                                folder_uuid: None,
                                                folders: folders.clone(),
                                            }
                                        },
                                    }
                                }
                            }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn set_side_sheet_section(pane_id: u64, section: &str, expanded: bool) {
    let _ = send(&crate::event::SideSheetCommandEvent {
        command: if expanded {
            "expand_section".to_string()
        } else {
            "collapse_section".to_string()
        },
        pane_id: pane_id.to_string(),
        stack_id: 0,
        line: 0,
        path: section.to_string(),
    });
}

#[component]
fn PaneSection(pane: PaneNode, index: usize) -> Element {
    let label = translate_with(
        "layout-stack-number",
        &[("number", TranslationValue::Number((index + 1) as i64))],
    );
    let pane_id = pane.id;
    let any_loading = pane.stacks.iter().any(|s| s.is_loading);
    let expanded = !pane.collapsed;
    let fold_title = if expanded {
        translate("layout-fold-stack")
    } else {
        translate("layout-unfold-stack")
    };
    let visible_stacks = pane
        .stacks
        .iter()
        .filter(|stack| !(stack.url.is_empty() && stack.title == "New Stack"))
        .cloned()
        .collect::<Vec<_>>();
    let active_stack = visible_stacks
        .iter()
        .find(|stack| stack.is_active)
        .or_else(|| visible_stacks.first())
        .cloned();
    rsx! {
        div { class: if pane.is_active && any_loading {
                "glass group mb-2 flex shrink-0 flex-col overflow-hidden rounded-lg pane-loading-ring"
            } else if pane.is_active {
                "glass group mb-2 flex shrink-0 flex-col overflow-hidden rounded-lg ring-2 ring-ring"
            } else {
                "glass group mb-2 flex shrink-0 flex-col overflow-hidden rounded-lg"
            },
            div {
                class: "flex items-center transition-colors hover:bg-glass-hover",
                div { class: "flex min-w-0 flex-1 items-center gap-2 px-2.5 py-2",
                    div { class: "grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-foreground/[0.07] text-foreground ring-1 ring-inset ring-foreground/10",
                        Icon { class: "h-3.5 w-3.5",
                            path { d: "M4 6h16M4 12h16M4 18h16" }
                        }
                    }
                    span {
                        class: if pane.is_active {
                            "min-w-0 flex-1 text-ui font-semibold text-foreground"
                        } else {
                            "min-w-0 flex-1 text-ui font-medium text-muted-foreground"
                        },
                        "{label}"
                    }
                }
                button {
                    r#type: "button",
                    aria_label: "{fold_title}",
                    title: "{fold_title}",
                    class: if expanded {
                        "mr-2 flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-sm text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 hover:bg-foreground/10 hover:text-foreground"
                    } else {
                        "mr-2 flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-sm bg-foreground/10 text-foreground"
                    },
                    onclick: move |_| set_side_sheet_section(pane_id, "pane", !expanded),
                    Icon {
                        class: if expanded { SIDEBAR_CARD_CHEVRON_OPEN } else { SIDEBAR_CARD_CHEVRON_CLOSED },
                        path { d: "m9 18 6-6-6-6" }
                    }
                }
            }
            div { class: "border-t border-foreground/10 p-1.5",
                div { class: "flex flex-col gap-1",
                    if !expanded && let Some(stack) = active_stack {
                        SideSheetStackRow { stack, pane_id }
                    }
                    div { class: if expanded {
                            "grid grid-rows-[1fr] opacity-100 transition-[grid-template-rows,opacity] duration-200 ease-out"
                        } else {
                            "grid grid-rows-[0fr] opacity-0 transition-[grid-template-rows,opacity] duration-200 ease-out"
                        },
                        div { class: "min-h-0 overflow-hidden",
                            div { class: "flex flex-col gap-1",
                                for stack in visible_stacks.iter() {
                                    SideSheetStackRow { stack: stack.clone(), pane_id }
                                }
                                NewStackRow { pane_id }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct BookmarkFolderChoice {
    uuid: String,
    label: String,
    ancestors: Vec<String>,
}

#[derive(Clone, PartialEq)]
enum BookmarkDragItem {
    Page { metadata: PageMetadata },
    Bookmark { uuid: String },
    Pin { uuid: String },
    Folder { uuid: String },
}

#[derive(Clone, PartialEq)]
enum BookmarkDropTarget {
    Root,
    Folder(String),
}

#[derive(Clone, PartialEq)]
struct BookmarkDragState {
    item: BookmarkDragItem,
    start_x: f64,
    start_y: f64,
    ghost_offset_x: f64,
    ghost_offset_y: f64,
    active: bool,
    target: Option<BookmarkDropTarget>,
}

impl BookmarkDragState {
    fn listeners(state: Signal<Option<Self>>) -> Vec<Attribute> {
        if state.read().is_none() {
            return Vec::new();
        }

        vec![
            dioxus_elements::events::onpointermove(move |event| {
                update_bookmark_drag(state, &event)
            }),
            dioxus_elements::events::onpointerup(move |event| end_bookmark_drag(state, &event)),
            dioxus_elements::events::onpointercancel(move |event| {
                cancel_bookmark_drag(state, &event)
            }),
        ]
    }
}

fn bookmark_nodes_contain_url(nodes: &[BookmarkNode], url: &str) -> bool {
    nodes.iter().any(|node| match node {
        BookmarkNode::Entry(bookmark) => bookmark.metadata.url == url,
        BookmarkNode::Folder(folder) => folder
            .children
            .iter()
            .any(|bookmark| bookmark.metadata.url == url),
    })
}

fn bookmark_folder_rows(nodes: &[BookmarkNode]) -> Vec<FolderRow> {
    nodes
        .iter()
        .filter_map(|node| match node {
            BookmarkNode::Folder(folder) => Some(folder.clone()),
            BookmarkNode::Entry(_) => None,
        })
        .collect()
}

fn bookmark_folder_choices(nodes: &[BookmarkNode]) -> Vec<BookmarkFolderChoice> {
    fn collect(
        folders: &[FolderRow],
        parent: Option<&str>,
        parent_label: &str,
        ancestors: &[String],
        visited: &mut std::collections::HashSet<String>,
        output: &mut Vec<BookmarkFolderChoice>,
    ) {
        for folder in folders
            .iter()
            .filter(|folder| folder.parent.as_deref() == parent)
        {
            if !visited.insert(folder.uuid.clone()) {
                continue;
            }
            let label = if parent_label.is_empty() {
                folder.name.clone()
            } else {
                format!("{parent_label} / {}", folder.name)
            };
            output.push(BookmarkFolderChoice {
                uuid: folder.uuid.clone(),
                label: label.clone(),
                ancestors: ancestors.to_vec(),
            });
            let mut child_ancestors = ancestors.to_vec();
            child_ancestors.push(folder.uuid.clone());
            collect(
                folders,
                Some(&folder.uuid),
                &label,
                &child_ancestors,
                visited,
                output,
            );
        }
    }

    let folders = bookmark_folder_rows(nodes);
    let mut output = Vec::new();
    collect(
        &folders,
        None,
        "",
        &[],
        &mut std::collections::HashSet::new(),
        &mut output,
    );
    output
}

#[component]
fn UpdateProgressBar(downloaded: u64, total: u64) -> Element {
    rsx! {
        Progress {
            value: (total > 0).then(|| download_pct(downloaded, total) as f64),
            attributes: vec![],
            ProgressIndicator { attributes: vec![] }
        }
    }
}

#[component]
fn Tab(tab: TabRow) -> Element {
    let id_switch = tab.id.clone();
    let id_close = tab.id.clone();
    let display_title = if !tab.title.is_empty() {
        tab.title.clone()
    } else if !tab.name.is_empty() {
        tab.name.clone()
    } else {
        translate("layout-tab")
    };
    let tooltip = display_title.clone();
    let is_active = tab.is_active;
    let skirt_classes = "relative \
        before:content-[''] before:absolute before:bottom-0 before:-left-2 before:h-2 before:w-2 before:pointer-events-none \
        before:[background:radial-gradient(circle_at_top_left,transparent_0,transparent_8px,var(--tab-bg)_8px)] \
        after:content-[''] after:absolute after:bottom-0 after:-right-2 after:h-2 after:w-2 after:pointer-events-none \
        after:[background:radial-gradient(circle_at_top_right,transparent_0,transparent_8px,var(--tab-bg)_8px)]";
    let tab_box_classes = "group flex h-10 w-52 min-w-52 max-w-52 basis-52 shrink-0 grow-0 -mb-[3px] pb-[3px] cursor-pointer items-center gap-2 px-3.5";

    let trunc = dir_truncate_class(&display_title);
    let (tab_style, tab_class, title_class, close_class) = if is_active {
        (
            "--tab-bg:var(--glass);".to_string(),
            cn([
                skirt_classes,
                tab_box_classes,
                "glass rounded-t-md border-b-0",
            ]),
            cn([
                "min-w-0 flex-1",
                trunc,
                "text-ui font-medium text-foreground",
            ]),
            "flex h-4 w-4 cursor-pointer shrink-0 items-center justify-center rounded-sm opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 hover:bg-foreground/10".to_string(),
        )
    } else {
        (
            String::new(),
            cn([
                tab_box_classes,
                "rounded-md text-muted-foreground hover:bg-glass-hover hover:px-4 hover:text-foreground",
            ]),
            cn(["min-w-0 flex-1", trunc, "text-ui"]),
            "flex h-4 w-4 cursor-pointer shrink-0 items-center justify-center rounded-sm opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 hover:bg-foreground/10".to_string(),
        )
    };

    let bookmark_metadata = PageMetadata {
        title: display_title.clone(),
        url: tab.url.clone(),
        icon: tab.icon.clone(),
        bg_color: tab.bg_color.clone(),
    };
    let pin_metadata = bookmark_metadata.clone();
    let menu_val = use_signal(|| tab.id.clone());

    rsx! {
        LayoutContextMenu {
            ContextMenuTrigger { attributes: vec![],
        div {
            class: "{tab_class}",
            style: "{tab_style}",
            onclick: move |_| {
                let _ = send(&TabsCommandEvent {
                    command: "switch".to_string(),
                    tab_id: Some(id_switch.clone()),
                });
            },
            div {
                title: "{tooltip}",
                class: "flex min-w-0 flex-1 items-center gap-2.5 overflow-hidden",
                HeaderTabIcon {
                    icon: tab.icon.clone(),
                    url: tab.url.clone(),
                    title: display_title.clone(),
                }
                span { class: "{title_class}", "{display_title}" }
            }
            if tab.is_done_unseen {
                span { class: "size-2 shrink-0 rounded-full bg-amber-400 ring-2 ring-background" }
            }
            button {
                r#type: "button",
                aria_label: translate("layout-close-tab"),
                title: translate("layout-close-tab"),
                class: "{close_class}",
                onmousedown: move |evt| {
                    evt.prevent_default();
                    evt.stop_propagation();
                },
                onclick: move |evt| {
                    evt.prevent_default();
                    evt.stop_propagation();
                    let _ = send(&TabsCommandEvent {
                        command: "close".to_string(),
                        tab_id: Some(id_close.clone()),
                    });
                },
                Icon { class: "h-2.5 w-2.5",
                    path { d: "M18 6 6 18" }
                    path { d: "m6 6 12 12" }
                }
            }
        }
            }
            ContextMenuContent { attributes: vec![],
                ContextMenuItem {
                    index: 0usize,
                    value: Into::<ReadSignal<String>>::into(menu_val),
                    on_select: move |_: String| add_to_bookmarks("add", bookmark_metadata.clone(), None),
                    attributes: vec![],
                    {translate("layout-bookmark")}
                }
                ContextMenuItem {
                    index: 1usize,
                    value: Into::<ReadSignal<String>>::into(menu_val),
                    on_select: move |_: String| add_to_bookmarks("pin_url", pin_metadata.clone(), None),
                    attributes: vec![],
                    {translate("layout-pin")}
                }
            }
        }
    }
}

#[component]
fn HeaderTabIcon(icon: PageIcon, url: String, title: String) -> Element {
    if title == "New Stack" && url.is_empty() {
        return rsx! { StackIcon { icon, url, title } };
    }
    let initial_ready = !icon.is_none();
    let mut displayed_icon = use_signal(|| icon.clone());
    let mut displayed_url = use_signal(|| url.clone());
    let mut fallback_ready = use_signal(|| initial_ready);
    let mut generation = use_signal(|| 0_u32);
    use_effect(use_reactive!(|(icon, url)| {
        let next = generation.peek().wrapping_add(1);
        generation.set(next);
        if !icon.is_none() {
            displayed_icon.set(icon);
            displayed_url.set(url);
            fallback_ready.set(true);
            return;
        }
        fallback_ready.set(false);
        let url = url.clone();
        spawn(async move {
            sleep_ms(500).await;
            if generation() != next {
                return;
            }
            displayed_icon.set(PageIcon::None);
            displayed_url.set(url);
            fallback_ready.set(true);
        });
    }));
    let shown_icon = displayed_icon();
    if shown_icon.is_none() && !fallback_ready() {
        return rsx! { span { class: "h-4 w-4 shrink-0" } };
    }
    rsx! {
        StackIcon {
            icon: shown_icon,
            url: displayed_url(),
            title,
        }
    }
}

#[component]
fn NewTabButton() -> Element {
    rsx! {
        button {
            r#type: "button",
            aria_label: translate("layout-new-tab"),
            title: translate("layout-new-tab"),
            class: "flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-glass-hover hover:text-foreground active:bg-glass-active active:text-foreground",
            onclick: move |_| {
                let _ = send(&TabsCommandEvent {
                    command: "new".to_string(),
                    tab_id: None,
                });
            },
            Icon { class: "h-3.5 w-3.5",
                path { d: "M12 5v14" }
                path { d: "M5 12h14" }
            }
        }
    }
}

#[component]
fn WindowDragRegion() -> Element {
    let mut region = use_signal(|| None::<Rc<MountedData>>);
    let publish = move || {
        spawn(async move {
            let Some(region) = region() else {
                return;
            };
            let Ok(rect) = region.get_client_rect().await else {
                return;
            };
            let _ = send(&WindowDragRegionEvent {
                left: rect.origin.x as f32,
                top: rect.origin.y as f32,
                width: rect.size.width as f32,
                height: rect.size.height as f32,
            });
        });
    };

    rsx! {
        div {
            class: "h-10 min-w-0 flex-1 self-stretch",
            onmounted: move |event: Event<MountedData>| {
                region.set(Some(event.data()));
                publish();
            },
            onresize: move |_: Event<ResizeData>| publish(),
        }
    }
}

#[component]
fn NavButton(
    label: String,
    command: &'static str,
    #[props(default)] disabled: bool,
    children: Element,
) -> Element {
    let class = if disabled {
        "flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground/40 transition-colors cursor-default"
    } else {
        "cursor-pointer flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-glass-hover hover:text-foreground active:bg-glass-active active:text-foreground"
    };
    rsx! {
        button {
            r#type: "button",
            aria_label: "{label}",
            title: "{label}",
            disabled,
            class,
            onclick: move |_| {
                if !disabled {
                    let _ = send(&HeaderCommandEvent {
                        header_command: command.to_string(),
                    });
                }
            },
            {children}
        }
    }
}

#[component]
fn HeaderAddressBar(active_row: Option<StackRow>, bg_color: Option<String>) -> Element {
    let has_content = active_row.as_ref().is_some_and(|row| !row.url.is_empty());
    let address = active_row.map(|row| row.address).unwrap_or_default();
    let empty_class = if bg_color.is_some() {
        "opacity-50"
    } else {
        "text-muted-foreground"
    };

    rsx! {
        div {
            class: "flex h-8 min-w-0 flex-1 cursor-pointer items-center gap-2",
            onclick: move |_| {
                let _ = send(&HeaderCommandEvent {
                    header_command: "focus_address_bar".to_string(),
                });
            },
            if !has_content {
                span { class: "truncate text-ui {empty_class}", {translate("layout-new-stack")} }
            } else {
                if !address.origin.is_empty() {
                    span {
                        class: "shrink-0 rounded-full bg-foreground/10 px-2 py-0.5 text-ui leading-tight",
                        "{address.origin}"
                    }
                }
                span { class: "min-w-0 truncate text-ui", "{address.rest}" }
            }
        }
    }
}

#[component]
fn TeamFacepile(members: Vec<TeamMemberRow>) -> Element {
    if members.is_empty() {
        return rsx! {};
    }
    let user = members.iter().find(|m| m.is_user).cloned();
    let agents: Vec<TeamMemberRow> = members.iter().filter(|m| !m.is_user).cloned().collect();
    let max = 5usize;
    let overflow = agents.len().saturating_sub(max);
    rsx! {
        div {
            class: "flex shrink-0 items-center gap-2 pl-3 pr-3",
            if let Some(user) = user {
                div {
                    class: "flex items-center gap-1.5 rounded-full bg-foreground/10 py-0.5 pl-0.5 pr-2.5 cursor-pointer transition-opacity hover:opacity-80",
                    title: translate("layout-team"),
                    onclick: move |_| {
                        let _ = send(&TeamCommandEvent {
                            command: "open".to_string(),
                            member_id: None,
                        });
                    },
                    Avatar {
                        src: None,
                        fallback: user.initials.clone(),
                        background: user.color.clone(),
                        alt: user.name.clone(),
                        class: "size-5 text-[9px]",
                    }
                    span { class: "whitespace-nowrap text-xs font-medium text-foreground", "{user.name}" }
                }
            }
            if !agents.is_empty() {
                div { class: "flex items-center -space-x-1.5",
                    for m in agents.iter().take(max) {
                        {
                            let src = favicon_src_for_url(&m.icon, &m.url);
                            let id = m.id.clone();
                            rsx! {
                                div {
                                    key: "{m.id}",
                                    title: "{m.name}",
                                    class: "relative inline-flex size-5 shrink-0 cursor-pointer transition-opacity hover:opacity-80",
                                    onclick: move |_| {
                                        let _ = send(&TeamCommandEvent {
                                            command: "focus".to_string(),
                                            member_id: Some(id.clone()),
                                        });
                                    },
                                    Avatar {
                                        src,
                                        fallback: m.initials.clone(),
                                        background: m.color.clone(),
                                        alt: m.name.clone(),
                                        class: "size-5 text-[9px] ring-2 ring-background",
                                    }
                                    if m.is_running {
                                        span { class: "absolute -bottom-0.5 -right-0.5 size-1.5 rounded-full bg-success ring-2 ring-background" }
                                    } else if m.is_done_unseen {
                                        span { class: "absolute -bottom-0.5 -right-0.5 size-2 rounded-full bg-amber-400 ring-2 ring-background" }
                                    }
                                }
                            }
                        }
                    }
                    if overflow > 0 {
                        div {
                            class: "relative inline-flex size-5 items-center justify-center rounded-full ring-2 ring-background bg-muted text-[9px] font-medium text-muted-foreground cursor-pointer transition-opacity hover:opacity-80",
                            title: translate("layout-team"),
                            onclick: move |_| {
                                let _ = send(&TeamCommandEvent {
                                    command: "open".to_string(),
                                    member_id: None,
                                });
                            },
                            "+{overflow}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ExtensionBar(extensions: Vec<ExtRow>) -> Element {
    rsx! {
        div { class: "flex shrink-0 items-center gap-1 pl-1",
            for ext in extensions.iter().filter(|e| e.enabled && e.icon.is_some()) {
                {
                    let id = ext.id.clone();
                    let name = ext.name.clone();
                    let icon = ext.icon.clone().unwrap_or_default();
                    rsx! {
                        button {
                            key: "{ext.id}",
                            class: "flex h-7 w-7 items-center justify-center rounded-lg hover:bg-foreground/[0.08]",
                            title: "{name}",
                            onclick: move |_| { let _ = send(&ExtActionRequest { id: id.clone() }); },
                            img { class: "h-4 w-4", src: "{icon}" }
                        }
                    }
                }
            }
            button {
                class: "flex h-7 w-7 items-center justify-center rounded-lg text-foreground/80 hover:bg-foreground/[0.08]",
                title: translate("layout-manage-extensions"),
                onclick: move |_| { let _ = send(&ExtOpenManagerRequest); },
                Icon { class: "h-4 w-4",
                    path { d: "M20.5 11H19V7c0-1.1-.9-2-2-2h-4V3.5C13 2.12 11.88 1 10.5 1S8 2.12 8 3.5V5H4c-1.1 0-1.99.9-1.99 2v3.8H3.5c1.49 0 2.7 1.21 2.7 2.7s-1.21 2.7-2.7 2.7H2V20c0 1.1.9 2 2 2h3.8v-1.5c0-1.49 1.21-2.7 2.7-2.7 1.49 0 2.7 1.21 2.7 2.7V22H17c1.1 0 2-.9 2-2v-4h1.5c1.38 0 2.5-1.12 2.5-2.5S21.88 11 20.5 11z" }
                }
            }
        }
    }
}

#[component]
fn PinTile(row: BookmarkRow) -> Element {
    let drag_state: Signal<Option<BookmarkDragState>> = use_context();
    let url_open = row.metadata.url.clone();
    let uuid_unpin = row.uuid.clone();
    let menu_val = use_signal(|| row.uuid.clone());
    let drag_item = BookmarkDragItem::Pin {
        uuid: row.uuid.clone(),
    };
    rsx! {
        BookmarkContextMenu {
            command: "menu_pin".to_string(),
            uuid: Some(row.uuid.clone()),
            metadata: None,
            trigger: rsx! {
                div {
                    "data-bookmark-drag-source": "true",
                    onpointerdown: {
                        let item = drag_item.clone();
                        move |event| begin_bookmark_drag(drag_state, &event, item.clone())
                    },
                    class: "flex aspect-square cursor-pointer items-center justify-center rounded-md bg-white/5 hover:bg-white/10",
                    onclick: {
                        let u = url_open.clone();
                        move |event| {
                            if bookmark_drag_blocks_click(drag_state) {
                                event.prevent_default();
                                event.stop_propagation();
                                return;
                            }
                            open_bookmark(u.clone());
                        }
                    },
                    title: "{row.metadata.title}",
                    PageIconView {
                        icon: row.metadata.icon.clone(),
                        url: row.metadata.url.clone(),
                        img_class: "h-5 w-5 shrink-0 rounded-sm object-contain".to_string(),
                        icon_class: "h-5 w-5 shrink-0 text-muted-foreground".to_string(),
                    }
                }
            },
            menu: rsx! {
                SideSheetContextMenuContent {
                    ContextMenuItem {
                        index: 0usize,
                        value: Into::<ReadSignal<String>>::into(menu_val),
                        on_select: { let u = url_open.clone(); move |_: String| open_bookmark(u.clone()) },
                        attributes: vec![],
                        {translate("common-open")}
                    }
                    ContextMenuItem {
                        index: 1usize,
                        value: Into::<ReadSignal<String>>::into(menu_val),
                        on_select: { let id = uuid_unpin.clone(); move |_: String| bookmark_cmd("unpin", Some(id.clone())) },
                        attributes: vec![],
                        {translate("layout-unpin-page")}
                    }
                    if row.bookmarked {
                        ContextMenuItem {
                            index: 2usize,
                            value: Into::<ReadSignal<String>>::into(menu_val),
                            on_select: { let id = row.uuid.clone(); move |_: String| bookmark_cmd("remove", Some(id.clone())) },
                            attributes: vec![],
                            {translate("layout-remove-bookmark")}
                        }
                    }
                }
            },
        }
    }
}

fn open_bookmark(url: String) {
    let _ = send(&BookmarksCommandEvent {
        command: "open".into(),
        url: Some(url),
        uuid: None,
        name: None,
        metadata: None,
        folder: None,
    });
}

fn bookmark_cmd(command: &str, uuid: Option<String>) {
    let _ = send(&BookmarksCommandEvent {
        command: command.into(),
        uuid,
        name: None,
        url: None,
        metadata: None,
        folder: None,
    });
}

fn add_to_bookmarks(command: &str, metadata: PageMetadata, folder: Option<String>) {
    let _ = send(&BookmarksCommandEvent {
        command: command.into(),
        uuid: None,
        name: None,
        url: None,
        metadata: Some(metadata),
        folder,
    });
}

fn move_bookmark(uuid: String, folder: Option<String>) {
    let _ = send(&BookmarksCommandEvent {
        command: "move".into(),
        uuid: Some(uuid),
        name: None,
        url: None,
        metadata: None,
        folder,
    });
}

fn move_pin(uuid: String, folder: Option<String>) {
    let _ = send(&BookmarksCommandEvent {
        command: "move_pin".into(),
        uuid: Some(uuid),
        name: None,
        url: None,
        metadata: None,
        folder,
    });
}

fn move_bookmark_folder(uuid: String, folder: Option<String>) {
    let _ = send(&BookmarksCommandEvent {
        command: "move_folder".into(),
        uuid: Some(uuid),
        name: None,
        url: None,
        metadata: None,
        folder,
    });
}

fn commit_bookmark_rename(uuid: String, name: String) {
    let name = name.trim().to_string();
    if name.is_empty() {
        return;
    }
    let _ = send(&BookmarksCommandEvent {
        command: "rename".into(),
        uuid: Some(uuid),
        name: Some(name),
        url: None,
        metadata: None,
        folder: None,
    });
}

fn create_bookmark_folder(name: String, parent: Option<String>) {
    let name = name.trim().to_string();
    if name.is_empty() {
        return;
    }
    let _ = send(&BookmarksCommandEvent {
        command: "new_folder".into(),
        uuid: None,
        name: Some(name),
        url: None,
        metadata: None,
        folder: parent,
    });
}

fn begin_bookmark_drag(
    mut state: Signal<Option<BookmarkDragState>>,
    event: &Event<PointerData>,
    item: BookmarkDragItem,
) {
    if event.trigger_button() != Some(MouseButton::Primary) {
        return;
    }
    set_bookmark_pointer_capture(event, true);
    let coordinates = event.client_coordinates();
    let grab = bookmark_grab_offset(event);
    state.set(Some(BookmarkDragState {
        item,
        start_x: coordinates.x,
        start_y: coordinates.y,
        ghost_offset_x: grab.map(|(x, _)| x).unwrap_or(12.0),
        ghost_offset_y: grab.map(|(_, y)| y).unwrap_or(12.0),
        active: false,
        target: None,
    }));
}

fn update_bookmark_drag(mut state: Signal<Option<BookmarkDragState>>, event: &Event<PointerData>) {
    let Some(mut drag) = state() else {
        return;
    };
    let coordinates = event.client_coordinates();
    let dx = coordinates.x - drag.start_x;
    let dy = coordinates.y - drag.start_y;
    if !drag.active && dx * dx + dy * dy < 16.0 {
        return;
    }
    let target = bookmark_drop_target_at(event);
    if !drag.active {
        set_bookmark_context_menu_active(true);
        create_bookmark_drag_ghost(event);
    }
    move_bookmark_drag_ghost(
        coordinates.x - drag.ghost_offset_x,
        coordinates.y - drag.ghost_offset_y,
    );
    if !drag.active || drag.target != target {
        drag.active = true;
        drag.target = target;
        state.set(Some(drag));
    }
}

fn perform_bookmark_drop(item: BookmarkDragItem, target: BookmarkDropTarget) {
    let folder = match target {
        BookmarkDropTarget::Root => None,
        BookmarkDropTarget::Folder(uuid) => Some(uuid),
    };
    match item {
        BookmarkDragItem::Page { metadata } => add_to_bookmarks("add", metadata, folder),
        BookmarkDragItem::Bookmark { uuid, .. } => move_bookmark(uuid, folder),
        BookmarkDragItem::Pin { uuid } => move_pin(uuid, folder),
        BookmarkDragItem::Folder { uuid, .. } => {
            if folder.as_deref() != Some(uuid.as_str()) {
                move_bookmark_folder(uuid, folder);
            }
        }
    }
}

fn set_root_radius_px(_radius: f32) {}

fn clear_bookmark_drag_after_click(mut state: Signal<Option<BookmarkDragState>>) {
    spawn(async move {
        sleep_ms(0).await;
        state.set(None);
    });
}

fn end_bookmark_drag(mut state: Signal<Option<BookmarkDragState>>, event: &Event<PointerData>) {
    set_bookmark_pointer_capture(event, false);
    let Some(mut drag) = state() else {
        return;
    };
    let coordinates = event.client_coordinates();
    let dx = coordinates.x - drag.start_x;
    let dy = coordinates.y - drag.start_y;
    if !drag.active && dx * dx + dy * dy < 16.0 {
        state.set(None);
        return;
    }
    let target = bookmark_drop_target_at(event);
    event.prevent_default();
    event.stop_propagation();
    drag.active = true;
    drag.target = target.clone();
    remove_bookmark_drag_ghost();
    set_bookmark_context_menu_active(false);
    if let Some(target) = target {
        perform_bookmark_drop(drag.item.clone(), target);
    }
    state.set(Some(drag));
    clear_bookmark_drag_after_click(state);
}

fn cancel_bookmark_drag(mut state: Signal<Option<BookmarkDragState>>, event: &Event<PointerData>) {
    set_bookmark_pointer_capture(event, false);
    remove_bookmark_drag_ghost();
    set_bookmark_context_menu_active(false);
    state.set(None);
}

mod bookmark_drag_dom {
    use super::*;

    pub(super) fn bookmark_grab_offset(_event: &Event<PointerData>) -> Option<(f64, f64)> {
        None
    }

    pub(super) fn set_bookmark_pointer_capture(_event: &Event<PointerData>, _capture: bool) {}

    pub(super) fn create_bookmark_drag_ghost(_event: &Event<PointerData>) {}

    pub(super) fn move_bookmark_drag_ghost(_left: f64, _top: f64) {}

    pub(super) fn remove_bookmark_drag_ghost() {}

    pub(super) fn bookmark_drop_target_at(
        _event: &Event<PointerData>,
    ) -> Option<BookmarkDropTarget> {
        None
    }
}

use bookmark_drag_dom::{
    bookmark_drop_target_at, bookmark_grab_offset, create_bookmark_drag_ghost,
    move_bookmark_drag_ghost, remove_bookmark_drag_ghost, set_bookmark_pointer_capture,
};

fn bookmark_drag_blocks_click(state: Signal<Option<BookmarkDragState>>) -> bool {
    state().is_some_and(|drag| drag.active)
}

fn bookmark_drop_targeted(
    state: Signal<Option<BookmarkDragState>>,
    target: &BookmarkDropTarget,
) -> bool {
    state().is_some_and(|drag| drag.active && drag.target.as_ref() == Some(target))
}

fn set_bookmark_text_input_active(active: bool) {
    let _ = send(&BookmarkTextInputEvent { active });
}

fn set_bookmark_context_menu_active(active: bool) {
    let _ = send(&BookmarkContextMenuEvent { active });
}

#[component]
fn BookmarkNameInput(
    draft: Signal<String>,
    class: String,
    placeholder: String,
    on_commit: EventHandler<String>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut draft = draft;
    let mut finished = use_signal(|| false);
    use_drop(move || set_bookmark_text_input_active(false));

    rsx! {
        input {
            r#type: "text",
            class,
            placeholder,
            value: "{draft}",
            autofocus: true,
            oncontextmenu: move |event: Event<MouseData>| {
                event.prevent_default();
                event.stop_propagation();
            },
            onmounted: move |event| {
                set_bookmark_text_input_active(true);
                focus_and_select_inline_rename(event);
            },
            oninput: move |event| draft.set(event.value()),
            onkeydown: move |event: Event<KeyboardData>| match event.key() {
                Key::Enter => {
                    event.prevent_default();
                    if !finished() {
                        finished.set(true);
                        set_bookmark_text_input_active(false);
                        on_commit.call(draft());
                    }
                }
                Key::Escape => {
                    event.prevent_default();
                    if !finished() {
                        finished.set(true);
                        set_bookmark_text_input_active(false);
                        on_cancel.call(());
                    }
                }
                _ => {}
            },
            onblur: move |_| {
                if !finished() {
                    finished.set(true);
                    set_bookmark_text_input_active(false);
                    on_commit.call(draft());
                }
            },
        }
    }
}

fn begin_inline_rename(mut editing: Signal<bool>, mut draft: Signal<String>, name: String) {
    draft.set(name);
    spawn(async move {
        sleep_ms(0).await;
        editing.set(true);
    });
}

fn focus_and_select_inline_rename(_event: Event<MountedData>) {}

#[component]
fn BookmarkFolder(
    folder: FolderRow,
    parent_uuid: Option<String>,
    folders: Vec<BookmarkFolderChoice>,
    folder_rows: Vec<FolderRow>,
    active_page: Option<StackNode>,
    smart: SmartBookmarkContent,
) -> Element {
    let drag_state: Signal<Option<BookmarkDragState>> = use_context();
    let uuid = folder.uuid.clone();
    let collapsed = folder.collapsed;
    let mut editing = use_signal(|| false);
    let draft = use_signal(|| folder.name.clone());
    let mut creating_child = use_signal(|| false);
    let child_draft = use_signal(|| translate("layout-new-folder"));
    let menu_val = use_signal(|| folder.uuid.clone());
    let new_folder_uuid = uuid.clone();
    let bookmark_menu_action: Signal<BookmarkMenuActionEvent> = use_context();
    let initial_menu_action = bookmark_menu_action.peek().sequence;
    let mut handled_menu_action = use_signal(|| initial_menu_action);
    let menu_action_uuid = uuid.clone();
    let menu_action_name = folder.name.clone();
    use_effect(move || {
        let action = bookmark_menu_action();
        if action.sequence == handled_menu_action() {
            return;
        }
        handled_menu_action.set(action.sequence);
        if action.uuid.as_deref() != Some(menu_action_uuid.as_str()) {
            return;
        }
        match action.action.as_str() {
            "new_folder" => begin_new_folder(creating_child, child_draft),
            "rename" => begin_inline_rename(editing, draft, menu_action_name.clone()),
            _ => {}
        }
    });
    let mut move_targets = Vec::new();
    if parent_uuid.is_some() {
        move_targets.push((None, translate("layout-move-to-bookmarks")));
    }
    move_targets.extend(
        folders
            .iter()
            .filter(|target| target.uuid != folder.uuid && !target.ancestors.contains(&folder.uuid))
            .map(|target| {
                (
                    Some(target.uuid.clone()),
                    translate_with(
                        "layout-move-to",
                        &[("folder", TranslationValue::String(&target.label))],
                    ),
                )
            }),
    );
    let remove_index = 4 + move_targets.len();
    let drop_target = BookmarkDropTarget::Folder(uuid.clone());
    let folder_targeted = bookmark_drop_targeted(drag_state, &drop_target);
    let drag_item = BookmarkDragItem::Folder { uuid: uuid.clone() };
    let mut child_folders = 0usize;
    for child in folder_rows.iter() {
        if child.parent.as_deref() == Some(folder.uuid.as_str()) {
            child_folders += 1;
        }
    }
    let smart_count = folder.smart.map_or(0, |kind| smart.count(kind));
    let child_count = child_folders + folder.children.len() + smart_count;
    let folder_is_empty = child_count == 0 && folder.smart.is_none();
    let active_metadata = active_page.clone().map(|page| PageMetadata {
        title: page.title,
        url: page.url,
        icon: page.icon,
        bg_color: page.bg_color,
    });

    rsx! {
        div {
            "data-bookmark-drop": "{uuid}",
            class: "flex flex-col",
            if editing() {
                div { class: "flex h-9 items-center gap-2 rounded-md border border-transparent px-2",
                    Icon {
                        class: if collapsed {
                            "h-4 w-4 shrink-0 rotate-0 text-muted-foreground transition-transform duration-200 ease-out"
                        } else {
                            "h-4 w-4 shrink-0 rotate-90 text-muted-foreground transition-transform duration-200 ease-out"
                        },
                        path { d: "m9 18 6-6-6-6" }
                    }
                    BookmarkNameInput {
                        draft,
                        class: "min-w-0 flex-1 bg-transparent text-ui font-medium text-foreground outline-none".to_string(),
                        placeholder: translate("layout-folder-name"),
                        on_commit: {
                            let id = uuid.clone();
                            move |name| {
                                editing.set(false);
                                commit_folder_rename(id.clone(), name);
                            }
                        },
                        on_cancel: move |_| editing.set(false),
                    }
                }
            } else {
                BookmarkContextMenu {
                    command: "menu_folder".to_string(),
                    uuid: Some(folder.uuid.clone()),
                    metadata: active_metadata,
                    trigger: rsx! {
                        div {
                            "data-bookmark-drag-source": "true",
                            class: if folder_targeted { "rounded-md ring-2 ring-ring" } else { "rounded-md" },
                            onpointerdown: {
                                let item = drag_item.clone();
                                move |event| begin_bookmark_drag(drag_state, &event, item.clone())
                            },
                            SidebarTreeRowGroup {
                                SidebarTreeRow {
                                    path: uuid.clone(),
                                    label: folder.name.clone(),
                                    is_dir: true,
                                    expanded: !collapsed,
                                    emphasis: true,
                                    title: folder.name.clone(),
                                    on_activate: {
                                        let id = uuid.clone();
                                        move |()| {
                                            if bookmark_drag_blocks_click(drag_state) {
                                                return;
                                            }
                                            bookmark_cmd("toggle_folder", Some(id.clone()));
                                        }
                                    },
                                    trailing: rsx! {
                                        if child_count > 0 {
                                            span { class: "shrink-0 text-[10px] tabular-nums text-muted-foreground/70",
                                                "{child_count}"
                                            }
                                        }
                                    },
                                }
                            }
                        }
                    },
                    menu: rsx! {
                        SideSheetContextMenuContent {
                        ContextMenuItem {
                            index: 0usize,
                            value: Into::<ReadSignal<String>>::into(menu_val),
                            on_select: { let id = uuid.clone(); move |_: String| bookmark_cmd("toggle_folder", Some(id.clone())) },
                            attributes: vec![],
                            {if collapsed { translate("common-expand") } else { translate("common-collapse") }}
                        }
                        ContextMenuItem {
                            index: 1usize,
                            value: Into::<ReadSignal<String>>::into(menu_val),
                            disabled: active_page.is_none(),
                            on_select: {
                                let id = uuid.clone();
                                let page = active_page.clone();
                                move |_: String| {
                                    if let Some(page) = page.clone() {
                                        add_to_bookmarks(
                                            "add",
                                            PageMetadata {
                                                title: page.title,
                                                url: page.url,
                                                icon: page.icon,
                                                bg_color: page.bg_color,
                                            },
                                            Some(id.clone()),
                                        );
                                    }
                                }
                            },
                            attributes: vec![],
                            {translate("layout-bookmark-current-page")}
                        }
                        ContextMenuItem {
                            index: 2usize,
                            value: Into::<ReadSignal<String>>::into(menu_val),
                            on_select: move |_: String| {
                                if collapsed {
                                    bookmark_cmd("toggle_folder", Some(new_folder_uuid.clone()));
                                }
                                begin_new_folder(creating_child, child_draft);
                            },
                            attributes: vec![],
                            {translate("layout-new-folder")}
                        }
                        ContextMenuItem {
                            index: 3usize,
                            value: Into::<ReadSignal<String>>::into(menu_val),
                            on_select: {
                                let name = folder.name.clone();
                                move |_: String| begin_inline_rename(editing, draft, name.clone())
                            },
                            attributes: vec![],
                            {translate("layout-rename-folder")}
                        }
                        for (index, (target_folder, label)) in move_targets.iter().enumerate() {
                            ContextMenuItem {
                                key: "{index}",
                                index: 4usize + index,
                                value: Into::<ReadSignal<String>>::into(menu_val),
                                on_select: {
                                    let id = uuid.clone();
                                    let folder = target_folder.clone();
                                    move |_: String| move_bookmark_folder(id.clone(), folder.clone())
                                },
                                attributes: vec![],
                                "{label}"
                            }
                        }
                        ContextMenuItem {
                            index: remove_index,
                            value: Into::<ReadSignal<String>>::into(menu_val),
                            on_select: { let id = uuid.clone(); move |_: String| bookmark_cmd("remove_folder", Some(id.clone())) },
                            attributes: vec![],
                            {translate("layout-remove-folder")}
                        }
                        }
                    },
                }
            }
            if creating_child() {
                div { class: "ml-3 flex h-9 items-center gap-2 rounded-md border border-transparent px-2",
                    Icon { class: "h-4 w-4 shrink-0 text-muted-foreground",
                        path { d: "M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z" }
                    }
                    BookmarkNameInput {
                        draft: child_draft,
                        class: "min-w-0 flex-1 bg-transparent text-ui font-medium text-foreground outline-none".to_string(),
                        placeholder: translate("layout-folder-name"),
                        on_commit: {
                            let parent = uuid.clone();
                            move |name| {
                                creating_child.set(false);
                                create_bookmark_folder(name, Some(parent.clone()));
                            }
                        },
                        on_cancel: move |_| creating_child.set(false),
                    }
                }
            }
            SidebarTreeChildren { expanded: !collapsed,
                div { class: "ml-3 flex flex-col gap-1",
                    for child_folder in folder_rows
                        .iter()
                        .filter(|child| child.parent.as_deref() == Some(folder.uuid.as_str()))
                    {
                        BookmarkFolder {
                            key: "{child_folder.uuid}",
                            folder: child_folder.clone(),
                            parent_uuid: Some(folder.uuid.clone()),
                            folders: folders.clone(),
                            folder_rows: folder_rows.clone(),
                            active_page: active_page.clone(),
                            smart: smart.clone(),
                        }
                    }
                    if let Some(kind) = folder.smart {
                        SmartBookmarkRows { kind, smart: smart.clone() }
                    }
                    for bookmark in folder.children.iter() {
                        BookmarkEntry {
                            key: "{bookmark.uuid}",
                            row: bookmark.clone(),
                            folder_uuid: Some(folder.uuid.clone()),
                            folders: folders.clone(),
                        }
                    }
                    if folder_is_empty {
                        div { class: "px-2 py-1.5 text-ui-xs text-muted-foreground", {translate("layout-empty-folder")} }
                    }
                }
            }
        }
    }
}

#[component]
fn SmartBookmarkRows(kind: vmux_core::SmartBookmarkFolder, smart: SmartBookmarkContent) -> Element {
    match kind {
        vmux_core::SmartBookmarkFolder::Projects => rsx! {
            if smart.projects.is_empty() {
                div { class: "px-2 py-1.5 text-ui-xs text-muted-foreground", {translate("layout-no-project-selected")} }
            } else {
                for project in smart.projects.iter().cloned() {
                    SmartProjectRow { project, pane_id: smart.pane_id }
                }
            }
        },
        vmux_core::SmartBookmarkFolder::Knowledge => rsx! {
            SmartKnowledgeRows {
                knowledge: smart.knowledge.clone(),
                loaded: smart.knowledge_loaded,
                pane_id: smart.pane_id,
            }
        },
        vmux_core::SmartBookmarkFolder::Tools => rsx! {
            SmartToolsRows {
                tools: smart.tools.clone(),
                loaded: smart.tools_loaded,
                pane_id: smart.pane_id,
            }
        },
    }
}

fn emit_project_command(command: &str, path: Option<String>) {
    let _ = send(&vmux_wire::space::ProjectCommandEvent {
        command: command.to_string(),
        path,
    });
}

fn open_project_path(pane_id: u64, path: String) {
    let _ = send(&crate::event::SideSheetCommandEvent {
        command: "open_project_path".to_string(),
        pane_id: pane_id.to_string(),
        stack_id: 0,
        line: 0,
        path,
    });
}

#[component]
fn SmartProjectRow(project: vmux_core::event::ProjectRow, pane_id: u64) -> Element {
    let tree = project.kind.opens_a_tree();
    let root = matches!(project.kind, vmux_core::event::ProjectRowKind::Project);
    let activate = project.path.clone();
    let forget = project.path.clone();
    let forget_title = translate("layout-project-forget");
    let activate_title = translate("layout-project-activate");
    rsx! {
        SidebarTreeRowGroup {
            SidebarTreeRow {
                path: project.path.clone(),
                label: project.label.clone(),
                is_dir: tree,
                expanded: project.expanded,
                depth: project.depth,
                emphasis: root,
                title: project.display_path.clone(),
                on_activate: move |()| match tree {
                    true => {
                        let _ = send(&vmux_core::event::ProjectTreeToggle {
                            path: activate.clone(),
                            pane_id: pane_id.to_string(),
                        });
                    }
                    false => open_project_path(pane_id, activate.clone()),
                },
                label_suffix: rsx! {
                    if !project.branch.is_empty() {
                        span { class: "shrink-0 truncate font-mono text-[10px] text-muted-foreground/70", "{project.branch}" }
                    }
                },
            }
            if root {
                if project.is_active {
                    span {
                        aria_label: "{activate_title}",
                        title: "{activate_title}",
                        class: "mr-1 flex h-6 w-6 shrink-0 items-center justify-center rounded-sm text-foreground",
                        Icon { class: "h-3.5 w-3.5 pointer-events-none",
                            path { d: "M12 17v5" }
                            path { d: "M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1Z" }
                        }
                    }
                }
                button {
                    r#type: "button",
                    aria_label: "{forget_title}",
                    title: "{forget_title}",
                    class: "mr-1.5 flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-sm text-muted-foreground opacity-0 transition-opacity group-hover/row:opacity-100 focus-visible:opacity-100 hover:bg-foreground/10 hover:text-foreground",
                    onclick: move |_| emit_project_command("forget", Some(forget.clone())),
                    Icon { class: "h-3 w-3 pointer-events-none",
                        path { d: "M18 6 6 18M6 6l12 12" }
                    }
                }
            }
        }
    }
}

fn toggle_knowledge_dir(pane_id: u64, path: String) {
    let _ = send(&vmux_core::knowledge::KnowledgeTreeToggle {
        path,
        pane_id: pane_id.to_string(),
    });
}

fn open_knowledge_path(pane_id: u64, path: String) {
    let _ = send(&crate::event::SideSheetCommandEvent {
        command: "open_knowledge_path".to_string(),
        pane_id: pane_id.to_string(),
        stack_id: 0,
        line: 0,
        path,
    });
}

#[component]
fn SmartKnowledgeRows(knowledge: KnowledgeTreeEvent, loaded: bool, pane_id: u64) -> Element {
    if !loaded {
        return rsx! {
            div { class: "px-2 py-1.5 text-ui-xs text-muted-foreground", {translate("layout-loading")} }
        };
    }
    if !knowledge.error.is_empty() {
        return rsx! {
            div { class: "px-2 py-1.5 text-ui-xs text-destructive", "{knowledge.error}" }
        };
    }
    if knowledge.entries.is_empty() {
        return rsx! {
            div { class: "px-2 py-1.5 text-ui-xs text-muted-foreground", {translate("layout-no-markdown-files")} }
        };
    }
    let entries_by_parent = Rc::new(knowledge.entries.iter().cloned().fold(
        HashMap::<String, Vec<KnowledgeEntry>>::new(),
        |mut grouped, entry| {
            grouped.entry(entry.parent.clone()).or_default().push(entry);
            grouped
        },
    ));
    rsx! {
        for entry in entries_by_parent.get(&knowledge.root).into_iter().flatten() {
            SmartKnowledgeRow {
                key: "{entry.path}",
                entry: entry.clone(),
                entries_by_parent: entries_by_parent.clone(),
                pane_id,
            }
        }
    }
}

#[component]
fn SmartKnowledgeRow(
    entry: KnowledgeEntry,
    entries_by_parent: Rc<HashMap<String, Vec<KnowledgeEntry>>>,
    pane_id: u64,
) -> Element {
    if entry.is_directory {
        let children = entries_by_parent.get(&entry.path);
        let toggle_path = entry.path.clone();
        return rsx! {
            div { class: "flex flex-col",
                SidebarTreeRowGroup {
                    SidebarTreeRow {
                        path: entry.path.clone(),
                        label: entry.name.clone(),
                        is_dir: true,
                        expanded: entry.expanded,
                        emphasis: true,
                        on_activate: move |()| toggle_knowledge_dir(pane_id, toggle_path.clone()),
                        trailing: rsx! { KnowledgeGitIndicator { status: entry.git_status } },
                    }
                }
                SidebarTreeChildren { expanded: entry.expanded,
                    div { class: "ml-3 flex flex-col gap-0.5 border-l border-foreground/10 pl-1.5",
                        if let Some(children) = children {
                            for child in children {
                                SmartKnowledgeRow {
                                    key: "{child.path}",
                                    entry: child.clone(),
                                    entries_by_parent: entries_by_parent.clone(),
                                    pane_id,
                                }
                            }
                        } else {
                            div { class: "px-2 py-1.5 text-ui-xs text-muted-foreground", {translate("layout-empty-folder")} }
                        }
                    }
                }
            }
        };
    }
    let path = entry.path.clone();
    let title = if entry.title.is_empty() {
        entry.name.clone()
    } else {
        entry.title.clone()
    };
    rsx! {
        SidebarTreeRowGroup {
            SidebarTreeRow {
                path: entry.path.clone(),
                label: title,
                is_dir: false,
                on_activate: move |()| open_knowledge_path(pane_id, path.clone()),
                trailing: rsx! { KnowledgeGitIndicator { status: entry.git_status } },
            }
        }
    }
}

#[component]
fn KnowledgeGitIndicator(status: KnowledgeGitStatus) -> Element {
    let (class, title) = match status {
        KnowledgeGitStatus::Clean => return rsx! {},
        KnowledgeGitStatus::Added => ("bg-ansi-2", translate("git-status-untracked")),
        KnowledgeGitStatus::Modified => ("bg-ansi-3", translate("git-status-modified")),
        KnowledgeGitStatus::Deleted => ("bg-ansi-1", translate("git-status-deleted")),
    };
    rsx! {
        span { class: "h-2 w-2 shrink-0 rounded-full {class}", title: "{title}" }
    }
}

fn open_tools(pane_id: u64) {
    let _ = send(&crate::event::SideSheetCommandEvent {
        command: "open_tools".to_string(),
        pane_id: pane_id.to_string(),
        stack_id: 0,
        line: 0,
        path: String::new(),
    });
}

#[component]
fn SmartToolsRows(tools: ToolsSnapshot, loaded: bool, pane_id: u64) -> Element {
    if !loaded {
        return rsx! {
            div { class: "px-2 py-1.5 text-ui-xs text-muted-foreground", {translate("tools-scanning")} }
        };
    }
    if !tools.error.is_empty() {
        return rsx! {
            div { class: "px-2 py-1.5 text-ui-xs text-destructive", "{tools.error}" }
        };
    }
    let categories = tools
        .categories
        .iter()
        .filter(|category| !category.items.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if categories.is_empty() {
        return rsx! {
            div { class: "px-2 py-1.5 text-ui-xs text-muted-foreground", {translate("tools-no-installed")} }
        };
    }
    rsx! {
        for category in categories {
            SmartToolCategoryRow { key: "{category.provider.id()}", category, pane_id }
        }
    }
}

fn tools_provider_title(provider: vmux_core::tools::ToolProvider) -> String {
    translate(match provider {
        vmux_core::tools::ToolProvider::HomebrewFormula => "tools-provider-homebrew-formulae",
        vmux_core::tools::ToolProvider::HomebrewCask => "tools-provider-homebrew-casks",
        vmux_core::tools::ToolProvider::Npm => "tools-provider-npm",
        vmux_core::tools::ToolProvider::Acp => "tools-provider-acp-agents",
        vmux_core::tools::ToolProvider::Lsp => "tools-provider-lsp-servers",
        vmux_core::tools::ToolProvider::Mcp => "tools-provider-mcp-servers",
        vmux_core::tools::ToolProvider::Dotfiles => "tools-provider-dotfiles",
    })
}

#[component]
fn SmartToolCategoryRow(category: ToolCategory, pane_id: u64) -> Element {
    let mut expanded = use_signal(|| false);
    let updates = category
        .items
        .iter()
        .filter(|item| item.status == ToolStatus::Outdated)
        .count();
    let conflicts = category
        .items
        .iter()
        .filter(|item| item.status == ToolStatus::Conflict)
        .count();
    rsx! {
        div { class: "flex flex-col gap-0.5",
            button {
                r#type: "button",
                class: "flex h-8 cursor-pointer items-center gap-1.5 rounded-md px-1.5 text-left text-muted-foreground hover:bg-glass-hover hover:text-foreground",
                onclick: move |_| expanded.set(!expanded()),
                Icon {
                    class: if expanded() { SIDEBAR_TREE_CHEVRON_OPEN } else { SIDEBAR_TREE_CHEVRON_CLOSED },
                    path { d: "m9 18 6-6-6-6" }
                }
                span { class: "min-w-0 flex-1 truncate text-ui font-medium", {tools_provider_title(category.provider)} }
                if updates > 0 {
                    span { class: "flex items-center gap-1 whitespace-nowrap text-[9px] text-muted-foreground",
                        span { class: "size-1 rounded-full bg-amber-500" }
                        "{updates}"
                    }
                }
                if conflicts > 0 {
                    span { class: "flex items-center gap-1 whitespace-nowrap text-[9px] text-muted-foreground",
                        span { class: "size-1 rounded-full bg-rose-500" }
                        "{conflicts}"
                    }
                }
                span { class: "text-[10px] tabular-nums text-muted-foreground/70", "{category.items.len()}" }
            }
            if expanded() {
                div { class: "ml-3 flex flex-col gap-0.5 border-l border-foreground/10 pl-1.5",
                    for item in category.items.iter() {
                        SmartToolItemRow { key: "{item.id}", item: item.clone(), pane_id }
                    }
                }
            }
        }
    }
}

#[component]
fn SmartToolItemRow(item: ToolItem, pane_id: u64) -> Element {
    let status_class = match item.status {
        ToolStatus::Installed => "bg-success",
        ToolStatus::Outdated => "bg-amber-400",
        ToolStatus::Conflict | ToolStatus::Failed => "bg-ansi-1",
        ToolStatus::Missing => "bg-muted-foreground/40",
        ToolStatus::Available => "bg-cyan-400/60",
    };
    rsx! {
        button {
            r#type: "button",
            title: tools_provider_title(item.provider),
            class: "flex h-8 cursor-pointer items-center gap-1.5 rounded-md px-1.5 pl-5 text-left text-muted-foreground hover:bg-glass-hover hover:text-foreground",
            onclick: move |_| open_tools(pane_id),
            span { class: "size-1.5 shrink-0 rounded-full {status_class}" }
            span { class: "min-w-0 flex-1 truncate text-ui", "{item.name}" }
            if item.managed {
                span { class: "text-[9px] text-cyan-300/80", {translate("tools-managed")} }
            }
            if let Some(version) = item.version.as_ref() {
                span { class: "max-w-20 truncate text-[9px] text-muted-foreground/60", "{version}" }
            }
        }
    }
}

fn begin_new_folder(mut creating: Signal<bool>, mut draft: Signal<String>) {
    draft.set(translate("layout-new-folder"));
    creating.set(true);
}

#[component]
fn BookmarkEntry(
    row: BookmarkRow,
    folder_uuid: Option<String>,
    folders: Vec<BookmarkFolderChoice>,
) -> Element {
    let drag_state: Signal<Option<BookmarkDragState>> = use_context();
    let url_open = row.metadata.url.clone();
    let uuid_pin = row.uuid.clone();
    let uuid_remove = row.uuid.clone();
    let uuid_rename = row.uuid.clone();
    let menu_val = use_signal(|| row.uuid.clone());
    let title = if row.metadata.title.is_empty() {
        row.metadata.url.clone()
    } else {
        row.metadata.title.clone()
    };
    let mut editing = use_signal(|| false);
    let draft = use_signal(|| title.clone());
    let bookmark_menu_action: Signal<BookmarkMenuActionEvent> = use_context();
    let initial_menu_action = bookmark_menu_action.peek().sequence;
    let mut handled_menu_action = use_signal(|| initial_menu_action);
    let menu_action_uuid = row.uuid.clone();
    let menu_action_name = title.clone();
    use_effect(move || {
        let action = bookmark_menu_action();
        if action.sequence == handled_menu_action() {
            return;
        }
        handled_menu_action.set(action.sequence);
        if action.action == "rename" && action.uuid.as_deref() == Some(menu_action_uuid.as_str()) {
            begin_inline_rename(editing, draft, menu_action_name.clone());
        }
    });
    let mut move_targets: Vec<(Option<String>, String)> = Vec::new();
    if folder_uuid.is_some() {
        move_targets.push((None, translate("layout-move-to-bookmarks")));
    }
    move_targets.extend(
        folders
            .iter()
            .filter(|folder| Some(folder.uuid.as_str()) != folder_uuid.as_deref())
            .map(|folder| {
                (
                    Some(folder.uuid.clone()),
                    translate_with(
                        "layout-move-to",
                        &[("folder", TranslationValue::String(&folder.label))],
                    ),
                )
            }),
    );
    let remove_index = 3 + move_targets.len();
    let drag_item = BookmarkDragItem::Bookmark {
        uuid: row.uuid.clone(),
    };
    rsx! {
        if editing() {
            div { class: "flex h-9 items-center gap-2 rounded-md border border-transparent px-2",
                PageIconView {
                    icon: row.metadata.icon.clone(),
                    url: row.metadata.url.clone(),
                    img_class: "h-4 w-4 shrink-0 rounded-sm object-contain".to_string(),
                    icon_class: "h-4 w-4 shrink-0 text-muted-foreground".to_string(),
                }
                BookmarkNameInput {
                    draft,
                    class: "min-w-0 flex-1 bg-transparent text-ui text-foreground outline-none".to_string(),
                    placeholder: String::new(),
                    on_commit: {
                        let id = uuid_rename.clone();
                        move |name| {
                            editing.set(false);
                            commit_bookmark_rename(id.clone(), name);
                        }
                    },
                    on_cancel: move |_| editing.set(false),
                }
            }
        } else {
            BookmarkContextMenu {
                command: "menu_bookmark".to_string(),
                uuid: Some(row.uuid.clone()),
                metadata: None,
                trigger: rsx! {
                    div {
                        "data-bookmark-drag-source": "true",
                        onpointerdown: {
                            let item = drag_item.clone();
                            move |event| begin_bookmark_drag(drag_state, &event, item.clone())
                        },
                        SidebarTreeRowGroup {
                            SidebarTreeRow {
                                path: row.metadata.url.clone(),
                                label: title.clone(),
                                is_dir: false,
                                title: title.clone(),
                                leading: rsx! {
                                    PageIconView {
                                        icon: row.metadata.icon.clone(),
                                        url: row.metadata.url.clone(),
                                        img_class: "h-3.5 w-3.5 shrink-0 rounded-sm object-contain".to_string(),
                                        icon_class: "h-3.5 w-3.5 shrink-0 text-muted-foreground".to_string(),
                                    }
                                },
                                on_activate: {
                                    let u = url_open.clone();
                                    move |()| {
                                        if bookmark_drag_blocks_click(drag_state) {
                                            return;
                                        }
                                        open_bookmark(u.clone());
                                    }
                                },
                            }
                        }
                    }
                },
                menu: rsx! {
                    SideSheetContextMenuContent {
                    ContextMenuItem {
                        index: 0usize,
                        value: Into::<ReadSignal<String>>::into(menu_val),
                        on_select: { let u = url_open.clone(); move |_: String| open_bookmark(u.clone()) },
                        attributes: vec![],
                        {translate("common-open")}
                    }
                    ContextMenuItem {
                        index: 1usize,
                        value: Into::<ReadSignal<String>>::into(menu_val),
                        on_select: {
                            let name = title.clone();
                            move |_: String| begin_inline_rename(editing, draft, name.clone())
                        },
                        attributes: vec![],
                        {translate("common-rename")}
                    }
                    ContextMenuItem {
                        index: 2usize,
                        value: Into::<ReadSignal<String>>::into(menu_val),
                        on_select: {
                            let id = uuid_pin.clone();
                            let command = if row.pinned { "unpin" } else { "pin" };
                            move |_: String| bookmark_cmd(command, Some(id.clone()))
                        },
                        attributes: vec![],
                        {if row.pinned { translate("layout-unpin-page") } else { translate("layout-pin") }}
                    }
                    for (index, (target_folder, label)) in move_targets.iter().enumerate() {
                        ContextMenuItem {
                            key: "{index}",
                            index: 3usize + index,
                            value: Into::<ReadSignal<String>>::into(menu_val),
                            on_select: {
                                let id = row.uuid.clone();
                                let folder = target_folder.clone();
                                move |_: String| move_bookmark(id.clone(), folder.clone())
                            },
                            attributes: vec![],
                            "{label}"
                        }
                    }
                    ContextMenuItem {
                        index: remove_index,
                        value: Into::<ReadSignal<String>>::into(menu_val),
                        on_select: { let id = uuid_remove.clone(); move |_: String| bookmark_cmd("remove", Some(id.clone())) },
                        attributes: vec![],
                        {translate("common-remove")}
                    }
                    }
                },
            }
        }
    }
}

fn request_bookmark_menu(command: &str, uuid: Option<String>, metadata: Option<PageMetadata>) {
    let _ = send(&BookmarksCommandEvent {
        command: command.to_string(),
        uuid,
        name: None,
        url: None,
        metadata,
        folder: None,
    });
}

fn commit_folder_rename(uuid: String, name: String) {
    let name = name.trim().to_string();
    let command = if name.is_empty() {
        "remove_folder"
    } else {
        "rename_folder"
    };
    let _ = send(&BookmarksCommandEvent {
        command: command.into(),
        uuid: Some(uuid),
        name: if name.is_empty() { None } else { Some(name) },
        url: None,
        metadata: None,
        folder: None,
    });
}

#[component]
fn LayoutContextMenu(children: Element) -> Element {
    rsx! {
        ContextMenu {
            attributes: vec![
                dioxus_elements::events::oncontextmenu(move |event: Event<MouseData>| {
                    event.stop_propagation();
                }),
            ],
            on_open_change: set_bookmark_context_menu_active,
            {children}
        }
    }
}

#[component]
fn BookmarkContextMenu(
    command: String,
    uuid: Option<String>,
    metadata: Option<PageMetadata>,
    trigger: Element,
    menu: Element,
) -> Element {
    #[cfg(target_os = "macos")]
    {
        let _ = menu;
        return rsx! {
            div {
                role: "button",
                aria_haspopup: "menu",
                user_select: "none",
                oncontextmenu: move |event: Event<MouseData>| {
                    event.prevent_default();
                    event.stop_propagation();
                    request_bookmark_menu(&command, uuid.clone(), metadata.clone());
                },
                {trigger}
            }
        };
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = command;
        let _ = uuid;
        let _ = metadata;
        rsx! {
            LayoutContextMenu {
                ContextMenuTrigger { attributes: vec![], {trigger} }
                {menu}
            }
        }
    }
}

#[component]
fn SideSheetStackRow(stack: StackNode, pane_id: u64) -> Element {
    let folder_context: Signal<Vec<BookmarkFolderChoice>> = use_context();
    let drag_state: Signal<Option<BookmarkDragState>> = use_context();
    let folders = folder_context();
    let is_active = stack.is_active;
    let stack_id = stack.id;
    let command = StackCommand::new(pane_id, stack_id);
    let mut hovered = use_signal(|| false);
    let menu_val = use_signal(|| stack.url.clone());
    let metadata = PageMetadata {
        title: stack.title.clone(),
        url: stack.url.clone(),
        icon: stack.icon.clone(),
        bg_color: stack.bg_color.clone(),
    };
    let drag_item = BookmarkDragItem::Page {
        metadata: metadata.clone(),
    };
    let bookmark_metadata = metadata.clone();
    let pin_metadata = metadata;
    let pin_index = 1 + folders.len();
    let display_title = StackTitle::of(&stack);
    let close_title = translate("layout-close-stack");

    let title_class = if is_active {
        format!(
            "min-w-0 flex-1 {} text-ui font-medium text-foreground",
            dir_truncate_class(&display_title)
        )
    } else {
        format!(
            "min-w-0 flex-1 {} text-ui",
            dir_truncate_class(&display_title)
        )
    };

    rsx! {
        LayoutContextMenu {
            ContextMenuTrigger {
                attributes: vec![],
                div {
                    "data-bookmark-drag-source": "true",
                    onpointerdown: {
                        let item = drag_item.clone();
                        move |event| begin_bookmark_drag(drag_state, &event, item.clone())
                    },
                    id: "sidesheet-stack-{pane_id}-{stack_id}",
                    class: if is_active {
                        "glass flex h-9 cursor-default items-center gap-2 rounded-md px-2"
                    } else {
                        "flex h-9 cursor-pointer items-center gap-2 rounded-md px-2 border border-transparent text-muted-foreground hover:bg-glass-hover hover:text-foreground"
                    },
                    onmouseenter: move |_| hovered.set(true),
                    onmouseleave: move |_| hovered.set(false),
                    onclick: move |event| {
                        if bookmark_drag_blocks_click(drag_state) {
                            event.prevent_default();
                            event.stop_propagation();
                            return;
                        }
                        command.activate();
                    },
                    StackIcon { icon: stack.icon.clone(), url: stack.url.clone(), title: stack.title.clone() }
                    span { class: "{title_class}", "{display_title}" }
                    button {
                        r#type: "button",
                        aria_label: "{close_title}",
                        title: "{close_title}",
                        class: if hovered() {
                            "ml-auto flex h-6 w-6 cursor-pointer shrink-0 items-center justify-center rounded-sm opacity-100 transition-opacity focus-visible:opacity-100 hover:bg-foreground/10"
                        } else {
                            "ml-auto flex h-6 w-6 cursor-pointer shrink-0 items-center justify-center rounded-sm opacity-0 transition-opacity focus-visible:opacity-100 hover:bg-foreground/10"
                        },
                        onmousedown: move |evt| {
                            evt.prevent_default();
                            evt.stop_propagation();
                        },
                        onpointerdown: move |evt| {
                            evt.prevent_default();
                            evt.stop_propagation();
                        },
                        onclick: move |evt| {
                            evt.prevent_default();
                            evt.stop_propagation();
                            command.close();
                        },
                        Icon { class: "h-3 w-3 pointer-events-none",
                            path { d: "M18 6 6 18" }
                            path { d: "m6 6 12 12" }
                        }
                    }
                }
            }
            SideSheetContextMenuContent {
                ContextMenuItem {
                    index: 0usize,
                    value: Into::<ReadSignal<String>>::into(menu_val),
                    on_select: move |_: String| add_to_bookmarks(
                        "add",
                        bookmark_metadata.clone(),
                        None,
                    ),
                    attributes: vec![],
                    {translate("layout-bookmark")}
                }
                for (index, folder) in folders.iter().enumerate() {
                    ContextMenuItem {
                        key: "{folder.uuid}",
                        index: 1usize + index,
                        value: Into::<ReadSignal<String>>::into(menu_val),
                        on_select: {
                            let metadata = PageMetadata {
                                title: stack.title.clone(),
                                url: stack.url.clone(),
                                icon: stack.icon.clone(),
                                bg_color: stack.bg_color.clone(),
                            };
                            let folder_uuid = folder.uuid.clone();
                            move |_: String| add_to_bookmarks(
                                "add",
                                metadata.clone(),
                                Some(folder_uuid.clone()),
                            )
                        },
                        attributes: vec![],
                    {translate_with(
                        "layout-bookmark-in",
                        &[("folder", TranslationValue::String(&folder.label))],
                    )}
                    }
                }
                ContextMenuItem {
                    index: pin_index,
                    value: Into::<ReadSignal<String>>::into(menu_val),
                    on_select: move |_: String| add_to_bookmarks(
                        "pin_url",
                        pin_metadata.clone(),
                        None,
                    ),
                    attributes: vec![],
                    {translate("layout-pin")}
                }
            }
        }
    }
}

#[component]
fn NewStackRow(pane_id: u64) -> Element {
    rsx! {
        SheetNewButton {
            label: translate("layout-new-stack"),
            icon: rsx! {
                Icon { class: "h-4 w-4 shrink-0",
                    path { d: "M12 5v14" }
                    path { d: "M5 12h14" }
                }
            },
            onclick: move |_| {
                let _ = send(&crate::event::SideSheetCommandEvent {
                    command: "new_stack".to_string(),
                    pane_id: pane_id.to_string(),
                    stack_id: 0,
                    line: 0,
                    path: String::new(),
                });
            },
        }
    }
}

#[component]
fn StackIcon(icon: PageIcon, url: String, title: String) -> Element {
    if title == "New Stack" && url.is_empty() {
        return rsx! {
            Icon { class: "h-4 w-4 shrink-0 text-muted-foreground",
                path { d: "M5 12h14" }
                path { d: "M12 5v14" }
            }
        };
    }
    rsx! {
        PageIconView {
            icon,
            url,
            img_class: "h-4 w-4 shrink-0 rounded-sm object-contain".to_string(),
            icon_class: "h-4 w-4 shrink-0 text-muted-foreground".to_string(),
        }
    }
}

#[component]
fn SideSheetContextMenuContent(children: Element) -> Element {
    rsx! {
        ContextMenuContent { attributes: vec![], {children} }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct StackCommand {
    pane_id: u64,
    stack_id: u64,
}

impl StackCommand {
    fn new(pane_id: u64, stack_id: u64) -> Self {
        Self { pane_id, stack_id }
    }

    fn activate(self) {
        self.dispatch("activate_stack");
    }

    fn close(self) {
        self.dispatch("close_stack");
    }

    fn dispatch(self, command: &str) {
        let _ = send(&crate::event::SideSheetCommandEvent {
            command: command.to_string(),
            pane_id: self.pane_id.to_string(),
            stack_id: self.stack_id,
            line: 0,
            path: String::new(),
        });
    }
}

struct StackTitle;

impl StackTitle {
    fn of(stack: &StackNode) -> String {
        let title = localized_stack_title(stack);
        if !title.trim().is_empty() {
            return title;
        }
        if let Some(host) = vmux_ui::favicon::host_for_favicon_fallback(&stack.url) {
            return host.to_string();
        }
        if stack.is_loading {
            return translate("layout-loading");
        }
        if stack.url.is_empty() {
            return translate("layout-new-stack");
        }
        stack.url.clone()
    }
}

fn localized_stack_title(stack: &StackNode) -> String {
    match stack.url.trim_end_matches('/') {
        "vmux://start" => translate("start-title"),
        "vmux://settings" => translate("settings-title"),
        _ if stack.url.is_empty() && stack.title == "New Stack" => translate("layout-new-stack"),
        _ => stack.title.clone(),
    }
}

fn download_pct(downloaded: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    (downloaded.saturating_mul(100) / total).min(100)
}

#[derive(Clone, PartialEq)]
enum UpdatePhase {
    Downloading {
        version: String,
        downloaded: u64,
        total: u64,
    },
    Installing {
        version: String,
    },
    Ready {
        version: String,
    },
}

#[component]
fn SheetNewButton(label: String, icon: Element, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "group flex h-9 cursor-pointer items-center gap-2 rounded-md px-2 border border-transparent text-left text-muted-foreground hover:bg-glass-hover hover:text-foreground",
            onclick: move |e| onclick.call(e),
            {icon}
            span { class: "min-w-0 flex-1 truncate text-ui font-medium", "{label}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_pct_clamps_and_handles_zero_total() {
        assert_eq!(download_pct(0, 0), 0);
        assert_eq!(download_pct(50, 100), 50);
        assert_eq!(download_pct(250, 100), 100);
    }

    fn state(header_open: bool, side_sheet_open: bool) -> LayoutStateEvent {
        LayoutStateEvent {
            header_open,
            side_sheet_open,
            ..Default::default()
        }
    }

    #[test]
    fn overlay_waits_for_layout_state() {
        assert!(!layout_overlay_ready(
            &state(false, false),
            false,
            true,
            true,
            true,
            true
        ));
    }

    #[test]
    fn overlay_waits_for_header_state_when_header_visible() {
        let visible = state(true, false);

        assert!(!layout_overlay_ready(
            &visible, true, false, true, true, true
        ));
        assert!(!layout_overlay_ready(
            &visible, true, true, false, true, true
        ));
        assert!(layout_overlay_ready(&visible, true, true, true, true, true));
    }

    #[test]
    fn overlay_waits_for_side_sheet_state_when_side_sheet_visible() {
        let visible = state(false, true);

        assert!(!layout_overlay_ready(
            &visible, true, true, true, false, true
        ));
        assert!(!layout_overlay_ready(
            &visible, true, true, true, true, false
        ));
        assert!(layout_overlay_ready(&visible, true, true, true, true, true));
    }

    #[test]
    fn overlay_can_be_ready_when_overlay_is_closed() {
        assert!(layout_overlay_ready(
            &state(false, false),
            true,
            false,
            false,
            false,
            false
        ));
    }
}
