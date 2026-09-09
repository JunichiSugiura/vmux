use crate::focus::FocusClaim;
use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_INLINE_EDIT_ID: AtomicUsize = AtomicUsize::new(0);

#[component]
pub fn InlineEdit(
    draft: Signal<String>,
    class: String,
    #[props(default)] placeholder: String,
    #[props(default = true)] caret_at_end: bool,
    #[props(default)] on_active_change: Option<EventHandler<bool>>,
    on_commit: EventHandler<String>,
    on_cancel: EventHandler<()>,
) -> Element {
    let mut draft = draft;
    let mut finished = use_signal(|| false);
    let id = use_hook(|| {
        format!(
            "vmux-inline-edit-{}",
            NEXT_INLINE_EDIT_ID.fetch_add(1, Ordering::Relaxed)
        )
    });
    let dropped = on_active_change;
    use_drop(move || {
        if let Some(callback) = dropped {
            callback.call(false);
        }
    });

    rsx! {
        input {
            id: "{id}",
            r#type: "text",
            class,
            placeholder,
            value: "{draft}",
            autofocus: true,
            onclick: move |event: Event<MouseData>| event.stop_propagation(),
            onpointerdown: move |event: Event<PointerData>| event.stop_propagation(),
            oncontextmenu: move |event: Event<MouseData>| {
                event.prevent_default();
                event.stop_propagation();
            },
            onmounted: move |event: Event<MountedData>| {
                if let Some(callback) = on_active_change {
                    callback.call(true);
                }
                let id = id.clone();
                spawn(async move {
                    let _ = event.data().set_focus(true).await;
                    if caret_at_end {
                        FocusClaim::new(id).caret_at_end().request();
                    }
                });
            },
            oninput: move |event| draft.set(event.value()),
            onkeydown: move |event: Event<KeyboardData>| match event.key() {
                Key::Enter => {
                    event.prevent_default();
                    event.stop_propagation();
                    let name = draft().trim().to_string();
                    if !finished() {
                        finished.set(true);
                        if name.is_empty() {
                            on_cancel.call(());
                        } else {
                            on_commit.call(name);
                        }
                    }
                }
                Key::Escape => {
                    event.prevent_default();
                    event.stop_propagation();
                    if !finished() {
                        finished.set(true);
                        on_cancel.call(());
                    }
                }
                _ => {}
            },
            onblur: move |_| {
                if finished() {
                    return;
                }
                finished.set(true);
                let name = draft().trim().to_string();
                if name.is_empty() {
                    on_cancel.call(());
                } else {
                    on_commit.call(name);
                }
            },
        }
    }
}
