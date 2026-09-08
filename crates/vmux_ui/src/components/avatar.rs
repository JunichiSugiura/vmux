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
    #[props(default)] seed: Option<String>,
) -> Element {
    let class = cn([AVATAR, class.as_str()]);
    let identicon = seed
        .as_deref()
        .filter(|seed| !seed.is_empty())
        .map(Identicon::of);
    rsx! {
        div {
            class,
            style: "--avatar-background:{background}",
            if let Some(src) = src {
                img { class: "size-full object-cover", src, alt }
            } else if let Some(identicon) = identicon {
                svg {
                    class: "size-full",
                    view_box: "0 0 5 5",
                    role: "img",
                    shape_rendering: "crispEdges",
                    title { "{alt}" }
                    rect { width: "5", height: "5", fill: "var(--avatar-background)" }
                    for (x, y) in identicon.cells {
                        rect {
                            x: "{x}",
                            y: "{y}",
                            width: "1",
                            height: "1",
                            fill: "rgba(255,255,255,0.82)",
                        }
                    }
                }
            } else {
                "{fallback}"
            }
        }
    }
}

struct Identicon {
    cells: Vec<(u8, u8)>,
}

impl Identicon {
    fn of(seed: &str) -> Self {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in seed.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let mut cells = Vec::new();
        for y in 0..5u8 {
            for x in 0..3u8 {
                hash ^= u64::from(x + y * 3);
                hash = hash.wrapping_mul(0x100000001b3);
                if hash & 1 == 0 {
                    continue;
                }
                cells.push((x, y));
                if x != 2 {
                    cells.push((4 - x, y));
                }
            }
        }
        Self { cells }
    }
}

#[cfg(test)]
mod tests {
    use super::Identicon;

    #[test]
    fn generated_avatar_is_horizontally_symmetric() {
        let identicon = Identicon::of("Personal");

        for (x, y) in &identicon.cells {
            assert!(identicon.cells.contains(&(4 - x, *y)));
        }
    }

    #[test]
    fn different_profiles_get_different_patterns() {
        assert_ne!(Identicon::of("Personal").cells, Identicon::of("Work").cells);
    }
}
