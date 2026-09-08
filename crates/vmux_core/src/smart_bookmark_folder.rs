#[cfg(host)]
use bevy::ecs::reflect::ReflectComponent;
#[cfg(host)]
use bevy::prelude::{Component, Reflect};

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
#[cfg_attr(host, derive(Component, Reflect))]
#[cfg_attr(host, reflect(Component))]
#[cfg_attr(host, type_path = "vmux_core")]
#[serde(rename_all = "snake_case")]
pub enum SmartBookmarkFolder {
    Projects,
    Knowledge,
    Tools,
}
