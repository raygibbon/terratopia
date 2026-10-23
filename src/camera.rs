use crate::entities::player::Player;
use terrabox::{Rect, Size};
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub x: i64,
    pub y: i64,
    pub viewport: Rect,
}
impl Camera {
    pub fn following(player: Player, viewport: Rect) -> Self {
        fn origin(center: i64, span: u16) -> i64 {
            let extent = i128::from(span.saturating_sub(1));
            (i128::from(center) - i128::from(span / 2))
                .clamp(i128::from(i64::MIN), i128::from(i64::MAX) - extent) as i64
        }
        Self {
            x: origin(player.x, viewport.width),
            y: origin(player.y, viewport.height),
            viewport,
        }
    }
    pub fn world_at(self, x: u16, y: u16) -> (i64, i64) {
        (
            self.x.saturating_add(i64::from(x)),
            self.y.saturating_add(i64::from(y)),
        )
    }
}
pub fn viewport(size: Size) -> Rect {
    Rect::new(
        0,
        3.min(size.height),
        size.width,
        size.height.saturating_sub(5),
    )
}
pub fn usable(size: Size) -> bool {
    size.width >= 40 && size.height >= 12
}
