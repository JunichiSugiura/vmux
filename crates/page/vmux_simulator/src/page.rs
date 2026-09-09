#![allow(non_snake_case)]

use crate::event::{
    HardwareButton, SIMULATOR_READY_EVENT, SimulatorClipboard, SimulatorClipboardAction,
    SimulatorKey, SimulatorReady, SimulatorTouch, SimulatorTouchPhase,
};
use crate::url::SimulatorRoute;
use dioxus::html::geometry::ClientPoint;
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use std::rc::Rc;
use vmux_ui::hooks::{send, use_event, use_theme};
use vmux_ui::i18n::translate;
use vmux_ui::platform::sleep_ms;

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
                Mirror { port: announced.port, device_name: announced.device_name.clone() }
            }
        }
    }
}

#[component]
fn Mirror(port: u16, device_name: String) -> Element {
    let mut press = use_signal(|| None::<PointerSession>);
    let mut image_size = use_signal(|| None::<(f64, f64)>);
    let mut home_progress = use_signal(|| 0.0f32);
    let mut surface = use_signal(|| None::<Rc<MountedData>>);
    let mut keyboard_open = use_signal(|| false);
    let progress = home_progress();
    let scale = 1.0 - progress * 0.12;
    let offset = -progress * 18.0;
    let radius = progress * 28.0;
    let transition = if press().is_some_and(PointerSession::is_home) {
        "none"
    } else {
        "transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1), border-radius 180ms ease-out"
    };
    let image_style = format!(
        "transform:translateY({offset:.2}px) scale({scale:.4});border-radius:{radius:.2}px;transition:{transition};"
    );

    rsx! {
        div {
            class: "relative flex h-full w-full items-center justify-center overflow-hidden bg-zinc-950/70 p-8 outline-none",
            tabindex: 0,
            onmounted: move |event: Event<MountedData>| {
                let target = event.data();
                surface.set(Some(target.clone()));
                spawn(async move {
                    if let Err(error) = target.set_focus(true).await {
                        dioxus::logger::tracing::warn!("focusing the simulator failed: {error:?}");
                    }
                });
            },
            onpointerdown: move |_| {
                if keyboard_open() {
                    return;
                }
                let Some(target) = surface.peek().clone() else {
                    return;
                };
                spawn(async move {
                    if let Err(error) = target.set_focus(true).await {
                        dioxus::logger::tracing::warn!("focusing the simulator failed: {error:?}");
                    }
                });
            },
            onkeydown: move |event| {
                if let Some(action) = ClipboardShortcut::of(&event) {
                    event.prevent_default();
                    let _ = send(&SimulatorClipboard { action });
                    return;
                }
                let Some(key) = Keystroke::of(&event) else {
                    return;
                };
                event.prevent_default();
                let _ = send(&key);
            },
            onpointermove: move |event: Event<PointerData>| {
                let Some(current) = press() else {
                    return;
                };
                event.prevent_default();
                if !event.held_buttons().contains(MouseButton::Primary) {
                    press.set(None);
                    current
                        .release_at(event.client_coordinates())
                        .dispatch(home_progress);
                    return;
                }
                let Some((next, touch)) = current.move_to(event.client_coordinates()) else {
                    return;
                };
                press.set(Some(next));
                home_progress.set(next.home_progress());
                if let Some(touch) = touch {
                    let _ = send(&touch);
                }
            },
            onpointerup: move |event: Event<PointerData>| {
                let Some(current) = press.take() else {
                    return;
                };
                event.prevent_default();
                current
                    .release_at(event.client_coordinates())
                    .dispatch(home_progress);
            },
            onpointercancel: move |_| {
                let Some(current) = press.take() else {
                    return;
                };
                current.cancel().dispatch(home_progress);
            },
            div { class: "pointer-events-none absolute left-5 top-4 text-sm font-medium text-zinc-300",
                "{device_name}"
            }
            button {
                r#type: "button",
                aria_label: translate("simulator-keyboard"),
                title: translate("simulator-keyboard"),
                class: "absolute right-5 top-3 z-20 flex h-8 w-8 items-center justify-center rounded-lg text-zinc-400 transition-colors hover:bg-white/10 hover:text-zinc-100 active:bg-white/15",
                onclick: move |_| keyboard_open.toggle(),
                svg {
                    class: "h-4 w-4",
                    view_box: "0 0 24 24",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.8",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    rect { x: "3", y: "5", width: "18", height: "14", rx: "2" }
                    path { d: "M7 9h.01M11 9h.01M15 9h.01M19 9h.01M7 13h.01M11 13h.01M15 13h.01M19 13h.01M8 17h8" }
                }
            }
            div { class: "relative rounded-[3.25rem] bg-gradient-to-b from-zinc-700 via-zinc-950 to-black p-[7px] shadow-[0_28px_80px_rgba(0,0,0,0.65)] ring-1 ring-white/20",
                div { class: "absolute -left-[3px] top-28 h-16 w-[3px] rounded-l bg-zinc-700" }
                div { class: "absolute -left-[3px] top-48 h-24 w-[3px] rounded-l bg-zinc-700" }
                div { class: "absolute -right-[3px] top-36 h-24 w-[3px] rounded-r bg-zinc-700" }
                div { class: "overflow-hidden rounded-[2.8rem] bg-black ring-1 ring-black",
                    img {
                        class: "block h-auto max-h-[calc(100vh-5rem)] max-w-[calc(100vw-5rem)] cursor-grab touch-none select-none active:cursor-grabbing",
                        style: image_style,
                        draggable: false,
                        src: "http://127.0.0.1:{port}/",
                        onresize: move |event: Event<ResizeData>| {
                            let Ok(size) = event.get_border_box_size() else {
                                return;
                            };
                            image_size.set(Some((size.width, size.height)));
                        },
                        onpointerdown: move |event: Event<PointerData>| {
                            if event
                                .trigger_button()
                                .is_some_and(|button| button != MouseButton::Primary)
                            {
                                return;
                            }
                            let Some(size) = image_size() else {
                                return;
                            };
                            event.prevent_default();
                            let Some((session, touch)) = PointerSession::start(&event, size) else {
                                return;
                            };
                            press.set(Some(session));
                            home_progress.set(0.0);
                            if let Some(touch) = touch {
                                let _ = send(&touch);
                            }
                        },
                    }
                }
            }
            if keyboard_open() {
                KeyboardCapture {
                    on_close: move |_| {
                        keyboard_open.set(false);
                        let Some(target) = surface.peek().clone() else {
                            return;
                        };
                        spawn(async move {
                            if let Err(error) = target.set_focus(true).await {
                                dioxus::logger::tracing::warn!("focusing the simulator failed: {error:?}");
                            }
                        });
                    }
                }
            }
        }
    }
}

