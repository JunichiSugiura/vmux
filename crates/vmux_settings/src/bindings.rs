//! Serializable keybinding tables (`settings.ron` → `input:`) and helpers for chord matching.

use bevy::prelude::*;
use serde::Deserialize;

/// Monotonic generation bumped when [`super::VmuxAppSettings`] input bindings change (file reload or startup).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VmuxBindingGeneration(pub u64);

/// Identifies a bindable command in RON (mirrors [`vmux_command::KeyAction`] without importing that crate).
#[derive(Clone, Copy, Debug, Deserialize, Reflect, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[reflect(Debug, PartialEq, Hash)]
pub enum BindingCommandId {
    Quit,
    ToggleCommandPalette,
    FocusCommandPaletteUrl,
    #[serde(alias = "toggle_history", alias = "ToggleHistory")]
    OpenHistory,
    OpenHistoryInNewTab,
    SplitHorizontal,
    SplitVertical,
    CycleNextPane,
    SelectPaneLeft,
    SelectPaneRight,
    SelectPaneUp,
    SelectPaneDown,
    SwapPaneLeft,
    SwapPaneRight,
    SwapPaneUp,
    SwapPaneDown,
    ToggleZoom,
    MirrorLayout,
    RotateBackward,
    RotateForward,
    #[serde(alias = "kill_active_pane")]
    ClosePane,
}

impl BindingCommandId {
    pub const fn palette_title(self) -> &'static str {
        match self {
            Self::Quit => "Quit vmux",
            Self::ToggleCommandPalette => "Toggle command palette",
            Self::FocusCommandPaletteUrl => "Focus URL in command palette",
            Self::OpenHistory => "Open history",
            Self::OpenHistoryInNewTab => "Open history in new tab",
            Self::SplitHorizontal => "Split pane horizontally",
            Self::SplitVertical => "Split pane vertically",
            Self::CycleNextPane => "Next pane",
            Self::SelectPaneLeft => "Focus pane left",
            Self::SelectPaneRight => "Focus pane right",
            Self::SelectPaneUp => "Focus pane up",
            Self::SelectPaneDown => "Focus pane down",
            Self::SwapPaneLeft => "Swap with left pane",
            Self::SwapPaneRight => "Swap with right pane",
            Self::SwapPaneUp => "Swap with pane above",
            Self::SwapPaneDown => "Swap with pane below",
            Self::ToggleZoom => "Toggle zoom pane",
            Self::MirrorLayout => "Mirror split",
            Self::RotateBackward => "Rotate layout backward",
            Self::RotateForward => "Rotate layout forward",
            Self::ClosePane => "Close pane",
        }
    }
}

/// One key chord step: held modifiers + physical key (Bevy [`KeyCode`] name).
#[derive(Clone, Debug, Deserialize, Reflect, PartialEq, Eq, Hash)]
#[reflect(Debug, PartialEq, Hash)]
pub struct ChordStep {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    /// Command / Super / Meta (⌘ on macOS).
    #[serde(default)]
    pub command: bool,
    /// [`KeyCode`] variant name, e.g. `"KeyB"`, `"Digit5"`, `"ArrowLeft"`.
    pub key: String,
}

impl ChordStep {
    pub fn modifiers_match(&self, keys: &ButtonInput<KeyCode>) -> bool {
        let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
        let command = keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight);
        ctrl == self.ctrl
            && shift == self.shift
            && alt == self.alt
            && command == self.command
    }

    /// `just_pressed` on the configured key with required modifiers held.
    pub fn just_pressed(&self, keys: &ButtonInput<KeyCode>) -> bool {
        let Some(k) = parse_key_code(&self.key) else {
            return false;
        };
        self.modifiers_match(keys) && keys.just_pressed(k)
    }

    /// Modifier keys held while `key` is pressed (for repeat / chord continuation).
    pub fn pressed_with_modifiers(&self, keys: &ButtonInput<KeyCode>) -> bool {
        let Some(k) = parse_key_code(&self.key) else {
            return false;
        };
        self.modifiers_match(keys) && keys.pressed(k)
    }
}

