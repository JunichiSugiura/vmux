use bevy::prelude::*;
use bevy_cef::prelude::{BinReceive, HostWindow};
use std::sync::{LazyLock, Mutex};
use vmux_core::overlay::{OverlayState, OverlayStateQuery};
use vmux_flex::prelude::{ComputedNode, LayoutSystems};
use vmux_layout::event::WindowDragRegionEvent;
use vmux_layout::{Header, LayoutCef, Open};

use crate::LayoutPointerCapture;

impl Plugin for WindowDragPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReportedWindowDragRegions>()
            .add_observer(on_window_drag_region)
            .add_systems(
                PostUpdate,
                publish_window_drag_region.after(LayoutSystems::Layout),
            );
    }
}

pub(crate) struct WindowDragPlugin;

static REGIONS: LazyLock<Mutex<Vec<WindowDragRegion>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowDragRegion {
    left_px: f32,
    top_px: f32,
    right_px: f32,
    bottom_px: f32,
}

impl WindowDragRegion {
    pub fn contains_point(x_px: f32, y_px: f32) -> bool {
        let published = REGIONS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        published.iter().any(|region| region.contains(x_px, y_px))
    }

    fn of(reported: WindowDragRegionEvent, header: ComputedNode) -> Option<Self> {
        if header.is_empty() || !reported.is_finite() {
            return None;
        }
        let scale = header.scale();
        let left_px = reported.left * scale;
        let top_px = reported.top * scale;
        let region = Self {
            left_px: left_px.max(header.min().x),
            top_px: top_px.max(header.min().y),
            right_px: (left_px + reported.width * scale).min(header.max().x),
            bottom_px: (top_px + reported.height * scale).min(header.max().y),
        };
        if region.right_px <= region.left_px || region.bottom_px <= region.top_px {
            return None;
        }
        Some(region)
    }

    fn contains(self, x_px: f32, y_px: f32) -> bool {
        x_px >= self.left_px
            && x_px <= self.right_px
            && y_px >= self.top_px
            && y_px <= self.bottom_px
    }

    fn publish(regions: Vec<Self>) {
        let mut published = REGIONS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *published = regions;
    }
}

#[derive(Resource, Default)]
struct ReportedWindowDragRegions(
    std::collections::BTreeMap<(Entity, String), WindowDragRegionEvent>,
);

impl ReportedWindowDragRegions {
    fn update(&mut self, webview: Entity, region: WindowDragRegionEvent) {
        let key = (webview, region.id.clone());
        if region.removed {
            self.0.remove(&key);
        } else {
            self.0.insert(key, region);
        }
    }
}

fn on_window_drag_region(
    trigger: On<BinReceive<WindowDragRegionEvent>>,
    mut reported: ResMut<ReportedWindowDragRegions>,
) {
    let region = trigger.event().payload.clone();
    reported.update(trigger.event().webview, region);
}

fn publish_window_drag_region(
    reported: Res<ReportedWindowDragRegions>,
    header_q: Query<(Entity, &ComputedNode, Has<Open>), With<Header>>,
    child_of: Query<&ChildOf>,
    host_windows: Query<&HostWindow>,
    focused_window: Res<vmux_layout::window::FocusedWindow>,
    overlay_q: OverlayStateQuery,
    pointer_capture_q: Query<(Entity, &HostWindow), (With<LayoutCef>, LayoutPointerCapture)>,
    mut last: Local<Vec<WindowDragRegion>>,
) {
    let overlay_owns_input = OverlayState::of_any(&overlay_q).owns_input()
        || pointer_capture_q
            .iter()
            .any(|(_, host)| Some(host.0) == focused_window.0);
    let mut regions = Vec::new();
    if !overlay_owns_input {
        for (entity, header, open) in header_q.iter() {
            if !open
                || vmux_layout::window::host_window_of(entity, &child_of, &host_windows)
                    != focused_window.0
            {
                continue;
            }
            for ((webview, _), reported) in reported.0.iter() {
                if vmux_layout::window::host_window_of(*webview, &child_of, &host_windows)
                    != focused_window.0
                {
                    continue;
                }
                if let Some(region) = WindowDragRegion::of(reported.clone(), *header) {
                    regions.push(region);
                }
            }
            break;
        }
    }
    if *last == regions {
        return;
    }
    *last = regions.clone();
    WindowDragRegion::publish(regions);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HeaderNode;

    impl HeaderNode {
        fn at(min: Vec2, size: Vec2, inverse_scale_factor: f32) -> ComputedNode {
            ComputedNode {
                size,
                center: min + size * 0.5,
                inverse_scale_factor,
                ..ComputedNode::default()
            }
        }
    }

    #[test]
    fn a_reported_region_becomes_physical_pixels_in_window_space() {
        let header = HeaderNode::at(Vec2::new(16.0, 16.0), Vec2::new(2000.0, 168.0), 0.5);
        let reported = WindowDragRegionEvent {
            id: "trailing".to_string(),
            removed: false,
            left: 300.0,
            top: 8.0,
            width: 400.0,
            height: 40.0,
        };

        let region = WindowDragRegion::of(reported, header).expect("region");

        assert_eq!(region.left_px, 600.0);
        assert_eq!(region.top_px, 16.0);
        assert_eq!(region.right_px, 1400.0);
        assert_eq!(region.bottom_px, 96.0);
        assert!(region.contains(700.0, 20.0));
        assert!(!region.contains(599.0, 20.0));
        assert!(!region.contains(700.0, 97.0));
    }

    #[test]
    fn a_region_reaching_past_the_header_is_clipped_to_it() {
        let header = HeaderNode::at(Vec2::ZERO, Vec2::new(1000.0, 168.0), 0.5);

        let region = WindowDragRegion::of(
            WindowDragRegionEvent {
                id: "trailing".to_string(),
                removed: false,
                left: 400.0,
                top: 0.0,
                width: 400.0,
                height: 40.0,
            },
            header,
        )
        .expect("region");

        assert_eq!(region.right_px, 1000.0);
        assert!(!region.contains(1001.0, 20.0));
    }

    #[test]
    fn a_region_the_header_has_squeezed_away_is_not_draggable() {
        let header = HeaderNode::at(Vec2::ZERO, Vec2::new(200.0, 84.0), 1.0);

        assert_eq!(
            WindowDragRegion::of(
                WindowDragRegionEvent {
                    id: "trailing".to_string(),
                    removed: false,
                    left: 400.0,
                    top: 0.0,
                    width: 100.0,
                    height: 40.0,
                },
                header
            ),
            None
        );
        assert_eq!(
            WindowDragRegion::of(
                WindowDragRegionEvent {
                    id: "trailing".to_string(),
                    removed: false,
                    left: 10.0,
                    top: 0.0,
                    width: 0.0,
                    height: 40.0,
                },
                header
            ),
            None
        );
    }

    #[test]
    fn removing_a_reported_region_drops_it_from_the_next_publish() {
        let mut reported = ReportedWindowDragRegions::default();
        let webview = Entity::from_bits(1);
        reported.update(webview, WindowDragRegionEvent {
            id: "leading".to_string(),
            removed: false,
            left: 10.0,
            top: 0.0,
            width: 20.0,
            height: 40.0,
        });

        reported.update(webview, WindowDragRegionEvent {
            id: "leading".to_string(),
            removed: true,
            ..Default::default()
        });

        assert!(reported.0.is_empty());
    }
}
