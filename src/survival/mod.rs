use crate::world::biome::Biome;
pub const DAY_TICKS: u64 = 480;
pub const NIGHT_START: u64 = 320;
pub const NIGHT_TICKS: u16 = 160;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Survival {
    pub health: u8,
    pub oxygen: u8,
    pub energy: u8,
    ticks: u64,
}
#[derive(Clone, Copy)]
pub struct Environment {
    pub biome: Biome,
    pub sheltered: bool,
    pub protected: bool,
    pub powered: bool,
}
impl Default for Survival {
    fn default() -> Self {
        Self {
            health: 100,
            oxygen: 100,
            energy: 100,
            ticks: 0,
        }
    }
}
impl Survival {
    pub fn from_parts(health: u8, oxygen: u8, energy: u8, ticks: u64) -> Option<Self> {
        (health <= 100 && oxygen <= 100 && energy <= 100).then_some(Self {
            health,
            oxygen,
            energy,
            ticks,
        })
    }
    pub fn ticks(&self) -> u64 {
        self.ticks
    }
    pub fn day(&self) -> u64 {
        self.ticks / DAY_TICKS + 1
    }
    pub fn night(&self) -> bool {
        self.ticks % DAY_TICKS >= NIGHT_START
    }
    pub fn seconds_to_dawn(&self) -> u64 {
        (DAY_TICKS - self.ticks % DAY_TICKS).div_ceil(4)
    }
    pub fn temperature(&self, env: Environment) -> i16 {
        if env.sheltered {
            return 18;
        }
        let base = match env.biome {
            Biome::Grassland => 18,
            Biome::Forest => 14,
            Biome::Desert => 44,
            Biome::Mountains => -12,
            Biome::Water => 5,
            Biome::Frozen => -30,
            Biome::Wasteland => 24,
        };
        base - if self.night() { 20 } else { 0 }
    }
    pub fn hazardous(&self, env: Environment) -> bool {
        if env.sheltered {
            return false;
        }
        let (low, high) = if env.protected { (-40, 65) } else { (0, 38) };
        let temp = self.temperature(env);
        temp < low || temp > high || env.biome == Biome::Wasteland && !env.protected
    }
    /// Fixed 250ms tick; visible vitals change only on second boundaries.
    pub fn tick(&mut self, env: Environment) -> bool {
        self.ticks = self.ticks.saturating_add(1);
        if !self.ticks.is_multiple_of(4) {
            return false;
        }
        let second = self.ticks / 4;
        if env.powered {
            self.oxygen = self.oxygen.saturating_add(3).min(100);
        } else if second.is_multiple_of(3) {
            self.oxygen = self.oxygen.saturating_sub(1);
        }
        if self.oxygen == 0 {
            self.health = self.health.saturating_sub(3);
        }
        let interval = if self.night() { 4 } else { 8 };
        if self.hazardous(env) && second.is_multiple_of(interval) {
            self.health = self.health.saturating_sub(2);
        }
        if env.sheltered && self.oxygen > 20 && second.is_multiple_of(5) {
            self.health = self.health.saturating_add(1).min(100);
        }
        if second.is_multiple_of(3) {
            self.energy = self
                .energy
                .saturating_add(if env.sheltered { 3 } else { 1 })
                .min(100);
        }
        true
    }
}
