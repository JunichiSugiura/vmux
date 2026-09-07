#![allow(non_snake_case)]

use crate::event::{
    HardwareButton, SIMULATOR_READY_EVENT, SimulatorGesture, SimulatorKey, SimulatorReady,
};
use crate::url::SimulatorRoute;
use dioxus::prelude::*;
use std::rc::Rc;
use vmux_ui::hooks::{send, use_event, use_theme};
use vmux_ui::i18n::translate;

#[component]
pub fn Page() -> Element {
    use_theme();
    let ready = use_event::<SimulatorReady>(SIMULATOR_READY_EVENT, SimulatorReady::default);
    let route = try_consume_context::<vmux_core::PageMetadata>()
        .and_then(|metadata| SimulatorRoute::of_url(&metadata.url));

    let announced = ready();
    rsx! {
        div { class: "flex h-screen w-screen items-center justify-center overflow-hidden bg-background",
            if announced.port == 0 {
                Waiting { route }
            } else {
                Mirror { port: announced.port }
            }
        }
    }
}

#[component]
fn Mirror(port: u16) -> Element {
    let mut press = use_signal(|| None::<(f64, f64)>);
    let mut image = use_signal(|| None::<Rc<MountedData>>);

    rsx! {
        div {
            class: "flex h-full w-full items-center justify-center overflow-hidden bg-zinc-950/70 p-8 outline-none",
            tabindex: 0,
            onkeydown: move |event| {
                let Some(key) = Keystroke::of(&event) else {
                    return;
                };
                event.prevent_default();
                let _ = send(&key);
            },
            div { class: "relative rounded-[3.25rem] bg-gradient-to-b from-zinc-700 via-zinc-950 to-black p-[7px] shadow-[0_28px_80px_rgba(0,0,0,0.65)] ring-1 ring-white/20",
                div { class: "absolute -left-[3px] top-28 h-16 w-[3px] rounded-l bg-zinc-700" }
                div { class: "absolute -left-[3px] top-48 h-24 w-[3px] rounded-l bg-zinc-700" }
                div { class: "absolute -right-[3px] top-36 h-24 w-[3px] rounded-r bg-zinc-700" }
                div { class: "overflow-hidden rounded-[2.8rem] bg-black ring-1 ring-black",
                    img {
                        class: "block h-auto max-h-[calc(100vh-5rem)] max-w-[calc(100vw-5rem)] select-none",
                        draggable: false,
                        src: "http://127.0.0.1:{port}/",
                        onmounted: move |event: Event<MountedData>| image.set(Some(event.data())),
                        onmousedown: move |event: Event<MouseData>| {
                            let point = event.element_coordinates();
                            press.set(Some((point.x, point.y)));
                        },
                        onmouseup: move |event| {
                            let Some(from) = press.take() else {
                                return;
                            };
                            let Some(element) = image() else {
                                return;
                            };
                            let point = event.element_coordinates();
                            spawn(async move {
                                let Ok(rect) = element.get_client_rect().await else {
                                    return;
                                };
                                let Some(from) = Pointer::fraction(from, (rect.size.width, rect.size.height)) else {
                                    return;
                                };
                                let Some(to) = Pointer::fraction((point.x, point.y), (rect.size.width, rect.size.height)) else {
                                    return;
                                };
                                let _ = send(&SimulatorGesture {
                                    from_x: from.0,
                                    from_y: from.1,
                                    to_x: to.0,
                                    to_y: to.1,
                                });
                            });
                        },
                        onmouseleave: move |_| {
                            press.set(None);
                        },
                    }
                }
            }
        }
    }
}

struct Keystroke;

impl Keystroke {
    fn of(event: &Event<KeyboardData>) -> Option<SimulatorKey> {
        let modifiers = event.modifiers();
        let key = event.key().to_string();
        if modifiers.meta() && !modifiers.ctrl() && !modifiers.alt() {
            return match key.to_ascii_lowercase().as_str() {
                "h" => Some(SimulatorKey::Button(HardwareButton::Home)),
                "l" => Some(SimulatorKey::Button(HardwareButton::Lock)),
                "s" => Some(SimulatorKey::Button(HardwareButton::Siri)),
                _ => None,
            };
        }
        if modifiers.meta() || modifiers.ctrl() || modifiers.alt() {
            return None;
        }
        SimulatorKey::of_browser_key(&key)
    }
}

#[component]
fn Waiting(route: Option<SimulatorRoute>) -> Element {
    let label = match route {
        Some(SimulatorRoute::Pinned(version)) => format!("iOS {version}"),
        _ => translate("common-loading"),
    };
    rsx! {
        div { class: "text-sm text-muted-foreground", "{label}" }
    }
}

struct Pointer;

impl Pointer {
    fn fraction(point: (f64, f64), size: (f64, f64)) -> Option<(f32, f32)> {
        if size.0 <= 0.0 || size.1 <= 0.0 {
            return None;
        }
        Some(((point.0 / size.0) as f32, (point.1 / size.1) as f32))
    }
}
