use crate::event::ChatCreateWorktree;
use crate::page::state::Chat;
use dioxus::prelude::*;
use vmux_ui::components::composer::{PROMPT_INPUT_ID, focus_prompt_end};
use vmux_ui::components::composer_bar::WorkspaceBadges as SharedWorkspaceBadges;
use vmux_ui::hooks::send;

#[component]
pub(super) fn WorkspaceBadges(chat: Chat) -> Element {
    let context = (chat.slash.composer_context)();
    let offers_worktree = !context.is_worktree && context.can_manage_workspace;
    rsx! {
        SharedWorkspaceBadges {
            is_git_repo: context.is_git_repo,
            workspace_known: context.workspace_selected,
            uncommitted: context.uncommitted,
            ahead: context.ahead,
            on_create_worktree: offers_worktree
                .then(|| EventHandler::new(move |()| {
                    let _ = send(&ChatCreateWorktree);
                    focus_prompt_end(PROMPT_INPUT_ID);
                })),
        }
    }
}
