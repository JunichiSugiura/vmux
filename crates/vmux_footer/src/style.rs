pub fn space_pill_class(is_active: bool) -> &'static str {
    if is_active {
        "group flex h-6 items-center gap-1 rounded-full bg-sidebar-primary pl-2.5 pr-1 text-ui-xs text-sidebar-primary-foreground shadow-sm"
    } else {
        "group flex h-6 items-center gap-1 rounded-full pl-2.5 pr-1 text-ui-xs text-muted-foreground hover:bg-glass-hover hover:text-foreground"
    }
}

pub fn space_close_button_class(is_active: bool) -> &'static str {
    if is_active {
        "flex h-4 w-4 cursor-pointer items-center justify-center rounded-full text-sidebar-primary-foreground opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 hover:bg-white/20"
    } else {
        "flex h-4 w-4 cursor-pointer items-center justify-center rounded-full text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100 hover:bg-glass-hover hover:text-foreground"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_space_pill_uses_continuous_blue_selector() {
        let class = space_pill_class(true);

        assert!(class.contains("bg-sidebar-primary"));
        assert!(class.contains("text-sidebar-primary-foreground"));
        assert!(!class.contains("glass"));
    }

    #[test]
    fn space_pills_fit_compact_footer() {
        assert!(space_pill_class(true).contains("h-6"));
        assert!(space_pill_class(false).contains("h-6"));
    }

    #[test]
    fn inactive_space_close_button_is_visible_on_hover() {
        let class = space_close_button_class(false);

        assert!(class.contains("group-hover:opacity-100"));
        assert!(!class.contains("invisible"));
        assert!(!class.contains("pointer-events-none"));
    }
}
