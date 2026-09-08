pub const SIMULATOR_READY_EVENT: &str = "simulator_ready";

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct SimulatorReady {
    pub port: u16,
    pub version: String,
    pub device_name: String,
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub struct SimulatorTouch {
    pub phase: SimulatorTouchPhase,
    pub x: f32,
    pub y: f32,
}

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum SimulatorTouchPhase {
    #[default]
    Down,
    Move,
    Up,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum SimulatorKey {
    Text(String),
    Code(u16),
    Button(HardwareButton),
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum HardwareButton {
    Home,
    Lock,
    Siri,
}

impl HardwareButton {
    pub fn as_arg(&self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Lock => "lock",
            Self::Siri => "siri",
        }
    }
}

impl SimulatorKey {
    pub fn of_browser_key(key: &str) -> Option<Self> {
        let code = match key {
            "Enter" => 40,
            "Escape" => 41,
            "Backspace" => 42,
            "Tab" => 43,
            "ArrowRight" => 79,
            "ArrowLeft" => 80,
            "ArrowDown" => 81,
            "ArrowUp" => 82,
            _ => {
                let mut chars = key.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else {
                    return None;
                };
                return Some(Self::Text(c.to_string()));
            }
        };
        Some(Self::Code(code))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_printable_key_is_typed_rather_than_coded() {
        assert_eq!(
            SimulatorKey::of_browser_key("a"),
            Some(SimulatorKey::Text("a".into()))
        );
        assert_eq!(
            SimulatorKey::of_browser_key("あ"),
            Some(SimulatorKey::Text("あ".into()))
        );
    }

    #[test]
    fn keys_with_no_text_become_hid_codes() {
        assert_eq!(
            SimulatorKey::of_browser_key("Enter"),
            Some(SimulatorKey::Code(40))
        );
        assert_eq!(
            SimulatorKey::of_browser_key("Backspace"),
            Some(SimulatorKey::Code(42))
        );
        assert_eq!(
            SimulatorKey::of_browser_key("ArrowUp"),
            Some(SimulatorKey::Code(82))
        );
    }

    #[test]
    fn a_modifier_or_unknown_named_key_is_dropped() {
        for key in ["Shift", "Meta", "F13", "Unidentified"] {
            assert_eq!(SimulatorKey::of_browser_key(key), None, "{key}");
        }
    }
}