#[component]
fn KeyboardCapture(on_close: EventHandler<()>) -> Element {
    let mut draft = use_signal(KeyboardDraft::default);
    let copy_title = format!("{} (⌘C)", translate("simulator-copy"));
    let paste_title = format!("{} (⌘V)", translate("simulator-paste"));

    rsx! {
        div {
            class: "absolute bottom-5 left-1/2 z-30 flex w-[calc(100%-2.5rem)] max-w-lg -translate-x-1/2 items-center gap-2 rounded-2xl border border-white/15 bg-zinc-900/95 p-2 shadow-2xl backdrop-blur-xl",
            onpointerdown: move |event| event.stop_propagation(),
            input {
                r#type: "text",
                autofocus: true,
                autocomplete: "off",
                autocapitalize: "none",
                spellcheck: false,
                value: "{draft().value()}",
                placeholder: translate("simulator-keyboard-placeholder"),
                aria_label: translate("simulator-keyboard-placeholder"),
                class: "h-9 min-w-0 flex-1 rounded-xl bg-white/10 px-3 text-sm text-zinc-100 outline-none placeholder:text-zinc-500 focus:ring-1 focus:ring-white/25",
                onmounted: move |event: Event<MountedData>| async move {
                    if let Err(error) = event.data().set_focus(true).await {
                        dioxus::logger::tracing::warn!("focusing simulator keyboard input failed: {error:?}");
                    }
                },
                oninput: move |event| {
                    let keys = draft.write().replace(event.value());
                    for key in keys {
                        let _ = send(&key);
                    }
                },
                onkeydown: move |event: Event<KeyboardData>| {
                    event.stop_propagation();
                    if let Some(action) = ClipboardShortcut::of(&event) {
                        event.prevent_default();
                        let _ = send(&SimulatorClipboard { action });
                        return;
                    }
                    match event.key() {
                        Key::Escape => {
                            event.prevent_default();
                            on_close.call(());
                        }
                        Key::Backspace if draft.peek().is_empty() => {
                            event.prevent_default();
                            let _ = send(&SimulatorKey::Code(42));
                        }
                        Key::Enter => {
                            event.prevent_default();
                            let _ = send(&SimulatorKey::Code(40));
                        }
                        Key::Tab => {
                            event.prevent_default();
                            let _ = send(&SimulatorKey::Code(43));
                        }
                        Key::ArrowRight => {
                            event.prevent_default();
                            let _ = send(&SimulatorKey::Code(79));
                        }
                        Key::ArrowLeft => {
                            event.prevent_default();
                            let _ = send(&SimulatorKey::Code(80));
                        }
                        Key::ArrowDown => {
                            event.prevent_default();
                            let _ = send(&SimulatorKey::Code(81));
                        }
                        Key::ArrowUp => {
                            event.prevent_default();
                            let _ = send(&SimulatorKey::Code(82));
                        }
                        _ => {}
                    }
                },
            }
            button {
                r#type: "button",
                title: "{copy_title}",
                class: "h-9 shrink-0 rounded-xl px-3 text-xs font-semibold text-zinc-300 transition-colors hover:bg-white/10 hover:text-white active:bg-white/15",
                onclick: move |_| {
                    let _ = send(&SimulatorClipboard { action: SimulatorClipboardAction::Copy });
                },
                {translate("simulator-copy")}
            }
            button {
                r#type: "button",
                title: "{paste_title}",
                class: "h-9 shrink-0 rounded-xl px-3 text-xs font-semibold text-zinc-300 transition-colors hover:bg-white/10 hover:text-white active:bg-white/15",
                onclick: move |_| {
                    let _ = send(&SimulatorClipboard { action: SimulatorClipboardAction::Paste });
                },
                {translate("simulator-paste")}
            }
            button {
                r#type: "button",
                class: "h-9 shrink-0 rounded-xl px-3 text-xs font-semibold text-zinc-300 transition-colors hover:bg-white/10 hover:text-white active:bg-white/15",
                onclick: move |_| on_close.call(()),
                {translate("common-done")}
            }
        }
    }
}

