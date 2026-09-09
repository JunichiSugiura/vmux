#![allow(non_snake_case)]

use dioxus::prelude::*;
use vmux_ui::components::avatar::Avatar;
use vmux_ui::components::badge::Badge;
use vmux_ui::components::inline_edit::InlineEdit;
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
    let count = members.len();
    let user = members.iter().find(|m| m.is_user).cloned();
    let agents: Vec<TeamMemberRow> = members.iter().filter(|m| !m.is_user).cloned().collect();
    let agent_count = agents.len();
    let subtitle = match agent_count {
        0 => translate("team-just-you"),
        count => translate_with(
            "team-agents",
            &[("count", TranslationValue::Number(count as i64))],
        ),
    };

    rsx! {
        div {
            class: "flex h-full min-h-0 flex-col bg-background text-foreground",
            header { class: "flex items-center justify-between border-b border-border px-5 py-4",
                div { class: "min-w-0",
                    h1 { class: "text-lg font-semibold tracking-tight", {translate("team-title")} }
                    p { class: "mt-0.5 truncate text-xs text-muted-foreground", "{subtitle}" }
                }
                if count > 0 {
                    Badge { class: "rounded-full border border-border bg-card px-2.5 py-1 text-xs font-medium text-muted-foreground",
                        "{count}"
                    }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-5 py-5",
                div { class: "mx-auto w-full max-w-4xl",
                    ProfileSection { profiles }
                    if members.is_empty() {
                        div { class: "flex min-h-72 flex-col items-center justify-center gap-2 text-muted-foreground",
                            div { class: "flex size-12 items-center justify-center rounded-full border border-dashed border-border",
                                svg {
                                    class: "size-5",
                                    view_box: "0 0 24 24",
                                    fill: "none",
                                    stroke: "currentColor",
                                    stroke_width: "1.5",
                                    path { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" }
                                    circle { cx: "9", cy: "7", r: "4" }
                                    path { d: "M22 21v-2a4 4 0 0 0-3-3.87" }
                                }
                            }
                            span { class: "text-sm", {translate("team-empty")} }
                        }
                    } else {
                        div { class: "flex flex-col gap-0.5",
                            if let Some(user) = user.clone() {
                                TeamRow { member: user }
                            }
                            if !agents.is_empty() {
                                div { class: "ml-6 flex flex-col gap-0.5 border-l border-border/60 pl-3",
                                    for agent in agents.iter() {
                                        TeamRow { key: "{agent.id}", member: agent.clone() }
                                    }
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
    let mut editing = use_signal(|| None::<String>);
    let mut draft = use_signal(String::new);
    let profile_count = profiles.len();

    rsx! {
        section { class: "mb-6",
            div { class: "mb-2.5 flex items-center justify-between px-0.5",
                div { class: "text-xs font-semibold uppercase tracking-[0.14em] text-muted-foreground", {translate("team-profiles")} }
            }
            div { class: "grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3",
                for profile in profiles.iter() {
                    if editing().as_deref() == Some(profile.id.as_str()) {
                        div { key: "edit-{profile.id}", class: "glass flex min-h-24 items-center rounded-xl border border-border/70 p-2",
                            InlineEdit {
                                draft,
                                caret_at_end: true,
                                class: "m-1 min-w-0 flex-1 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground outline-none focus:border-primary/50".to_string(),
                                placeholder: translate("team-profile-name"),
                                on_commit: {
                                    let id = profile.id.clone();
                                    move |name| {
                                        editing.set(None);
                                        emit_profile_command("update_profile", Some(id.clone()), Some(name));
                                    }
                                },
                                on_cancel: move |_| editing.set(None),
                            }
                        }
                    } else {
                        ProfileRowView {
                            key: "profile-{profile.id}",
                            profile: profile.clone(),
                            on_edit: {
                                let id = profile.id.clone();
                                let name = profile.name.clone();
                                move |_| {
                                    draft.set(name.clone());
                                    editing.set(Some(id.clone()));
                                }
                            },
                        }
                    }
                }
                if creating() {
                    div { key: "create-{profile_count}", class: "glass flex min-h-24 items-center rounded-xl border border-border/70 p-2",
                        InlineEdit {
                            draft,
                            caret_at_end: true,
                            class: "m-1 min-w-0 flex-1 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground outline-none focus:border-primary/50".to_string(),
                            placeholder: translate("team-profile-name"),
                            on_commit: move |name| {
                                creating.set(false);
                                emit_profile_command("create_profile", None, Some(name));
                            },
                            on_cancel: move |_| creating.set(false),
                        }
                    }
                } else {
                    button {
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
fn ProfileRowView(profile: ProfileRow, on_edit: EventHandler<()>) -> Element {
    let switch_id = profile.id.clone();
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
                "glass group relative min-h-24 overflow-hidden rounded-xl border border-blue-400/25 bg-blue-400/[0.06] ring-1 ring-inset ring-blue-400/10"
            } else {
                "glass group relative min-h-24 overflow-hidden rounded-xl border border-border/70 transition-colors hover:bg-glass-hover"
            },
            button {
                r#type: "button",
                disabled: profile.is_active,
                title,
                class: "flex h-full min-h-24 w-full min-w-0 items-center gap-3 p-3 pr-10 text-left disabled:cursor-default",
                onclick: move |_| emit_profile_command("switch_profile", Some(switch_id.clone()), None),
                div { class: if profile.is_active {
                        "flex size-10 shrink-0 items-center justify-center rounded-full bg-blue-500/15 text-sm font-semibold text-blue-600 dark:text-blue-300"
                    } else {
                        "flex size-10 shrink-0 items-center justify-center rounded-full bg-foreground/[0.07] text-sm font-semibold text-foreground"
                    },
                    {profile.name.chars().next().unwrap_or('P').to_uppercase().to_string()}
                }
                div { class: "flex min-w-0 flex-1 flex-col gap-1",
                    span { class: "truncate text-sm font-semibold text-foreground", "{profile.name}" }
                    span { class: "truncate font-mono text-[10px] text-muted-foreground", "{profile.id}" }
                    if profile.is_active {
                        Badge { class: "w-fit rounded-full bg-blue-500/15 px-2 py-0.5 text-[10px] font-medium text-blue-600 dark:text-blue-300",
                            {translate("common-active")}
                        }
                    }
                }
            }
            button {
                r#type: "button",
                aria_label: translate("team-edit-profile"),
                title: translate("team-edit-profile"),
                class: "absolute right-2 top-2 flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 hover:bg-foreground/[0.08] hover:text-foreground",
                onclick: move |_| on_edit.call(()),
                svg { class: "size-3.5", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round",
                    path { d: "M12 20h9" }
                    path { d: "M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z" }
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
fn TeamRow(member: TeamMemberRow) -> Element {
    let default_title = format!("{} (", member.name);
    let subtitle = if member.is_user {
        Some(translate("team-you"))
    } else if !member.title.is_empty()
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
            TeamAvatar { member: member.clone() }
            div { class: "flex min-w-0 flex-1 flex-col gap-0.5 pt-0.5",
                div { class: "flex min-w-0 items-center gap-2",
                    span {
                        class: if member.is_user {
                            "text-sm font-semibold text-foreground"
                        } else {
                            "truncate text-sm font-semibold text-foreground"
                        },
                        "{member.name}"
                    }
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
                if !member.is_user && !member.sid.is_empty() {
                    span { class: "truncate font-mono text-[11px] text-muted-foreground/50", "{member.sid}" }
                }
            }
        }
    }
}

#[component]
fn TeamAvatar(member: TeamMemberRow) -> Element {
    let src = favicon_src_for_url(&member.icon, &member.url);

    rsx! {
        div { class: "relative shrink-0",
            Avatar {
                src,
                fallback: member.initials.clone(),
                background: member.color.clone(),
                alt: member.name.clone(),
                class: "size-8 text-sm",
                seed: member.is_user.then(|| member.name.clone()),
            }
            if member.is_running {
                span { class: "absolute -bottom-0.5 -right-0.5 size-3 rounded-full bg-success ring-2 ring-background animate-pulse" }
            } else if member.is_done_unseen {
                span { class: "absolute -bottom-0.5 -right-0.5 size-3 rounded-full bg-amber-400 ring-2 ring-background animate-pulse" }
            }
        }
    }
}
