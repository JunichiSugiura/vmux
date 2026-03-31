//! Vmux-only tmux-style dividers (not from upstream gallery).

use super::util::merge_class;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum UiDividerVariant {
    /// Inline `|` with tmux-dim styling.
    #[default]
    VerticalBar,
    /// Flexing horizontal rule with gradient (e.g. next to a section label).
    HorizontalFade,
}

#[component]
pub fn UiDivider(
    #[props(default)] class: Option<String>,
    #[props(default)] variant: UiDividerVariant,
) -> Element {
    match variant {
        UiDividerVariant::VerticalBar => {
            let c = merge_class(
                "inline-flex shrink-0 items-center font-bold text-tmux-dim",
                class.as_deref(),
            );
            rsx! {
                span { class: "{c}", "|" }
            }
        }
        UiDividerVariant::HorizontalFade => {
            let c = merge_class(
                "h-px min-w-0 flex-1 bg-gradient-to-r from-white/12 to-transparent",
                class.as_deref(),
            );
            rsx! {
                span { class: "{c}" }
            }
        }
    }
}