/// Global shortcut: one chord → command (used with leafwing [`InputMap`](leafwing_input_manager::InputMap)).
#[derive(Clone, Debug, Deserialize, Reflect, PartialEq, Eq, Hash)]
#[reflect(Debug, PartialEq, Hash)]
pub struct GlobalBinding {
    pub command: BindingCommandId,
    pub chord: ChordStep,
}

/// After prefix: second chord → command.
#[derive(Clone, Debug, Deserialize, Reflect, PartialEq, Eq, Hash)]
#[reflect(Debug, PartialEq, Hash)]
pub struct PrefixSecondBinding {
    pub chord: ChordStep,
    pub command: BindingCommandId,
}

#[derive(Clone, Debug, Deserialize, Reflect, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct PrefixChordSettings {
    /// First key that arms the prefix sequence (e.g. Ctrl+B).
    pub lead: ChordStep,
    #[serde(default = "default_prefix_timeout")]
    pub timeout_secs: f32,
    /// Keys accepted while prefix is armed.
    pub second: Vec<PrefixSecondBinding>,
}

fn default_prefix_timeout() -> f32 {
    crate::PREFIX_TIMEOUT_SECS
}

/// Full binding table (globals + prefix + optional direct Ctrl+Arrow focus).
#[derive(Clone, Debug, Deserialize, Reflect, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct VmuxBindingSettings {
    pub global: Vec<GlobalBinding>,
    pub prefix: PrefixChordSettings,
    #[serde(default = "default_ctrl_arrow_focus")]
    pub ctrl_arrow_focus: bool,
}

fn default_ctrl_arrow_focus() -> bool {
    true
}

impl Default for VmuxBindingSettings {
    fn default() -> Self {
        preset_bindings("tmux")
    }
}

/// Built-in presets: `"tmux"` (default), `"vim"` (placeholder alternate layout).
pub fn preset_bindings(name: &str) -> VmuxBindingSettings {
    match name.to_ascii_lowercase().as_str() {
        "vim" => vim_preset(),
        _ => tmux_preset(),
    }
}

/// Preset when `input` is omitted from `settings.ron`: env [`VMUX_BINDING_PRESET`] if set, else `name` (e.g. `"tmux"`), else tmux defaults.
pub fn preset_from_env_or_default(name: Option<&str>) -> VmuxBindingSettings {
    if let Ok(e) = std::env::var("VMUX_BINDING_PRESET") {
        return preset_bindings(&e);
    }
    preset_bindings(name.unwrap_or("tmux"))
}

fn tmux_preset() -> VmuxBindingSettings {
    let global = tmux_global_defaults();
    let second = tmux_prefix_second_defaults();
    VmuxBindingSettings {
        global,
        prefix: PrefixChordSettings {
            lead: ChordStep {
                ctrl: true,
                shift: false,
                alt: false,
                command: false,
                key: "KeyB".into(),
            },
            timeout_secs: super::PREFIX_TIMEOUT_SECS,
            second,
        },
        ctrl_arrow_focus: true,
    }
}

/// Alternate preset: different prefix lead (Ctrl+`) for visibility; second keys match tmux layout.
fn vim_preset() -> VmuxBindingSettings {
    let mut s = tmux_preset();
    s.prefix.lead = ChordStep {
        ctrl: true,
        shift: false,
        alt: false,
        command: false,
        key: "Backquote".into(),
    };
    s
}

