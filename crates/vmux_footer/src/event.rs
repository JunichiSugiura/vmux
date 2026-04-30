pub const SPACES_EVENT: &str = "spaces";

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpacesHostEvent {
    pub spaces: Vec<SpaceRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpaceRow {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct FooterCommandEvent {
    /// "switch" | "new"
    pub command: String,
    #[serde(default)]
    pub space_id: Option<String>,
}
