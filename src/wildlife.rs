//! Small, deterministic active simulation. Untouched ambient animals may respawn;
//! injuries, corpses and player loot are retained independently of terrain chunks.
use crate::{
    planet::{Camp, Point},
    weather::Weather,
    world::{World, biome::Biome, seed::hash},
};
use std::collections::{BTreeMap, BTreeSet};
pub const ACTIVE_RADIUS: i64 = 48;
pub const MAX_ACTIVE: usize = 64;
pub const MAX_MEMORY: usize = 10_000;
pub type CreatureId = (i64, i64, u8);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behaviour {
    Grazer,
    Defender,
    Predator,
}
impl Behaviour {
    pub fn name(self) -> &'static str {
        match self {
            Self::Grazer => "grazer",
            Self::Defender => "defender",
            Self::Predator => "predator",
        }
    }
    pub fn glyph(self) -> char {
        match self {
            Self::Grazer => 'g',
            Self::Defender => 'd',
            Self::Predator => 'p',
        }
    }
    pub fn colour(self) -> u8 {
        match self {
            Self::Grazer => 10,
            Self::Defender => 11,
            Self::Predator => 9,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Species {
    pub name: String,
    pub behaviour: Behaviour,
    pub health: u8,
    pub speed: u8,
    pub detection: u8,
    pub resistance: u8,
    pub preferred: Biome,
    pub damage: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreatureState {
    pub id: CreatureId,
    pub position: Point,
    pub health: u8,
    pub angry: bool,
    pub looted: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Creature {
    pub state: CreatureState,
    pub home: Point,
    pub species: Species,
}
#[derive(Default)]
pub struct Wildlife {
    pub active: Vec<Creature>,
    pub memory: BTreeMap<CreatureId, CreatureState>,
    pub scanned: BTreeSet<CreatureId>,
}
pub fn distance(a: Point, b: Point) -> i128 {
    (i128::from(a.0) - i128::from(b.0)).abs() + (i128::from(a.1) - i128::from(b.1)).abs()
}
pub fn species(seed: u64, id: CreatureId, behaviour: Behaviour) -> Species {
    let h = hash(seed ^ 0x53504543494553 ^ u64::from(id.2), id.0, id.1);
    let first = [
        "Amber", "Velvet", "Glass", "Ribbon", "Spore", "Slate", "Moon", "Ash",
    ];
    let last = match behaviour {
        Behaviour::Grazer => ["hopper", "grazer", "moth", "strider"],
        Behaviour::Defender => ["hornback", "guardian", "shellback", "bristle"],
        Behaviour::Predator => ["fang", "stalker", "prowler", "claw"],
    };
    Species {
        name: format!(
            "{} {}",
            first[(h % 8) as usize],
            last[((h >> 8) % 4) as usize]
        ),
        behaviour,
        health: 12 + ((h >> 16) % 13) as u8,
        speed: 2 + ((h >> 24) % 3) as u8,
        detection: 6 + ((h >> 32) % 5) as u8,
        resistance: 15 + ((h >> 40) % 76) as u8,
        preferred: match (h >> 48) % 5 {
            0 => Biome::Forest,
            1 => Biome::Frozen,
            2 => Biome::Desert,
            3 => Biome::Wasteland,
            _ => Biome::Grassland,
        },
        damage: 3 + ((h >> 56) % 3) as u8,
    }
}
impl Wildlife {
    pub fn prototype(world: &mut World, camp: &Camp, id: CreatureId) -> Option<Creature> {
        let seed = world.seed();
        let h = hash(seed ^ 0x4352454154555245 ^ u64::from(id.2), id.0, id.1);
        let behaviour = if id.2 >= 3 {
            match id.2 {
                3 => Behaviour::Grazer,
                4 => Behaviour::Defender,
                5 => Behaviour::Predator,
                _ => return None,
            }
        } else {
            match h % 3 {
                0 => Behaviour::Grazer,
                1 => Behaviour::Defender,
                _ => Behaviour::Predator,
            }
        };
        let mut sp = species(seed, id, behaviour);
        sp.speed = sp
            .speed
            .saturating_add((world.profile().gravity / 120) as u8);
        let home = if id.2 >= 3 {
            if world.profile().legacy || (id.0, id.1) != camp.origin {
                return None;
            }
            camp.offset(
                match id.2 {
                    3 => 6,
                    4 => 10,
                    _ => 16,
                },
                if id.2 == 4 { 1 } else { 0 },
            )
        } else {
            if h % 100 >= u64::from(world.profile().wildlife) {
                return None;
            }
            let mut home = None;
            for attempt in 0..4u64 {
                let n = hash(seed ^ h ^ attempt, id.0, id.1);
                let x = (i128::from(id.0) * 32 + i128::from(n % 32))
                    .try_into()
                    .ok()?;
                let y = (i128::from(id.1) * 32 + i128::from((n >> 8) % 32))
                    .try_into()
                    .ok()?;
                let biome = camp.tile(world, x, y).biome;
                if !camp.clearing(x, y)
                    && biome.passable()
                    && (biome == sp.preferred || biome == Biome::Grassland)
                {
                    home = Some((x, y));
                    break;
                }
            }
            home?
        };
        Some(Creature {
            state: CreatureState {
                id,
                position: home,
                health: sp.health,
                angry: false,
                looted: false,
            },
            home,
            species: sp,
        })
    }
    pub fn refresh(&mut self, world: &mut World, camp: &Camp, player: Point) {
        self.active.retain(|c| {
            distance(c.state.position, player) <= i128::from(ACTIVE_RADIUS)
                && !(c.state.health == 0 && c.state.looted)
        });
        let mut candidates = Vec::new();
        for dy in -2..=2 {
            for dx in -2..=2 {
                for slot in 0..3 {
                    let id = (
                        player.0.div_euclid(32) + dx,
                        player.1.div_euclid(32) + dy,
                        slot,
                    );
                    if let Some(c) = Self::prototype(world, camp, id) {
                        candidates.push(c);
                    }
                }
            }
        }
        if !world.profile().legacy {
            for slot in 3..=5 {
                if let Some(c) = Self::prototype(world, camp, (camp.origin.0, camp.origin.1, slot))
                {
                    candidates.push(c);
                }
            }
        }
        candidates.sort_by_key(|c| (distance(c.home, player), c.state.id));
        let mut occupied: BTreeSet<_> = self
            .active
            .iter()
            .filter(|c| c.state.health > 0)
            .map(|c| c.state.position)
            .collect();
        for mut c in candidates {
            if self.active.len() >= MAX_ACTIVE {
                break;
            }
            if self.active.iter().any(|a| a.state.id == c.state.id) {
                continue;
            }
            if let Some(state) = self.memory.get(&c.state.id) {
                c.state = *state;
            }
            let p = c.state.position;
            if distance(p, player) > i128::from(ACTIVE_RADIUS)
                || c.state.health == 0 && c.state.looted
            {
                continue;
            }
            if c.state.health > 0
                && (p == player || occupied.contains(&p) || !Self::walkable(world, camp, p))
            {
                continue;
            }
            if c.state.health > 0 {
                occupied.insert(p);
            }
            self.active.push(c);
        }
        self.active.sort_by_key(|c| c.state.id);
    }
    fn walkable(world: &mut World, camp: &Camp, p: Point) -> bool {
        !camp.clearing(p.0, p.1)
            && !camp.sheltered(p)
            && camp.tile(world, p.0, p.1).biome.passable()
    }
    pub fn occupied(&self, p: Point) -> bool {
        self.active
            .iter()
            .any(|c| c.state.health > 0 && c.state.position == p)
    }
    pub fn tick(
        &mut self,
        world: &mut World,
        camp: &Camp,
        player: Point,
        ticks: u64,
    ) -> (u8, Vec<String>) {
        if !ticks.is_multiple_of(4) {
            return (0, Vec::new());
        }
        let seed = world.seed();
        let second = ticks / 4;
        let refuge = camp.clearing(player.0, player.1) || camp.sheltered(player);
        let mut damage = 0u8;
        let mut events = Vec::new();
        let mut occupied: BTreeSet<_> = self
            .active
            .iter()
            .filter(|c| c.state.health > 0)
            .map(|c| c.state.position)
            .collect();
        for creature in &mut self.active {
            let state = &mut creature.state;
            let sp = &creature.species;
            if state.health == 0 {
                continue;
            }
            let dist = distance(state.position, player);
            let home_distance = distance(player, creature.home);
            let angry = state.angry;
            state.angry = if refuge {
                false
            } else {
                match sp.behaviour {
                    Behaviour::Grazer => false,
                    Behaviour::Defender => (dist <= 2 || state.angry) && home_distance <= 6,
                    Behaviour::Predator => {
                        (dist <= i128::from(sp.detection) || state.angry)
                            && dist <= i128::from(sp.detection) + 4
                            && home_distance <= 12
                    }
                }
            };
            if state.angry && !angry {
                events.push(format!(
                    "{} turns hostile! Retreat; F attacks adjacent wildlife.",
                    sp.name
                ));
            }
            if state.angry && dist <= 1 {
                damage = damage.saturating_add(sp.damage);
            }
            let biome = camp.tile(world, state.position.0, state.position.1).biome;
            let weather = Weather::at(
                seed,
                world.profile(),
                biome,
                state.position.0,
                state.position.1,
                ticks,
            );
            if second.is_multiple_of(8)
                && weather.current.dangerous()
                && sp.resistance < weather.severity * 20
                && (self.memory.contains_key(&state.id) || self.memory.len() < MAX_MEMORY)
            {
                state.health = state.health.saturating_sub(1);
                self.memory.insert(state.id, *state);
                if state.health == 0 {
                    occupied.remove(&state.position);
                    continue;
                }
            }
            if !second.is_multiple_of(u64::from(sp.speed)) {
                if self.memory.contains_key(&state.id) {
                    self.memory.insert(state.id, *state);
                }
                continue;
            }
            let flee = sp.behaviour == Behaviour::Grazer && dist <= 4;
            let target = if state.angry || flee {
                player
            } else {
                creature.home
            };
            let mut moves = Vec::new();
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let (Some(x), Some(y)) = (
                    state.position.0.checked_add(dx),
                    state.position.1.checked_add(dy),
                ) else {
                    continue;
                };
                let p = (x, y);
                let leash = if state.angry { 12 } else { 4 };
                if p == player
                    || occupied.contains(&p)
                    || distance(p, creature.home) > leash
                    || !Self::walkable(world, camp, p)
                {
                    continue;
                }
                let rank = if flee {
                    -distance(p, target)
                } else if state.angry || distance(state.position, creature.home) > 2 {
                    distance(p, target)
                } else {
                    (hash(seed ^ ticks ^ u64::from(state.id.2), x, y) % 16) as i128
                };
                moves.push((rank, p));
            }
            moves.sort_unstable();
            if let Some(&(_, p)) = moves.first() {
                occupied.remove(&state.position);
                state.position = p;
                occupied.insert(p);
            }
            if self.memory.contains_key(&state.id) {
                self.memory.insert(state.id, *state);
            }
        }
        (damage, events)
    }
    pub fn attack(&mut self, player: Point) -> Result<String, &'static str> {
        let c = self
            .active
            .iter_mut()
            .filter(|c| c.state.health > 0 && distance(c.state.position, player) <= 1)
            .min_by_key(|c| c.state.id)
            .ok_or("No adjacent creature. F attacks; E interacts.")?;
        if !self.memory.contains_key(&c.state.id) && self.memory.len() >= MAX_MEMORY {
            return Err("Wildlife record limit reached; avoid further combat");
        }
        c.state.health = c.state.health.saturating_sub(8);
        c.state.angry = c.species.behaviour != Behaviour::Grazer;
        self.memory.insert(c.state.id, c.state);
        Ok(if c.state.health == 0 {
            format!(
                "{} defeated. Stand on x and E to recover organics.",
                c.species.name
            )
        } else {
            format!(
                "Hit {}: {} health remaining.",
                c.species.name, c.state.health
            )
        })
    }
    pub fn corpse(&self, p: Point) -> Option<CreatureId> {
        self.active
            .iter()
            .find(|c| c.state.health == 0 && !c.state.looted && c.state.position == p)
            .map(|c| c.state.id)
    }
    pub fn loot_corpse(&mut self, id: CreatureId) {
        if let Some(c) = self.active.iter_mut().find(|c| c.state.id == id) {
            c.state.looted = true;
            self.memory.insert(id, c.state);
        }
    }
    pub fn validate_record(world: &mut World, camp: &Camp, state: CreatureState) -> bool {
        let Some(c) = Self::prototype(world, camp, state.id) else {
            return false;
        };
        state.health <= c.species.health
            && distance(state.position, c.home) <= 12
            && camp
                .tile(world, state.position.0, state.position.1)
                .biome
                .passable()
            && !camp.clearing(state.position.0, state.position.1)
            && (state.health == 0 || !camp.sheltered(state.position))
            && !(state.health > 0 && state.looted)
            && !(state.angry && c.species.behaviour == Behaviour::Grazer)
    }
}
