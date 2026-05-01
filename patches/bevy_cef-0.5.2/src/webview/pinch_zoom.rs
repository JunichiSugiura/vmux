const PINCH_ZOOM_LEVELS_PER_DELTA: f64 = 4.0;
const MIN_ZOOM_LEVEL: f64 = -7.0;
const MAX_ZOOM_LEVEL: f64 = 7.0;

pub(crate) fn zoom_level_after_pinch(current: f64, delta: f64) -> f64 {
    (current + delta * PINCH_ZOOM_LEVELS_PER_DELTA).clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinch_delta_maps_to_cef_zoom_step() {
        assert_eq!(zoom_level_after_pinch(0.0, 0.125), 0.5);
        assert_eq!(zoom_level_after_pinch(1.0, -0.125), 0.5);
    }

    #[test]
    fn pinch_zoom_level_is_clamped() {
        assert_eq!(zoom_level_after_pinch(7.0, 1.0), 7.0);
        assert_eq!(zoom_level_after_pinch(-7.0, -1.0), -7.0);
    }
}