fn tmux_global_defaults() -> Vec<GlobalBinding> {
    let mut g = Vec::new();
    g.push(GlobalBinding {
        command: BindingCommandId::Quit,
        chord: ChordStep {
            ctrl: false,
            shift: false,
            alt: false,
            command: true,
            key: "KeyQ".into(),
        },
    });
    g.push(GlobalBinding {
        command: BindingCommandId::Quit,
        chord: ChordStep {
            ctrl: true,
            shift: false,
            alt: false,
            command: false,
            key: "KeyQ".into(),
        },
    });
    #[cfg(target_os = "macos")]
    {
        g.push(GlobalBinding {
            command: BindingCommandId::ToggleCommandPalette,
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: true,
                key: "KeyT".into(),
            },
        });
        g.push(GlobalBinding {
            command: BindingCommandId::FocusCommandPaletteUrl,
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: true,
                key: "KeyL".into(),
            },
        });
        g.push(GlobalBinding {
            command: BindingCommandId::OpenHistory,
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: true,
                key: "KeyY".into(),
            },
        });
    }
    #[cfg(not(target_os = "macos"))]
    {
        g.push(GlobalBinding {
            command: BindingCommandId::ToggleCommandPalette,
            chord: ChordStep {
                ctrl: true,
                shift: false,
                alt: false,
                command: false,
                key: "KeyT".into(),
            },
        });
        g.push(GlobalBinding {
            command: BindingCommandId::FocusCommandPaletteUrl,
            chord: ChordStep {
                ctrl: true,
                shift: false,
                alt: false,
                command: false,
                key: "KeyL".into(),
            },
        });
        g.push(GlobalBinding {
            command: BindingCommandId::OpenHistory,
            chord: ChordStep {
                ctrl: true,
                shift: true,
                alt: false,
                command: false,
                key: "KeyH".into(),
            },
        });
    }
    g
}

fn tmux_prefix_second_defaults() -> Vec<PrefixSecondBinding> {
    vec![
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: true,
                alt: false,
                command: false,
                key: "Digit5".into(),
            },
            command: BindingCommandId::SplitHorizontal,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: true,
                alt: false,
                command: false,
                key: "Quote".into(),
            },
            command: BindingCommandId::SplitVertical,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "KeyO".into(),
            },
            command: BindingCommandId::CycleNextPane,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "ArrowLeft".into(),
            },
            command: BindingCommandId::SelectPaneLeft,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "ArrowRight".into(),
            },
            command: BindingCommandId::SelectPaneRight,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "ArrowUp".into(),
            },
            command: BindingCommandId::SelectPaneUp,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "ArrowDown".into(),
            },
            command: BindingCommandId::SelectPaneDown,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: true,
                shift: false,
                alt: false,
                command: false,
                key: "ArrowLeft".into(),
            },
            command: BindingCommandId::SwapPaneLeft,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: true,
                shift: false,
                alt: false,
                command: false,
                key: "ArrowRight".into(),
            },
            command: BindingCommandId::SwapPaneRight,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: true,
                shift: false,
                alt: false,
                command: false,
                key: "ArrowUp".into(),
            },
            command: BindingCommandId::SwapPaneUp,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: true,
                shift: false,
                alt: false,
                command: false,
                key: "ArrowDown".into(),
            },
            command: BindingCommandId::SwapPaneDown,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "KeyZ".into(),
            },
            command: BindingCommandId::ToggleZoom,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "KeyM".into(),
            },
            command: BindingCommandId::MirrorLayout,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: true,
                alt: false,
                command: false,
                key: "BracketRight".into(),
            },
            command: BindingCommandId::RotateBackward,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: true,
                alt: false,
                command: false,
                key: "BracketLeft".into(),
            },
            command: BindingCommandId::RotateForward,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "KeyR".into(),
            },
            command: BindingCommandId::RotateBackward,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: true,
                alt: false,
                command: false,
                key: "KeyR".into(),
            },
            command: BindingCommandId::RotateForward,
        },
        PrefixSecondBinding {
            chord: ChordStep {
                ctrl: false,
                shift: false,
                alt: false,
                command: false,
                key: "KeyX".into(),
            },
            command: BindingCommandId::ClosePane,
        },
    ]
}

