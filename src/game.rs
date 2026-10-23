use crate::{
    entities::player::Player,
    exploration::{Exploration, ScanReport},
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
    pub message: String,
    pub exploration: Exploration,
    pub last_scan: Option<ScanReport>,
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
            message: "Landing zone secure. R scans for discoveries.".into(),
            exploration: Exploration::default(),
            last_scan: None,
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
                self.observe(false);
            }
            (State::Playing, Action::Move(dx, dy)) => {
                self.message = if self.player.step(dx, dy, &mut self.world) {
                    self.observe(false);
                    let tile = self.world.tile(self.player.x, self.player.y);
                    tile.landmark
                        .map_or("Exploring the surface.", |site| site.name())
                } else {
                    "Route blocked: water, mountains or world boundary."
                }
                .into();
                return true;
            }
            (State::Playing, Action::Scan) => {
                let report = self.observe(true);
                self.message = format!(
                    "Scan: +{} cells, +{} sites. {}",
                    report.new_cells,
                    report.new_sites,
                    report
                        .sites
                        .first()
                        .map_or("No signals nearby.".into(), |site| format!(
                            "{} at {},{}",
                            site.kind.name(),
                            site.x,
                            site.y
                        ))
                );
                self.last_scan = Some(report);
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
    fn observe(&mut self, scan: bool) -> ScanReport {
        self.exploration
            .reveal(&mut self.world, self.player.x, self.player.y, scan)
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
