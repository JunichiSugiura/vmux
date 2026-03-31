//! Build [`InputMap`](leafwing_input_manager::input_map::InputMap) from [`VmuxBindingSettings`].

use leafwing_input_manager::prelude::*;
use vmux_command::{key_action_from_binding_id, KeyAction};
use vmux_settings::{parse_key_code, ChordStep, VmuxBindingSettings};

fn chord_step_to_buttonlike(step: &ChordStep) -> Option<ButtonlikeChord> {
    let k = parse_key_code(&step.key)?;
    let mut c = ButtonlikeChord::default();
    if step.ctrl {
        c = c.with(ModifierKey::Control);
    }
    if step.shift {
        c = c.with(ModifierKey::Shift);
    }
    if step.alt {
        c = c.with(ModifierKey::Alt);
    }
    if step.command {
        c = c.with(ModifierKey::Super);
    }
    Some(c.with(k))
}

pub fn build_input_map(bindings: &VmuxBindingSettings) -> InputMap<KeyAction> {
    let mut input_map = InputMap::<KeyAction>::default();
    for g in &bindings.global {
        let Some(ka) = key_action_from_binding_id(g.command) else {
            continue;
        };
        let Some(chord) = chord_step_to_buttonlike(&g.chord) else {
            continue;
        };
        input_map.insert(ka, chord);
    }
    input_map
}