struct ClipboardShortcut;

impl ClipboardShortcut {
    fn of(event: &Event<KeyboardData>) -> Option<SimulatorClipboardAction> {
        let modifiers = event.modifiers();
        if !modifiers.meta() || modifiers.ctrl() || modifiers.alt() || modifiers.shift() {
            return None;
        }
        match event.key().to_string().to_ascii_lowercase().as_str() {
            "c" => Some(SimulatorClipboardAction::Copy),
            "v" => Some(SimulatorClipboardAction::Paste),
            _ => None,
        }
    }
}

#[derive(Clone, Default)]
struct KeyboardDraft(String);

impl KeyboardDraft {
    fn value(&self) -> &str {
        &self.0
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn replace(&mut self, next: String) -> Vec<SimulatorKey> {
        let previous = std::mem::replace(&mut self.0, next);
        if let Some(added) = self.0.strip_prefix(&previous) {
            if added.is_empty() {
                return Vec::new();
            }
            return vec![SimulatorKey::Text(added.to_string())];
        }
        if let Some(removed) = previous.strip_prefix(&self.0) {
            return removed.chars().map(|_| SimulatorKey::Code(42)).collect();
        }
        let mut keys = Vec::with_capacity(previous.chars().count() + 1);
        keys.extend(previous.chars().map(|_| SimulatorKey::Code(42)));
        if !self.0.is_empty() {
            keys.push(SimulatorKey::Text(self.0.clone()));
        }
        keys
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
        Some(SimulatorRoute::Pinned {
            version,
            device_name: Some(device_name),
        }) => format!("{device_name} · iOS {version}"),
        Some(SimulatorRoute::Pinned { version, .. }) => format!("iOS {version}"),
        _ => translate("common-loading"),
    };
    rsx! {
        div { class: "text-sm text-muted-foreground", "{label}" }
    }
}

#[derive(Clone, Copy)]
struct PointerSession {
    origin: (f64, f64),
    size: (f64, f64),
    start: (f32, f32),
    last: (f32, f32),
    home: bool,
}

impl PointerSession {
    fn start(
        event: &Event<PointerData>,
        size: (f64, f64),
    ) -> Option<(Self, Option<SimulatorTouch>)> {
        let client = event.client_coordinates();
        let local = event.element_coordinates();
        let origin = (client.x - local.x, client.y - local.y);
        let point = Self::fraction((client.x, client.y), origin, size)?;
        let home = point.1 >= 0.94;
        let session = Self {
            origin,
            size,
            start: point,
            last: point,
            home,
        };
        let touch = (!home).then(|| session.touch(SimulatorTouchPhase::Down));
        Some((session, touch))
    }

