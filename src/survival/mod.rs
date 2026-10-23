use crate::{
    weather::{Condition, Weather},
    world::{biome::Biome, profile::Profile},
};
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
#[derive(Clone, Copy)]
pub struct Conditions {
    pub planet: Profile,
    pub weather: Weather,
    pub filtered: bool,
    pub crash_shield: bool,
    pub resting: bool,
}
impl Default for Conditions {
    fn default() -> Self {
        Self {
            planet: Profile::legacy(),
            weather: Weather::default(),
            filtered: false,
            crash_shield: false,
            resting: false,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exposure {
    pub temperature: i16,
    pub damage: u8,
    pub breathable: bool,
    pub oxygen_interval: u64,
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
    pub fn exposure(&self, env: Environment, mut context: Conditions) -> Exposure {
        let breathable = context.planet.breathable() && context.weather.current != Condition::Toxic;
        let oxygen_interval = if context.planet.legacy || context.planet.oxygen >= 5 {
            3
        } else {
            2
        };
        if context.crash_shield {
            context.planet = Profile::legacy();
            context.weather = Weather::default();
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
        // Legacy worlds retain their exact biome temperatures. New worlds use
        // the planetary climate as the base, with smaller local biome offsets.
        let baseline = if context.planet.legacy {
            base
        } else {
            context.planet.temperature + (base - 18) / 3
        };
        let temperature = if env.sheltered {
            18
        } else {
            baseline
                - if self.night() {
                    i16::from(context.planet.variation)
                } else {
                    0
                }
                + context.weather.temperature()
        };
        let (low, high) = if env.protected { (-40, 65) } else { (0, 38) };
        let mut damage = if temperature < low || temperature > high {
            2
        } else {
            0
        };
        if !env.protected && env.biome == Biome::Wasteland {
            damage = damage.max(2);
        }
        if !env.protected && context.planet.radiation > 35 {
            damage += 1;
        }
        if !context.filtered && context.planet.toxicity > 35 {
            damage += 1;
        }
        if context.weather.current == Condition::Toxic && !context.filtered {
            damage += context.weather.severity;
        }
        if context.weather.current == Condition::Radiation && !env.protected {
            damage += context.weather.severity;
        }
        if env.sheltered {
            damage = 0;
        }
        Exposure {
            temperature,
            damage,
            breathable,
            oxygen_interval,
        }
    }
    pub fn temperature(&self, env: Environment) -> i16 {
        self.exposure(env, Conditions::default()).temperature
    }
    pub fn hazardous(&self, env: Environment) -> bool {
        self.exposure(env, Conditions::default()).damage > 0
    }
    pub fn tick(&mut self, env: Environment) -> bool {
        self.tick_living(env, Conditions::default())
    }
    pub fn tick_living(&mut self, env: Environment, context: Conditions) -> bool {
        self.ticks = self.ticks.saturating_add(1);
        if !self.ticks.is_multiple_of(4) {
            return false;
        }
        let second = self.ticks / 4;
        let exposure = self.exposure(env, context);
        if env.powered {
            self.oxygen = self.oxygen.saturating_add(3).min(100);
        } else if exposure.breathable {
            if second.is_multiple_of(6) {
                self.oxygen = self.oxygen.saturating_add(1).min(100);
            }
        } else if second.is_multiple_of(exposure.oxygen_interval) {
            self.oxygen = self.oxygen.saturating_sub(1);
        }
        if self.oxygen == 0 {
            self.health = self.health.saturating_sub(3);
        }
        if second.is_multiple_of(if self.night() { 4 } else { 8 }) {
            self.health = self.health.saturating_sub(exposure.damage);
        }
        if env.sheltered && self.oxygen > 20 && second.is_multiple_of(5) {
            self.health = self.health.saturating_add(1).min(100);
        }
        if context.resting {
            self.energy = self
                .energy
                .saturating_add(if env.sheltered { 8 } else { 5 })
                .min(100);
        } else if second.is_multiple_of(3) {
            self.energy = self
                .energy
                .saturating_add(if env.sheltered { 3 } else { 1 })
                .min(100);
        }
        true
    }
}
