#![allow(non_snake_case)]

use dioxus::prelude::*;
use vmux_command::event::CommandBarOpenEvent;
use vmux_ui::components::start_hero::{START_BACKDROP_STYLE, StartBackdrop, StartHero};
use vmux_ui::hooks::{send, use_event, use_listener, use_theme};
use vmux_ui::i18n::translate;

use crate::event::{
    START_COMMAND_BAR_OPEN_EVENT, START_FOCUS_INPUT_EVENT, StartDataRequest, StartFocusInput,
};
use vmux_command::page::{CommandPalette, StartInlineTransition, focus_prompt_input};
use vmux_ui::agent_accent::agent_accent;
use vmux_ui::favicon::Favicon;
use vmux_ui::launcher::palette::{AgentSegment, PaletteSurface};
use vmux_ui::matrix_rain::MatrixRain;

#[component]
pub fn Page(
    #[props(default)] on_inline_transition: Option<EventHandler<StartInlineTransition>>,
) -> Element {
    let locale = use_theme();
    let state = use_event::<CommandBarOpenEvent>(
        START_COMMAND_BAR_OPEN_EVENT,
        CommandBarOpenEvent::default,
    );
    let mut mounted = use_signal(|| false);

    let _focus_listener = use_listener::<StartFocusInput, _>(START_FOCUS_INPUT_EVENT, move |_| {
        focus_prompt_input();
    });

    use_effect(move || {
        locale();
        let _ = send(&StartDataRequest);
    });

    use_effect(move || {
        focus_prompt_input();
        mounted.set(true);
    });

    rsx! {
        main {
            class: "relative isolate flex min-h-0 flex-1 flex-col overflow-y-auto overscroll-contain bg-background px-4 py-6 text-foreground sm:px-6",
            style: START_BACKDROP_STYLE,
            StartBackdrop {}
            div { class: "m-auto w-full",
                StartHero { revealed: mounted(),
                    div { class: "relative w-full",
                        CommandPalette {
                            state,
                            surface: PaletteSurface::Start,
                            on_close: move |_| {},
                            on_dismiss: move |_| {},
                            on_activity: move |_| {},
                            on_start_inline_transition: on_inline_transition,
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn StartPage() -> Element {
    let mut transition = use_signal(|| None::<String>);
    if let Some(target_url) = transition() {
        return rsx! {
            PreparingAgent { target_url }
        };
    }

    rsx! {
        Page {
            on_inline_transition: move |requested: StartInlineTransition| {
                transition.set(Some(requested.target_url));
            },
        }
    }
}

#[component]
fn PreparingAgent(target_url: String) -> Element {
    use_theme();
    let agent = PreparingAgentIdentity::of(&target_url);
    let accent = agent_accent(&agent.segment);
    let accent_rgb = accent.rain_rgb.to_string();
    let page_style = format!("--agent-accent:rgb({accent_rgb});");
    let rain_word = agent.name.to_uppercase();
    let preparing = translate("agent-preparing");

    rsx! {
        main {
            class: "relative isolate flex h-dvh flex-col overflow-hidden bg-background text-foreground",
            style: "{page_style}",
            div { class: "pointer-events-none absolute inset-0 overflow-hidden",
                MatrixRain {
                    accent_rgb,
                    words: vec![rain_word],
                }
            }
            header { class: "relative z-10 flex min-w-0 items-center gap-2.5 border-b border-foreground/10 bg-background/95 px-5 py-3",
                Favicon {
                    favicon_url: String::new(),
                    url: target_url.clone(),
                    class: Some("h-6 w-6 rounded-full object-contain".to_string()),
                    globe_class: Some("h-6 w-6 text-muted-foreground".to_string()),
                }
                span { class: "h-2.5 w-2.5 rounded-full bg-sky-400" }
                div { class: "min-w-0 flex-1",
                    div { class: "truncate text-sm font-semibold", "{agent.name}" }
                    div { class: "truncate text-[10px] text-muted-foreground/60", "{agent.segment}" }
                }
            }
            div { class: "relative z-10 flex min-h-0 flex-1 items-center justify-center",
                div { class: "flex flex-col items-center gap-2",
                    div { class: "flex h-14 w-14 items-center justify-center overflow-hidden rounded-full bg-background/70 ring-1 ring-foreground/10 backdrop-blur-md",
                        Favicon {
                            favicon_url: String::new(),
                            url: target_url,
                            class: Some("h-full w-full object-cover".to_string()),
                            globe_class: Some("h-7 w-7 text-muted-foreground".to_string()),
                        }
                    }
                    h2 { class: "text-3xl font-semibold capitalize tracking-tight", "{agent.name}" }
                    div { class: "rounded-full bg-background/70 px-3 py-1 text-xs text-muted-foreground ring-1 ring-foreground/10 backdrop-blur-md",
                        "{preparing}"
                    }
                }
            }
            div { class: "relative z-10 bg-gradient-to-t from-background via-background/95 to-transparent px-4 pb-4 pt-8",
                div { class: "mx-auto h-16 max-w-3xl rounded-2xl bg-foreground/[0.08] ring-1 ring-inset ring-foreground/10 backdrop-blur-md" }
            }
        }
    }
}

struct PreparingAgentIdentity {
    segment: String,
    name: String,
}

impl PreparingAgentIdentity {
    fn of(url: &str) -> Self {
        let segment = AgentSegment::in_url(url).unwrap_or_else(|| "agent".to_string());
        let mut words = Vec::new();
        for part in segment.split('-').filter(|part| !part.is_empty()) {
            let mut chars = part.chars();
            let Some(first) = chars.next() else {
                continue;
            };
            words.push(first.to_uppercase().chain(chars).collect::<String>());
        }
        let name = words.join(" ");
        Self { segment, name }
    }
}
