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
        let camera = terragame_terminal::Camera::following((player.x, player.y).into(), viewport);
        Self {
            x: camera.origin.x,
            y: camera.origin.y,
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
