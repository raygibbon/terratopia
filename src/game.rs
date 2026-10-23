use crate::{
    entities::player::Player,
    input::Action,
    survival::Survival,
    world::{World, seed::planet_name},
};
use std::time::Duration;
pub const STEP: Duration = Duration::from_millis(250);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    MainMenu,
    NewWorld,
    Playing,
    Paused,
    Quit,
}
pub struct Game {
    pub state: State,
    pub world: World,
    pub player: Player,
    pub survival: Survival,
    pub planet: String,
    pub message: &'static str,
    accumulator: Duration,
}
impl Game {
    pub fn new(seed: u64) -> Self {
        let mut world = World::new(seed, 128);
        let (x, y) = world.landing_site();
        world.focus(x, y);
        Self {
            state: State::MainMenu,
            world,
            player: Player { x, y },
            survival: Survival::default(),
            planet: planet_name(seed),
            message: "Landing zone secure. Explore the surface.",
            accumulator: Duration::ZERO,
        }
    }
    pub fn handle(&mut self, action: Action) -> bool {
        let before = self.state;
        match (self.state, action) {
            (_, Action::Quit) => self.state = State::Quit,
            (State::MainMenu, Action::Accept | Action::NewWorld) => self.state = State::NewWorld,
            (State::NewWorld, Action::Accept) => {
                let seed = self.world.seed();
                *self = Self::new(seed);
                self.state = State::Playing;
            }
            (State::Playing, Action::Move(dx, dy)) => {
                self.message = if self.player.step(dx, dy, &mut self.world) {
                    "Exploring the surface."
                } else {
                    "Route blocked: water, mountains or world boundary."
                };
                return true;
            }
            (State::Playing, Action::Pause) => self.state = State::Paused,
            (State::Paused, Action::Pause | Action::Accept) => self.state = State::Playing,
            (State::Paused, Action::Menu) | (State::NewWorld, Action::Menu) => {
                self.state = State::MainMenu
            }
            _ => {}
        }
        if self.state != before {
            self.accumulator = Duration::ZERO;
        }
        self.state != before
    }
    /// Four fixed updates maximum; discard excess elapsed time after a stall.
    pub fn advance(&mut self, elapsed: Duration) -> bool {
        if self.state != State::Playing {
            return false;
        }
        self.accumulator += elapsed.min(STEP * 4);
        let mut redraw = false;
        for _ in 0..4 {
            if self.accumulator < STEP {
                break;
            }
            self.accumulator -= STEP;
            redraw |= self.survival.tick();
        }
        redraw
    }
}
