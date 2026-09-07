use crate::util::cn;
use dioxus::prelude::*;

const AVATAR: &str = "inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-[var(--avatar-background)] font-semibold text-white";

#[component]
pub fn Avatar(
    src: Option<String>,
    fallback: String,
    background: String,
    #[props(default)] alt: String,
    #[props(default)] class: String,
) -> Element {
    let class = cn([AVATAR, class.as_str()]);
    rsx! {
        div {
            class,
            style: "--avatar-background:{background}",
            if let Some(src) = src {
                img { class: "size-full object-cover", src, alt }
            } else {
                "{fallback}"
            }
        }
    }
}