impl VmuxBindingSettings {
    /// Secondary column text for the command palette (shortcut hint).
    pub fn shortcut_hint(&self, id: BindingCommandId) -> String {
        let global: Vec<&ChordStep> = self
            .global
            .iter()
            .filter(|g| g.command == id)
            .map(|g| &g.chord)
            .collect();
        if !global.is_empty() {
            return global
                .iter()
                .map(|c| format_chord_step(c))
                .collect::<Vec<_>>()
                .join(" / ");
        }
        for pb in &self.prefix.second {
            if pb.command == id {
                let lead = format_chord_step(&self.prefix.lead);
                let sec = format_chord_step(&pb.chord);
                return format!("{lead}, then {sec}");
            }
        }
        String::new()
    }

    /// True if this frame is a global binding chord for palette / URL (CEF suppression).
    pub fn global_chord_just_pressed_for_command(
        &self,
        keys: &ButtonInput<KeyCode>,
        command: BindingCommandId,
    ) -> bool {
        self.global.iter().any(|g| {
            g.command == command
                && g.chord.modifiers_match(keys)
                && parse_key_code(&g.chord.key).is_some_and(|k| keys.just_pressed(k))
        })
    }

    /// True if any global binding on this key+modifiers just fired (palette / URL).
    pub fn palette_suppress_chord_just_pressed(&self, keys: &ButtonInput<KeyCode>) -> bool {
        self.global_chord_just_pressed_for_command(keys, BindingCommandId::ToggleCommandPalette)
            || self.global_chord_just_pressed_for_command(
                keys,
                BindingCommandId::FocusCommandPaletteUrl,
            )
    }

    /// Prefix lead armed this frame (first key of a chord sequence).
    pub fn prefix_lead_just_pressed(&self, keys: &ButtonInput<KeyCode>) -> bool {
        self.prefix.lead.just_pressed(keys)
    }

    /// Same lead as first press; second Ctrl+B cancels.
    pub fn prefix_lead_just_pressed_double(&self, keys: &ButtonInput<KeyCode>) -> bool {
        self.prefix.lead.just_pressed(keys)
    }

    /// Match second-key table while prefix is armed.
    pub fn prefix_second_command(&self, keys: &ButtonInput<KeyCode>) -> Option<BindingCommandId> {
        for pb in &self.prefix.second {
            if pb.chord.just_pressed(keys) {
                return Some(pb.command);
            }
        }
        None
    }
}

fn format_chord_step(c: &ChordStep) -> String {
    let mut parts = Vec::new();
    if c.command {
        // ASCII label: default UI font has no ⌘ / modifier symbols (avoids tofu in Bevy palette).
        parts.push(if cfg!(target_os = "macos") { "Cmd" } else { "Super" });
    }
    if c.ctrl {
        parts.push(if cfg!(target_os = "macos") {
            "Ctrl"
        } else {
            "Ctrl"
        });
    }
    if c.shift {
        parts.push("Shift");
    }
    if c.alt {
        parts.push("Alt");
    }
    let key = if let Some(k) = parse_key_code(&c.key) {
        key_code_display(k)
    } else {
        c.key.clone()
    };
    if parts.is_empty() {
        key
    } else {
        format!("{}+{}", parts.join("+"), key)
    }
}

fn key_code_display(k: KeyCode) -> String {
    match k {
        // Arrow glyphs are outside the default palette font’s coverage; spell them out.
        KeyCode::ArrowLeft => "Left".into(),
        KeyCode::ArrowRight => "Right".into(),
        KeyCode::ArrowUp => "Up".into(),
        KeyCode::ArrowDown => "Down".into(),
        KeyCode::Digit5 => "%".into(),
        KeyCode::BracketLeft => "[".into(),
        KeyCode::BracketRight => "]".into(),
        _ => format!("{k:?}"),
    }
}

/// Parse Bevy [`KeyCode`] from a stable variant name (`KeyB`, `Digit5`, …).
pub fn parse_key_code(s: &str) -> Option<KeyCode> {
    Some(match s.trim() {
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Quote" => KeyCode::Quote,
        "Backquote" => KeyCode::Backquote,
        "Escape" => KeyCode::Escape,
        "Enter" => KeyCode::Enter,
        "Space" => KeyCode::Space,
        "Tab" => KeyCode::Tab,
        _ => return None,
    })
}