    fn move_to(mut self, point: ClientPoint) -> Option<(Self, Option<SimulatorTouch>)> {
        let point = Self::fraction((point.x, point.y), self.origin, self.size)?;
        if point == self.last {
            return None;
        }
        self.last = point;
        let touch = (!self.home).then(|| self.touch(SimulatorTouchPhase::Move));
        Some((self, touch))
    }

    fn release_at(mut self, point: ClientPoint) -> PointerRelease {
        if let Some(point) = Self::fraction((point.x, point.y), self.origin, self.size) {
            self.last = point;
        }
        if !self.home {
            return PointerRelease::Touch(self.touch(SimulatorTouchPhase::Up));
        }
        if self.completes_home() {
            PointerRelease::Home
        } else {
            PointerRelease::None
        }
    }

    fn cancel(self) -> PointerRelease {
        if self.home {
            PointerRelease::None
        } else {
            PointerRelease::Touch(self.touch(SimulatorTouchPhase::Cancel))
        }
    }

    fn touch(self, phase: SimulatorTouchPhase) -> SimulatorTouch {
        SimulatorTouch {
            phase,
            x: self.last.0,
            y: self.last.1,
        }
    }

    fn is_home(self) -> bool {
        self.home
    }

    fn home_progress(self) -> f32 {
        if !self.home {
            return 0.0;
        }
        ((self.start.1 - self.last.1) / 0.35).clamp(0.0, 1.0)
    }

    fn completes_home(self) -> bool {
        let dx = self.last.0 - self.start.0;
        let dy = self.last.1 - self.start.1;
        dy < -0.1 && dy.abs() > dx.abs()
    }

    fn fraction(point: (f64, f64), origin: (f64, f64), size: (f64, f64)) -> Option<(f32, f32)> {
        if size.0 <= 0.0 || size.1 <= 0.0 {
            return None;
        }
        Some((
            ((point.0 - origin.0) / size.0).clamp(0.0, 1.0) as f32,
            ((point.1 - origin.1) / size.1).clamp(0.0, 1.0) as f32,
        ))
    }
}

enum PointerRelease {
    Touch(SimulatorTouch),
    Home,
    None,
}

impl PointerRelease {
    fn dispatch(self, mut home_progress: Signal<f32>) {
        match self {
            Self::Touch(touch) => {
                home_progress.set(0.0);
                let _ = send(&touch);
            }
            Self::Home => {
                home_progress.set(1.0);
                let _ = send(&SimulatorKey::Button(HardwareButton::Home));
                spawn(async move {
                    sleep_ms(300).await;
                    home_progress.set(0.0);
                });
            }
            Self::None => home_progress.set(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_capture_forwards_insertions_deletions_and_replacements() {
        let mut draft = KeyboardDraft::default();

        assert_eq!(
            draft.replace("pair".to_string()),
            vec![SimulatorKey::Text("pair".to_string())]
        );
        assert_eq!(
            draft.replace("pai".to_string()),
            vec![SimulatorKey::Code(42)]
        );
        assert_eq!(
            draft.replace("link".to_string()),
            vec![
                SimulatorKey::Code(42),
                SimulatorKey::Code(42),
                SimulatorKey::Code(42),
                SimulatorKey::Text("link".to_string()),
            ]
        );
    }
}
