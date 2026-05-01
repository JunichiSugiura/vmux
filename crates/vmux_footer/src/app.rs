#![allow(non_snake_case)]

use dioxus::prelude::*;
use vmux_footer::event::{FooterCommandEvent, SPACES_EVENT, SpaceRow, SpacesHostEvent};
use vmux_footer::style::{space_close_button_class, space_pill_class};
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
        div { class: "flex h-full min-h-0 min-w-0 flex-1 items-center gap-1 rounded-lg px-1.5 text-foreground",
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
    let pill_class = space_pill_class(space.is_active);
    let id_switch = space.id.clone();
    let id_close = space.id.clone();
    let name = space.name.clone();
    let is_active = space.is_active;
    let close_class = space_close_button_class(is_active);
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
                span { class: if is_active { "font-mono text-sidebar-primary-foreground" } else { "font-mono text-muted-foreground" }, "{index}" }
                span { class: "min-w-0 truncate", "{name}" }
            }
            button {
                r#type: "button",
                aria_label: "Close space",
                title: "Close space",
                class: close_class,
                onclick: move |evt| {
                    evt.stop_propagation();
                    let _ = try_cef_emit_serde(&FooterCommandEvent {
                        command: "close".to_string(),
                        space_id: Some(id_close.clone()),
                    });
                },
                Icon { class: "h-2.5 w-2.5",
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
            class: "flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-glass-hover hover:text-foreground active:bg-glass-active active:text-foreground",
            onclick: move |_| {
                let _ = try_cef_emit_serde(&FooterCommandEvent {
                    command: "new".to_string(),
                    space_id: None,
                });
            },
            Icon { class: "h-3.5 w-3.5",
                path { d: "M12 5v14" }
                path { d: "M5 12h14" }
            }
        }
    }
}
