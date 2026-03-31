//! Full scrollable gallery: one demo per widget under [`vmux_ui::webview::components`].

use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus::signals::ReadSignal;
use dioxus_primitives::checkbox::CheckboxState;
use time::{Date, UtcDateTime};
use vmux_ui::webview::components::{
    accordion::{Accordion, AccordionContent, AccordionItem, AccordionTrigger},
    alert_dialog::{
        AlertDialogAction, AlertDialogActions, AlertDialogCancel, AlertDialogContent,
        AlertDialogDescription, AlertDialogRoot, AlertDialogTitle,
    },
    aspect_ratio::AspectRatio,
    avatar::{Avatar, AvatarFallback, AvatarImage},
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonVariant},
    calendar::{
        Calendar, CalendarGrid, CalendarHeader, CalendarMonthTitle, CalendarNavigation,
        CalendarNextMonthButton, CalendarPreviousMonthButton, CalendarView,
    },
    card::{Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle},
    checkbox::Checkbox,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    context_menu::{ContextMenu, ContextMenuContent, ContextMenuItem, ContextMenuTrigger},
    date_picker::{DatePicker, DatePickerInput},
    dialog::{DialogContent, DialogDescription, DialogRoot, DialogTitle},
    drag_and_drop_list::DragAndDropList,
    dropdown_menu::{
        DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
    },
    hover_card::{HoverCard, HoverCardContent, HoverCardTrigger},
    icon::{Icon, ViewBox},
    input::Input,
    label::Label,
    menubar::{Menubar, MenubarContent, MenubarItem, MenubarMenu, MenubarTrigger},
    pagination::{
        Pagination, PaginationContent, PaginationEllipsis, PaginationItem, PaginationLink,
        PaginationNext, PaginationPrevious,
    },
    popover::{PopoverContent, PopoverRoot, PopoverTrigger},
    progress::{Progress, ProgressIndicator},
    radio_group::{RadioGroup, RadioItem},
    scroll_area::ScrollArea,
    select::{
        Select, SelectGroup, SelectItemIndicator, SelectList, SelectOption, SelectTrigger, SelectValue,
    },
    separator::Separator,
    sheet::{Sheet, SheetContent, SheetDescription, SheetHeader, SheetSide, SheetTitle},
    sidebar::{
        Sidebar, SidebarContent, SidebarInset, SidebarMenu, SidebarMenuButton, SidebarMenuItem,
        SidebarProvider, SidebarTrigger,
    },
    skeleton::Skeleton,
    slider::{Slider, SliderRange, SliderThumb, SliderTrack},
    switch::{Switch, SwitchThumb},
    tabs::{TabContent, TabList, TabTrigger, Tabs, TabsVariant},
    textarea::Textarea,
    toast::ToastProvider,
    toggle::Toggle,
    toggle_group::{ToggleGroup, ToggleItem},
    toolbar::{Toolbar, ToolbarButton, ToolbarGroup, ToolbarSeparator},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
    virtual_list::VirtualList,
    UiDivider, UiDividerVariant, UiInputShell, UiPanel, UiRow, UiStack, UiText, UiTextSize,
    UiTextTone,
};

use super::layout::{DEMO, MAIN, SECTION, SECTION_TITLE};

