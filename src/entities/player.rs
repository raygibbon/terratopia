use crate::world::World;
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player {
    pub x: i64,
    pub y: i64,
}
impl Player {
    pub fn step(&mut self, dx: i64, dy: i64, world: &mut World) -> bool {
        if !matches!((dx, dy), (1, 0) | (-1, 0) | (0, 1) | (0, -1)) {
            return false;
        }
        let (Some(x), Some(y)) = (self.x.checked_add(dx), self.y.checked_add(dy)) else {
            return false;
        };
        if !world.tile(x, y).biome.passable() {
            return false;
        }
        self.x = x;
        self.y = y;
        world.focus(x, y);
        true
    }
}
