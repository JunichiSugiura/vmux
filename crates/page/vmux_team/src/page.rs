#![allow(non_snake_case)]

use dioxus::prelude::*;
use vmux_ui::components::avatar::Avatar;
use vmux_ui::components::badge::Badge;
use vmux_ui::components::inline_edit::{EditableText, InlineEdit};
use vmux_ui::favicon::favicon_src_for_url;
use vmux_ui::hooks::{send, use_event, use_theme};
use vmux_ui::i18n::{TranslationValue, translate, translate_with};
use vmux_wire::team::{ProfileRow, TEAM_EVENT, TeamCommandEvent, TeamEvent, TeamMemberRow};

#[component]
pub fn Page() -> Element {
    use_theme();
    let team = use_event::<TeamEvent>(TEAM_EVENT, TeamEvent::default);

    let snapshot = team();
    let profiles = snapshot.profiles;
    let members = snapshot.members;
    let agents: Vec<TeamMemberRow> = members.iter().filter(|m| !m.is_user).cloned().collect();

    rsx! {
        div {
            class: "flex h-full min-h-0 flex-col bg-background text-foreground",
            header { class: "flex items-center justify-between border-b border-border px-5 py-4",
                div { class: "min-w-0",
                    h1 { class: "text-lg font-semibold tracking-tight", {translate("team-profiles")} }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-5 py-5",
                div { class: "mx-auto w-full max-w-4xl",
                    ProfileSection { profiles }
                    if !agents.is_empty() {
                        section {
                            div { class: "mb-2.5 px-0.5 text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground", {translate("team-agent")} }
                            div { class: "flex flex-col gap-0.5",
                                for agent in agents.iter() {
                                    AgentRow { key: "{agent.id}", member: agent.clone() }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ProfileSection(profiles: Vec<ProfileRow>) -> Element {
    let mut creating = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let profile_count = profiles.len();

    rsx! {
        section { class: "mb-6",
            div { class: "mb-2.5 flex items-center justify-between px-0.5",
                div { class: "text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground", {translate("team-profiles")} }
            }
            div { class: "grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3",
                for profile in profiles.iter() {
                    ProfileRowView {
                        key: "profile-{profile.id}",
                        profile: profile.clone(),
                    }
                }
                if creating() {
                    div { key: "create-{profile_count}", class: "glass flex min-h-24 items-center rounded-xl border border-border/70 p-2",
                        InlineEdit {
                            draft,
                            caret_at_end: true,
                            class: "m-1 min-w-0 flex-1 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground outline-none focus:border-primary/50".to_string(),
                            placeholder: translate("team-profile-name"),
                            aria_label: translate("team-profile-name"),
                            restore_focus_id: "new-profile".to_string(),
                            on_commit: move |name| {
                                creating.set(false);
                                emit_profile_command("create_profile", None, Some(name));
                            },
                            on_cancel: move |_| creating.set(false),
                        }
                    }
                } else {
                    button {
                        id: "new-profile",
                        r#type: "button",
                        class: "flex min-h-24 items-center justify-center gap-2 rounded-xl border border-dashed border-border text-sm font-medium text-muted-foreground transition-colors hover:border-foreground/25 hover:bg-foreground/[0.035] hover:text-foreground",
                        onclick: move |_| {
                            draft.set(String::new());
                            creating.set(true);
                        },
                        svg { class: "size-4", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                            path { d: "M12 5v14M5 12h14" }
                        }
                        {translate("team-new-profile")}
                    }
                }
            }
        }
    }
}

#[component]
fn ProfileRowView(profile: ProfileRow) -> Element {
    let switch_id = profile.id.clone();
    let avatar_color = vmux_wire::avatar::hash_color(&profile.id);
    let editing = use_signal(|| false);
    let draft = use_signal(|| profile.name.clone());
    let title = if profile.is_active {
        profile.name.clone()
    } else {
        translate_with(
            "team-switch-profile",
            &[("profile", TranslationValue::String(&profile.name))],
        )
    };
    rsx! {
        div { class: if profile.is_active {
                "glass group relative min-h-24 overflow-hidden rounded-xl border border-primary/25 bg-primary/[0.06] ring-1 ring-inset ring-primary/10"
            } else {
                "glass group relative min-h-24 cursor-pointer overflow-hidden rounded-xl border border-border/70 transition-colors hover:bg-glass-hover"
            },
            if !profile.is_active {
                button {
                    r#type: "button",
                    class: if editing() {
                        "pointer-events-none absolute inset-0 z-0 rounded-xl outline-none"
                    } else {
                        "absolute inset-0 z-0 rounded-xl outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-primary/60"
                    },
                    title: title.clone(),
                    aria_label: title,
                    disabled: editing(),
                    onclick: move |_| {
                        emit_profile_command("switch_profile", Some(switch_id.clone()), None);
                    },
                }
            }
            div { class: "pointer-events-none relative z-10 flex h-full min-h-24 w-full min-w-0 items-center gap-3 p-3 text-left",
                Avatar {
                    src: None,
                    seed: profile.id.clone(),
                    background: avatar_color,
                    alt: profile.name.clone(),
                    class: "size-10",
                }
                div { class: "flex min-w-0 flex-1 flex-col gap-1",
                    EditableText {
                        value: profile.name.clone(),
                        editing,
                        draft,
                        display_class: "pointer-events-auto min-w-0 cursor-text truncate rounded px-1 py-0.5 text-left text-sm font-semibold text-foreground hover:bg-foreground/[0.06]".to_string(),
                        input_class: "pointer-events-auto min-w-0 w-full rounded-md bg-background/70 px-2 py-1 text-sm font-semibold text-foreground outline-none ring-1 ring-inset ring-primary/40".to_string(),
                        title: translate("team-edit-profile"),
                        placeholder: translate("team-profile-name"),
                        on_commit: {
                            let id = profile.id.clone();
                            move |name| emit_profile_command("update_profile", Some(id.clone()), Some(name))
                        },
                    }
                    span { class: "truncate font-mono text-[10px] text-muted-foreground", "{profile.id}" }
                    if profile.is_active {
                        Badge { class: "w-fit rounded-full bg-primary/15 px-2 py-0.5 text-[10px] font-medium text-primary",
                            {translate("common-active")}
                        }
                    }
                }
            }
        }
    }
}

fn emit_profile_command(command: &str, profile_id: Option<String>, profile_name: Option<String>) {
    let _ = send(&TeamCommandEvent {
        command: command.to_string(),
        member_id: None,
        profile_id,
        profile_name,
    });
}

#[component]
fn AgentRow(member: TeamMemberRow) -> Element {
    let default_title = format!("{} (", member.name);
    let subtitle = if !member.title.is_empty()
        && member.title != member.name
        && !member.title.starts_with(&default_title)
    {
        Some(member.title.clone())
    } else if member.sid.is_empty() {
        Some(translate("team-agent"))
    } else {
        None
    };

    rsx! {
        div {
            class: "flex items-start gap-3 rounded-lg px-2 py-2 hover:bg-foreground/[0.04]",
            AgentAvatar { member: member.clone() }
            div { class: "flex min-w-0 flex-1 flex-col gap-0.5 pt-0.5",
                div { class: "flex min-w-0 items-center gap-2",
                    span { class: "truncate text-sm font-semibold text-foreground", "{member.name}" }
                    if member.is_running {
                        Badge { class: "gap-1.5 rounded-full bg-success/15 px-2 py-0.5 text-[11px] font-medium text-success",
                            span { class: "size-1.5 rounded-full bg-success animate-pulse" }
                            {translate("common-running")}
                        }
                    } else if member.is_done_unseen {
                        Badge { class: "gap-1.5 rounded-full bg-amber-500/15 px-2 py-0.5 text-[11px] font-medium text-amber-600 dark:text-amber-400",
                            span { class: "size-1.5 rounded-full bg-amber-400 animate-pulse" }
                            {translate("common-done")}
                        }
                    }
                }
                if let Some(subtitle) = subtitle {
                    span { class: "truncate text-xs text-muted-foreground", "{subtitle}" }
                }
                if !member.sid.is_empty() {
                    span { class: "truncate font-mono text-[11px] text-muted-foreground/50", "{member.sid}" }
                }
            }
        }
    }
}

#[component]
fn AgentAvatar(member: TeamMemberRow) -> Element {
    let src = favicon_src_for_url(&member.icon, &member.url);

    rsx! {
        div { class: "relative shrink-0",
            Avatar {
                src,
                seed: member.name.clone(),
                background: member.color.clone(),
                alt: member.name.clone(),
                class: "size-8 text-sm",
            }
            if member.is_running {
                span { class: "absolute -bottom-0.5 -right-0.5 size-3 rounded-full bg-success ring-2 ring-background animate-pulse" }
            } else if member.is_done_unseen {
                span { class: "absolute -bottom-0.5 -right-0.5 size-3 rounded-full bg-amber-400 ring-2 ring-background animate-pulse" }
            }
        }
    }
}