#[component]
pub fn GalleryDemos() -> Element {
    let dlg_id = use_signal(|| None::<String>);
    let mut dlg_open = use_signal(|| Some(false));
    let alert_id = use_signal(|| None::<String>);
    let mut alert_open = use_signal(|| Some(false));
    let mut alert_close_cancel = alert_open.clone();
    let mut alert_close_ok = alert_open.clone();
    let sheet_id = use_signal(|| None::<String>);
    let mut sheet_open = use_signal(|| Some(false));

    let mut tabs_val = use_signal(|| Some("t1".to_string()));
    let tabs_disabled = use_signal(|| false);
    let tabs_horizontal = use_signal(|| true);

    let mut selected_date = use_signal(|| None::<Date>);
    let mut view_date = use_signal(|| UtcDateTime::now().date());
    let mut picker_date = use_signal(|| None::<Date>);

    let mut radio_sel = use_signal(|| Some("r1".to_string()));
    let mut select_val = use_signal(|| Some(Some("apple".to_string())));
    let mut toggle_pressed = use_signal(|| Some(HashSet::from([0usize])));
    let tabs_roving = use_signal(|| true);

    let dlg_modal = use_signal(|| true);
    let opt_apple = use_signal(|| "apple".to_string());
    let opt_pear = use_signal(|| "pear".to_string());
    let vl_count = use_signal(|| 50usize);
    let vl_buffer = use_signal(|| 5usize);

    let dd_menu_a = use_signal(|| "a".to_string());
    let ctx_menu_x = use_signal(|| "x".to_string());

    rsx! {
        main { class: "{MAIN}",
            p { class: "text-[13px] text-white/55",
                "Scroll to browse every vendored DioxusLabs widget plus vmux chrome (UiStack, UiRow, …)."
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "UiText & UiDivider" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    UiStack { class: "gap-2",
                        UiText { tone: UiTextTone::Muted, "Muted" }
                        UiText { tone: UiTextTone::Accent, "Accent" }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "UiRow & UiPanel & UiInputShell" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    UiRow { class: "flex-wrap items-center gap-2",
                        UiText { "left" }
                        UiDivider { variant: UiDividerVariant::VerticalBar }
                        UiPanel {
                            aria_label: Some("Demo".to_string()),
                            UiText { size: UiTextSize::Xs, "Panel" }
                        }
                    }
                    Label { html_for: "gal-in", class: "sr-only", "Demo" }
                    UiInputShell {
                        leading: rsx! {
                            Icon { view_box: ViewBox::new(0, 0, 24, 24), stroke_width: 2., class: "h-[14px] w-[14px]",
                                circle { cx: 11, cy: 11, r: 8 }
                                path { d: "m21 21-4.3-4.3" }
                            }
                        },
                        input: rsx! {
                            input { id: "gal-in", class: "w-full rounded-lg border border-white/[0.08] bg-white/[0.04] py-2 pl-9 pr-3 text-[13px] outline-none",
                                r#type: "text", placeholder: "Search…" }
                        },
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Button & Badge" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    UiRow { class: "flex-wrap gap-2",
                        Button { variant: ButtonVariant::Primary, "Primary" }
                        Button { variant: ButtonVariant::Secondary, "Secondary" }
                        Badge { variant: BadgeVariant::Primary, "Badge" }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Avatar & AspectRatio" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    UiRow { class: "items-center gap-4",
                        Avatar {
                            AvatarImage { src: "", alt: "" }
                            AvatarFallback { "VX" }
                        }
                        AspectRatio { ratio: 16.0 / 9.0,
                            div { class: "flex h-full w-full items-center justify-center bg-white/10", "16:9" }
                        }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Card" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Card {
                        CardHeader {
                            CardTitle { "Card title" }
                            CardDescription { "Description" }
                        }
                        CardContent { p { "Card body" } }
                        CardFooter { Button { variant: ButtonVariant::Primary, "Action" } }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Input & Textarea" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO} space-y-2",
                    Input { attributes: vec![], placeholder: None::<String>, children: rsx! {} }
                    Textarea { attributes: vec![], placeholder: None::<String>, children: rsx! {} }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Checkbox & Switch & Radio" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    UiStack { class: "gap-3",
                        Checkbox {
                            default_checked: CheckboxState::Unchecked,
                            on_checked_change: Callback::new(|_| {}),
                            attributes: vec![], children: rsx! { "Check" }
                        }
                        Switch {
                            attributes: vec![], children: rsx! { SwitchThumb {} }
                        }
                        RadioGroup {
                            value: radio_sel(),
                            on_value_change: Callback::new(move |v| radio_sel.set(Some(v))),
                            attributes: vec![],
                            RadioItem { value: "r1", index: 0usize, attributes: vec![], "One" }
                            RadioItem { value: "r2", index: 1usize, attributes: vec![], "Two" }
                        }
                        Toggle { attributes: vec![], children: rsx! { "Toggle" } }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "ToggleGroup" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    ToggleGroup {
                        pressed: Into::<ReadSignal<Option<HashSet<usize>>>>::into(toggle_pressed),
                        on_pressed_change: Callback::new(move |s| toggle_pressed.set(Some(s))),
                        attributes: vec![],
                        ToggleItem { index: 0usize, attributes: vec![], "A" }
                        ToggleItem { index: 1usize, attributes: vec![], "B" }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Slider & Progress" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Slider { attributes: vec![], SliderTrack {}, SliderRange {}, SliderThumb {} }
                    Progress { attributes: vec![], value: Some(0.4), max: 1.0, ProgressIndicator {} }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Separator & Skeleton" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Separator { decorative: true, horizontal: false, attributes: vec![], children: rsx! {} }
                    Skeleton { height: Some("3rem"), width: Some("100%"), attributes: vec![] }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "ScrollArea" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    ScrollArea { attributes: vec![],
                        for i in 0..12 { p { key: "{i}", "Line {i}" } }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Tabs" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Tabs {
                        value: Into::<ReadSignal<Option<String>>>::into(tabs_val),
                        default_value: "t1".to_string(),
                        on_value_change: Callback::new(move |s| tabs_val.set(Some(s))),
                        disabled: Into::<ReadSignal<bool>>::into(tabs_disabled),
                        horizontal: Into::<ReadSignal<bool>>::into(tabs_horizontal),
                        roving_loop: Into::<ReadSignal<bool>>::into(tabs_roving),
                        variant: TabsVariant::Default,
                        attributes: vec![],
                        TabList { attributes: vec![],
                            TabTrigger { value: "t1", index: 0usize, attributes: vec![], "One" }
                            TabTrigger { value: "t2", index: 1usize, attributes: vec![], "Two" }
                        }
                        TabContent { value: "t1", index: 0usize, attributes: vec![], class: None, "Tab panel 1" }
                        TabContent { value: "t2", index: 1usize, attributes: vec![], class: None, "Tab panel 2" }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Accordion & Collapsible" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Accordion { attributes: vec![],
                        AccordionItem { index: 0usize, attributes: vec![],
                            AccordionTrigger { attributes: vec![], "Item A" }
                            AccordionContent { attributes: vec![], "Content A" }
                        }
                    }
                    Collapsible { attributes: vec![],
                        CollapsibleTrigger { attributes: vec![], "More" }
                        CollapsibleContent { attributes: vec![], "Hidden text" }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Dialog, AlertDialog, Sheet" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    UiRow { class: "flex-wrap gap-2",
                        Button { variant: ButtonVariant::Outline, onclick: move |_| dlg_open.set(Some(true)), "Dialog" }
                        Button { variant: ButtonVariant::Outline, onclick: move |_| alert_open.set(Some(true)), "Alert" }
                        Button { variant: ButtonVariant::Outline, onclick: move |_| sheet_open.set(Some(true)), "Sheet" }
                    }
                    DialogRoot {
                        id: Into::<ReadSignal<Option<String>>>::into(dlg_id),
                        open: Into::<ReadSignal<Option<bool>>>::into(dlg_open),
                        on_open_change: Callback::new(move |o| dlg_open.set(Some(o))),
                        default_open: false,
                        is_modal: Into::<ReadSignal<bool>>::into(dlg_modal),
                        attributes: vec![],
                        DialogContent { attributes: vec![],
                            DialogTitle { attributes: vec![], "Dialog" }
                            DialogDescription { attributes: vec![], "Body" }
                            Button { variant: ButtonVariant::Primary, onclick: move |_| dlg_open.set(Some(false)), "Close" }
                        }
                    }
                    AlertDialogRoot {
                        id: Into::<ReadSignal<Option<String>>>::into(alert_id),
                        open: Into::<ReadSignal<Option<bool>>>::into(alert_open),
                        on_open_change: Callback::new(move |o| alert_open.set(Some(o))),
                        default_open: false,
                        attributes: vec![],
                        AlertDialogContent { attributes: vec![],
                            AlertDialogTitle { attributes: vec![], "Confirm" }
                            AlertDialogDescription { attributes: vec![], "Proceed?" }
                            AlertDialogActions { attributes: vec![],
                                AlertDialogCancel { attributes: vec![], on_click: Some(EventHandler::new(move |_| {
                                    alert_close_cancel.set(Some(false));
                                })), "Cancel" }
                                AlertDialogAction { attributes: vec![], on_click: Some(EventHandler::new(move |_| {
                                    alert_close_ok.set(Some(false));
                                })), "OK" }
                            }
                        }
                    }
                    Sheet {
                        id: Into::<ReadSignal<Option<String>>>::into(sheet_id),
                        open: Into::<ReadSignal<Option<bool>>>::into(sheet_open),
                        on_open_change: Callback::new(move |o| sheet_open.set(Some(o))),
                        default_open: false,
                        is_modal: true,
                        attributes: vec![],
                        SheetContent { side: SheetSide::Right, attributes: vec![],
                            SheetHeader { attributes: vec![],
                                SheetTitle { attributes: vec![], "Sheet" }
                                SheetDescription { attributes: vec![], "Side panel" }
                            }
                            Button { variant: ButtonVariant::Primary, onclick: move |_| sheet_open.set(Some(false)), "Close" }
                        }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Popover, Tooltip, HoverCard" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    UiRow { class: "flex-wrap items-center gap-2",
                        PopoverRoot { default_open: false, attributes: vec![],
                            PopoverTrigger { attributes: vec![], Button { variant: ButtonVariant::Outline, "Popover" } }
                            PopoverContent { attributes: vec![], "Popover content" }
                        }
                        Tooltip { attributes: vec![],
                            TooltipTrigger { attributes: vec![], "Hover" }
                            TooltipContent { attributes: vec![], "Tip" }
                        }
                        HoverCard { attributes: vec![],
                            HoverCardTrigger { attributes: vec![], "Hover card" }
                            HoverCardContent { attributes: vec![], "Content" }
                        }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "DropdownMenu & ContextMenu" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    DropdownMenu { attributes: vec![],
                        DropdownMenuTrigger { attributes: vec![], "Menu" }
                        DropdownMenuContent { attributes: vec![],
                            DropdownMenuItem::<String> {
                                index: 0usize,
                                value: Into::<ReadSignal<String>>::into(dd_menu_a),
                                on_select: move |_: String| {},
                                attributes: vec![],
                                "One"
                            }
                        }
                    }
                    ContextMenu { attributes: vec![],
                        ContextMenuTrigger { attributes: vec![], "Right‑click" }
                        ContextMenuContent { attributes: vec![],
                            ContextMenuItem {
                                index: 0usize,
                                value: Into::<ReadSignal<String>>::into(ctx_menu_x),
                                on_select: move |_: String| {},
                                attributes: vec![],
                                "Action"
                            }
                        }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Menubar" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Menubar { attributes: vec![],
                        MenubarMenu { index: 0usize, attributes: vec![],
                            MenubarTrigger { attributes: vec![], "File" }
                            MenubarContent { attributes: vec![],
                                MenubarItem {
                                    index: 0usize,
                                    value: "n".to_string(),
                                    on_select: move |_: String| {},
                                    attributes: vec![],
                                    "New"
                                }
                            }
                        }
                    }
                    UiText { size: UiTextSize::Xs, tone: UiTextTone::Muted,
                        "Navbar requires a dioxus_router shell; use the upstream components preview for full nav demos." }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Pagination & Toolbar" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Pagination { attributes: vec![],
                        PaginationContent { attributes: vec![],
                            PaginationItem { attributes: vec![], PaginationPrevious { attributes: vec![] } }
                            PaginationItem { attributes: vec![],
                                PaginationLink { is_active: true, attributes: vec![], children: rsx! { "1" } }
                            }
                            PaginationItem { attributes: vec![], PaginationEllipsis { attributes: vec![] } }
                            PaginationItem { attributes: vec![], PaginationNext { attributes: vec![] } }
                        }
                    }
                    Toolbar { attributes: vec![],
                        ToolbarGroup { attributes: vec![],
                            ToolbarButton { index: 0usize, attributes: vec![], "Cut" }
                            ToolbarSeparator { attributes: vec![] }
                            ToolbarButton { index: 1usize, attributes: vec![], "Copy" }
                        }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Select" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Select::<String> {
                        value: Into::<ReadSignal<Option<Option<String>>>>::into(select_val),
                        on_value_change: Callback::new(move |v| select_val.set(Some(v))),
                        default_value: Some("apple".into()),
                        placeholder: Into::<ReadSignal<String>>::into(use_signal(|| "Pick…".to_string())),
                        attributes: vec![],
                        SelectTrigger { attributes: vec![], SelectValue { attributes: vec![] } }
                        SelectList { attributes: vec![],
                            SelectGroup { attributes: vec![],
                                SelectOption::<String> {
                                    value: Into::<ReadSignal<String>>::into(opt_apple),
                                    index: 0usize,
                                    text_value: None,
                                    attributes: vec![],
                                    "Apple" SelectItemIndicator {}
                                }
                                SelectOption::<String> {
                                    value: Into::<ReadSignal<String>>::into(opt_pear),
                                    index: 1usize,
                                    text_value: None,
                                    attributes: vec![],
                                    "Pear" SelectItemIndicator {}
                                }
                            }
                        }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Calendar & DatePicker" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    Calendar {
                        selected_date: selected_date(),
                        on_date_change: move |d| selected_date.set(d),
                        view_date: view_date(),
                        on_view_change: move |d| view_date.set(d),
                        attributes: vec![],
                        CalendarView {
                            CalendarHeader {
                                CalendarNavigation {
                                    CalendarPreviousMonthButton { attributes: vec![] }
                                    CalendarMonthTitle { attributes: vec![] }
                                    CalendarNextMonthButton { attributes: vec![] }
                                }
                            }
                            CalendarGrid { attributes: vec![] }
                        }
                    }
                    DatePicker {
                        selected_date: picker_date(),
                        on_value_change: move |d| picker_date.set(d),
                        attributes: vec![],
                        DatePickerInput { attributes: vec![] }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "DragAndDropList & VirtualList" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    DragAndDropList {
                        items: vec![rsx! { "Alpha" }, rsx! { "Beta" }, rsx! { "Gamma" }],
                        is_removable: false,
                        aria_label: Some("Reorder".into()),
                        attributes: vec![],
                        children: rsx! {}
                    }
                    VirtualList {
                        count: Into::<ReadSignal<usize>>::into(vl_count),
                        buffer: Into::<ReadSignal<usize>>::into(vl_buffer),
                        estimate_size: Some(Callback::new(|_| 28u32)),
                        render_item: Callback::new(|i| rsx! { div { class: "px-2 py-1", "Row {i}" } }),
                        attributes: vec![],
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "ToastProvider" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO}",
                    ToastProvider {
                        children: rsx! {
                            UiText { size: UiTextSize::Xs, tone: UiTextTone::Muted,
                                "Toast host mounted (use use_toast() from a child to show toasts)." }
                        }
                    }
                }
            }

            section { class: "{SECTION}",
                div { class: "{SECTION_TITLE}",
                    UiText { size: UiTextSize::Sm, tone: UiTextTone::Default, "Sidebar" }
                    UiDivider { variant: UiDividerVariant::HorizontalFade }
                }
                div { class: "{DEMO} overflow-hidden rounded-lg border border-white/10",
                    SidebarProvider { default_open: true, attributes: vec![],
                        Sidebar { attributes: vec![],
                            SidebarContent { attributes: vec![],
                                SidebarMenu { attributes: vec![],
                                    SidebarMenuItem { attributes: vec![],
                                        SidebarMenuButton { attributes: vec![], "Item" }
                                    }
                                }
                            }
                            SidebarInset { attributes: vec![],
                                SidebarTrigger { attributes: vec![] }
                                div { class: "p-4", UiText { "Main" } }
                            }
                        }
                    }
                }
            }
        }
    }
}
