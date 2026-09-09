#![allow(non_snake_case)]

use crate::{CheatsheetEvent, EVENT, ShortcutGroup};
use dioxus::prelude::*;
use vmux_ui::hooks::{use_listener, use_theme};
use vmux_ui::i18n::{TranslationValue, translate, translate_with};

#[component]
pub fn Page() -> Element {
    use_theme();
    let mut state = use_signal(CheatsheetEvent::default);
    let mut query = use_signal(String::new);
    let _listener = use_listener::<CheatsheetEvent, _>(EVENT, move |event| state.set(event));

    let needle = query().trim().to_lowercase();
    let groups = state
        .read()
        .groups
        .iter()
        .filter_map(|group| group.filtered(&needle))
        .collect::<Vec<_>>();
    let count = groups
        .iter()
        .map(|group| group.entries.len())
        .sum::<usize>();
    let subtitle = translate_with(
        "cheatsheet-count",
        &[("count", TranslationValue::Number(count as i64))],
    );

    rsx! {
        main { class: "flex h-full min-h-0 flex-col bg-background text-foreground",
            header { class: "shrink-0 border-b border-border px-5 py-4",
                div { class: "mx-auto flex w-full max-w-4xl items-center gap-4",
                    div { class: "min-w-0",
                        h1 { class: "text-lg font-semibold tracking-tight", {translate("cheatsheet-title")} }
                        p { class: "mt-0.5 text-xs text-muted-foreground", "{subtitle}" }
                    }
                    div { class: "flex-1" }
                    input {
                        r#type: "search",
                        class: "w-56 rounded-xl bg-foreground/[0.04] px-3 py-2 text-sm text-foreground outline-none ring-1 ring-inset ring-foreground/10 transition-colors placeholder:text-muted-foreground/60 focus:bg-foreground/[0.06] focus:ring-cyan-400/30",
                        placeholder: translate("cheatsheet-search"),
                        value: "{query}",
                        oninput: move |event| query.set(event.value()),
                    }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-5 py-5",
                div { class: "mx-auto grid w-full max-w-4xl grid-cols-1 gap-3 lg:grid-cols-2",
                    if groups.is_empty() {
                        div { class: "col-span-full flex min-h-72 items-center justify-center text-sm text-muted-foreground",
                            {translate("cheatsheet-empty")}
                        }
                    } else {
                        for group in groups {
                            ShortcutGroupView { key: "{group.name}", group }
                        }
                    }
                }
            }
        }
    }
}

impl ShortcutGroup {
    fn filtered(&self, needle: &str) -> Option<Self> {
        if needle.is_empty() {
            return Some(self.clone());
        }
        let group_matches = self.name.to_lowercase().contains(needle);
        let entries = self
            .entries
            .iter()
            .filter(|entry| {
                group_matches
                    || entry.name.to_lowercase().contains(needle)
                    || entry
                        .shortcuts
                        .iter()
                        .any(|shortcut| shortcut.to_lowercase().contains(needle))
            })
            .cloned()
            .collect::<Vec<_>>();
        (!entries.is_empty()).then(|| Self {
            name: self.name.clone(),
            entries,
        })
    }
}

#[component]
fn ShortcutGroupView(group: ShortcutGroup) -> Element {
    rsx! {
        section { class: "glass overflow-hidden rounded-2xl border border-border/70",
            h2 { class: "border-b border-border/60 px-4 py-3 text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground",
                "{group.name}"
            }
            div { class: "divide-y divide-border/50",
                for entry in group.entries {
                    div { class: "flex min-h-11 items-center gap-4 px-4 py-2.5",
                        span { class: "min-w-0 flex-1 text-sm text-foreground", "{entry.name}" }
                        div { class: "flex shrink-0 flex-wrap justify-end gap-1.5",
                            for shortcut in entry.shortcuts {
                                kbd { class: "rounded-md bg-foreground/[0.06] px-2 py-1 font-mono text-[11px] font-medium text-foreground ring-1 ring-inset ring-foreground/10",
                                    "{shortcut}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
