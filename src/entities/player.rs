use crate::world::World;
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player {
    pub x: i64,
    pub y: i64,
}
impl Player {
    pub fn step(&mut self, dx: i64, dy: i64, world: &mut World) -> bool {
        let moved = self.step_if(dx, dy, |x, y| world.tile(x, y).biome.passable());
        if moved {
            world.focus(self.x, self.y);
        }
        moved
    }
    pub fn step_if(
        &mut self,
        dx: i64,
        dy: i64,
        mut passable: impl FnMut(i64, i64) -> bool,
    ) -> bool {
        if !matches!((dx, dy), (1, 0) | (-1, 0) | (0, 1) | (0, -1)) {
            return false;
        }
        let (Some(x), Some(y)) = (self.x.checked_add(dx), self.y.checked_add(dy)) else {
            return false;
        };
        if !passable(x, y) {
            return false;
        }
        self.x = x;
        self.y = y;
        true
    }
}
