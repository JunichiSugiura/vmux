#![allow(non_snake_case)]

use dioxus::prelude::*;
use vmux_footer::event::{FooterCommandEvent, SPACES_EVENT, SpaceRow, SpacesHostEvent};
use vmux_ui::components::icon::Icon;
use vmux_ui::hooks::{try_cef_emit_serde, use_event_listener, use_theme};

#[component]
pub fn App() -> Element {
    use_theme();
    let mut spaces_state = use_signal(SpacesHostEvent::default);
    let listener = use_event_listener::<SpacesHostEvent, _>(SPACES_EVENT, move |data| {
        spaces_state.set(data);
    });

    let SpacesHostEvent { spaces } = spaces_state();

    rsx! {
        div { class: "flex h-full min-h-0 min-w-0 flex-1 items-center gap-1 rounded-lg px-2 text-foreground",
            if (listener.is_loading)() {
                span { class: "text-ui text-muted-foreground", "Connecting…" }
            } else if let Some(err) = (listener.error)() {
                span { class: "text-ui text-destructive", "{err}" }
            } else {
                div { class: "flex min-w-0 flex-1 items-center gap-1 overflow-x-auto",
                    for (idx, space) in spaces.iter().enumerate() {
                        SpacePill {
                            key: "{space.id}",
                            index: idx + 1,
                            space: space.clone(),
                        }
                    }
                    NewSpaceButton {}
                }
            }
        }
    }
}

#[component]
fn SpacePill(index: usize, space: SpaceRow) -> Element {
    let pill_class = if space.is_active {
        "group glass flex h-7 items-center gap-1.5 rounded-full pl-3 pr-1 text-xs text-foreground shadow-sm"
    } else {
        "group flex h-7 items-center gap-1.5 rounded-full pl-3 pr-1 text-xs text-muted-foreground hover:bg-glass-hover hover:text-foreground"
    };
    let id_switch = space.id.clone();
    let id_close = space.id.clone();
    let name = space.name.clone();
    let is_active = space.is_active;
    // Reserve fixed slot (h-5 w-5) on every pill so adjacent pills never
    // shift; close button only becomes visible/interactive on hover of the
    // active pill.
    let close_class = if is_active {
        "flex h-5 w-5 cursor-pointer items-center justify-center rounded-full text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:bg-glass-hover hover:text-foreground"
    } else {
        "flex h-5 w-5 items-center justify-center rounded-full pointer-events-none invisible"
    };
    rsx! {
        div { class: pill_class,
            button {
                r#type: "button",
                title: "{name}",
                class: "flex min-w-0 cursor-pointer items-center gap-2",
                onclick: move |_| {
                    let _ = try_cef_emit_serde(&FooterCommandEvent {
                        command: "switch".to_string(),
                        space_id: Some(id_switch.clone()),
                    });
                },
                span { class: "font-mono text-muted-foreground", "{index}" }
                span { class: "min-w-0 truncate", "{name}" }
            }
            button {
                r#type: "button",
                aria_label: "Close space",
                title: "Close space",
                class: close_class,
                tabindex: if is_active { "0" } else { "-1" },
                onclick: move |evt| {
                    evt.stop_propagation();
                    if is_active {
                        let _ = try_cef_emit_serde(&FooterCommandEvent {
                            command: "close".to_string(),
                            space_id: Some(id_close.clone()),
                        });
                    }
                },
                Icon { class: "h-3 w-3",
                    path { d: "M18 6 6 18" }
                    path { d: "m6 6 12 12" }
                }
            }
        }
    }
}

#[component]
fn NewSpaceButton() -> Element {
    rsx! {
        button {
            r#type: "button",
            aria_label: "New space",
            title: "New space",
            class: "flex h-7 w-7 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-glass-hover hover:text-foreground active:bg-glass-active active:text-foreground",
            onclick: move |_| {
                let _ = try_cef_emit_serde(&FooterCommandEvent {
                    command: "new".to_string(),
                    space_id: None,
                });
            },
            Icon { class: "h-4 w-4",
                path { d: "M12 5v14" }
                path { d: "M5 12h14" }
            }
        }
    }
}
