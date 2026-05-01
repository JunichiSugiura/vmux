pub const FOOTER_HEIGHT_PX: f32 = 32.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footer_height_is_compact() {
        assert_eq!(FOOTER_HEIGHT_PX, 32.0);
    }
}
