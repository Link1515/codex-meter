use crate::window::types::WindowPlacementState;

#[derive(Debug, Clone, Copy)]
pub struct VisibleBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn ensure_visible(
    placement: &WindowPlacementState,
    bounds: VisibleBounds,
) -> WindowPlacementState {
    let max_x = bounds.x + bounds.width.saturating_sub(placement.width) as i32;
    let max_y = bounds.y + bounds.height.saturating_sub(placement.height) as i32;

    WindowPlacementState {
        x: placement.x.clamp(bounds.x, max_x.max(bounds.x)),
        y: placement.y.clamp(bounds.y, max_y.max(bounds.y)),
        width: placement.width.min(bounds.width),
        height: placement.height.min(bounds.height),
        display_id: placement.display_id.clone(),
        updated_at: placement.updated_at.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{ensure_visible, VisibleBounds};
    use crate::window::types::WindowPlacementState;

    #[test]
    fn corrects_out_of_bounds_placement() {
        let placement = WindowPlacementState {
            x: 3000,
            y: -500,
            width: 360,
            height: 240,
            display_id: None,
            updated_at: "now".to_string(),
        };

        let corrected = ensure_visible(
            &placement,
            VisibleBounds {
                x: 0,
                y: 0,
                width: 1280,
                height: 720,
            },
        );

        assert_eq!(corrected.x, 920);
        assert_eq!(corrected.y, 0);
    }
}
