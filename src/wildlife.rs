//! Persistent ecosystem rules. Engine handles are runtime-only; tuple IDs never change.
use crate::{
    planet::{Camp, Point},
    weather::Weather,
    world::{World, biome::Biome, seed::hash},
};
use std::collections::{BTreeMap, BTreeSet};
pub const ACTIVE_RADIUS: i64 = 48;
pub const MAX_ACTIVE: usize = 1_024;
pub const MAX_REGIONS: usize = 10_000;
pub const REGION_CAPACITY: u16 = 24;
pub const PATH_BUDGET: usize = 512;
pub const MAX_MEMORY: usize = 10_000;
pub type CreatureId = (i64, i64, u8);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behaviour {
    Grazer,
    Defender,
    Predator,
    Scavenger,
}
impl Behaviour {
    pub fn name(self) -> &'static str {
        match self {
            Self::Grazer => "grazer",
            Self::Defender => "defender",
            Self::Predator => "predator",
            Self::Scavenger => "scavenger",
        }
    }
    pub fn glyph(self) -> char {
        match self {
            Self::Grazer => 'g',
            Self::Defender => 'd',
            Self::Predator => 'p',
            Self::Scavenger => 's',
        }
    }
    pub fn colour(self) -> u8 {
        match self {
            Self::Grazer => 10,
            Self::Defender => 11,
            Self::Predator => 9,
            Self::Scavenger => 13,
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
    pub lifespan: u32,
    pub maturity: u32,
    pub birth_period: u64,
    pub territory: u8,
    pub temperature: i16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreatureState {
    pub id: CreatureId,
    pub position: Point,
    pub health: u8,
    pub angry: bool,
    pub looted: bool,
    pub needs: Needs,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Creature {
    pub state: CreatureState,
    pub home: Point,
    pub species: Species,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Activity {
    #[default]
    Idle,
    Wander,
    Forage,
    Eat,
    Rest,
    Flee,
    Defend,
    Hunt,
    Return,
}
impl Activity {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Wander => "wander",
            Self::Forage => "forage",
            Self::Eat => "eat",
            Self::Rest => "rest",
            Self::Flee => "flee",
            Self::Defend => "defend",
            Self::Hunt => "hunt",
            Self::Return => "return to territory",
        }
    }
    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            0 => Self::Idle,
            1 => Self::Wander,
            2 => Self::Forage,
            3 => Self::Eat,
            4 => Self::Rest,
            5 => Self::Flee,
            6 => Self::Defend,
            7 => Self::Hunt,
            8 => Self::Return,
            _ => return None,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Needs {
    pub hunger: u8,
    pub energy: u8,
    pub fear: u8,
    pub age: u32,
    pub activity: Activity,
}
impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 30,
            energy: 100,
            fear: 0,
            age: 0,
            activity: Activity::Idle,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Population {
    /// Desired living counts. Dormant records are reconciled only on activation.
    pub counts: [u16; 4],
    pub food: u16,
    pub capacity: u16,
    pub epoch: u64,
    /// Monotonic birth slots; IDs are never reused, even after death.
    pub next_slot: u16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EcosystemEvent {
    CreatureSpawned(CreatureId),
    CreatureDied(CreatureId),
    CreatureAttacked(CreatureId, CreatureId),
    CreatureAte(CreatureId),
    ResourceConsumed(Point),
    PopulationChanged(Point, u16),
    WeatherChanged(u8),
}
#[derive(Default, Clone, Copy, Debug)]
pub struct Work {
    pub decisions: u64,
    pub queries: u64,
    pub path_expansions: u64,
    pub coarse_regions: u64,
    pub index_ns: u128,
    pub query_ns: u128,
    pub path_ns: u128,
    pub ai_ns: u128,
}
pub struct Wildlife {
    // Dense, ID-sorted game-facing records. The engine arena maps runtime handles
    // to dense indices; all occupancy and nearby queries use SpatialIndex.
    pub active: Vec<Creature>,
    pub memory: BTreeMap<CreatureId, CreatureState>,
    pub scanned: BTreeSet<CreatureId>,
    pub populations: BTreeMap<Point, Population>,
    pub time: u64,
    pub work: Work,
    pub events: terragame_core::EventQueue<EcosystemEvent>,
    pub event_overflows: u64,
    entities: terragame_core::EntityStore<usize>,
    handles: BTreeMap<CreatureId, terragame_core::EntityId>,
    spatial: terragame_world::SpatialIndex,
    regions: terragame_world::ActiveRegions,
    schedule: terragame_core::Scheduler,
}
impl Default for Wildlife {
    fn default() -> Self {
        let mut schedule = terragame_core::Scheduler::new(2);
        schedule.register(std::num::NonZeroU64::new(4).unwrap(), 0);
        schedule.register(std::num::NonZeroU64::new(240).unwrap(), 0);
        Self {
            active: Vec::new(),
            memory: BTreeMap::new(),
            scanned: BTreeSet::new(),
            populations: BTreeMap::new(),
            time: 0,
            work: Work::default(),
            events: terragame_core::EventQueue::new(4096),
            event_overflows: 0,
            entities: terragame_core::EntityStore::new(MAX_ACTIVE as u32),
            handles: BTreeMap::new(),
            spatial: terragame_world::SpatialIndex::new(
                terragame_world::ChunkLayout::new(16).unwrap(),
                MAX_ACTIVE,
            ),
            regions: terragame_world::ActiveRegions::new(2, 25).unwrap(),
            schedule,
        }
    }
}
fn region(id: CreatureId, camp: &Camp) -> Point {
    if (3..=5).contains(&id.2) {
        (camp.origin.0.div_euclid(32), camp.origin.1.div_euclid(32))
    } else {
        (id.0, id.1)
    }
}
fn category(b: Behaviour) -> usize {
    match b {
        Behaviour::Grazer => 0,
        Behaviour::Defender => 1,
        Behaviour::Predator => 2,
        Behaviour::Scavenger => 3,
    }
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
        Behaviour::Scavenger => ["carrion bird", "collector", "sifter", "crawler"],
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
        lifespan: 24_000,
        maturity: 600,
        birth_period: 240,
        territory: 12,
        temperature: match behaviour {
            Behaviour::Grazer => 35,
            _ => 50,
        },
    }
}
impl Wildlife {
    pub fn restore_regions(&mut self, player: Point) {
        self.regions.update(
            (player.0.div_euclid(32), player.1.div_euclid(32)),
            self.time,
        );
    }
    pub fn migrate_populations(&mut self, world: &mut World, camp: &Camp) {
        for state in self.memory.values() {
            let key = region(state.id, camp);
            let pop = self.populations.entry(key).or_insert(Population {
                counts: [0; 4],
                food: 120,
                capacity: REGION_CAPACITY,
                epoch: self.time / 240,
                next_slot: 32,
            });
            if state.health > 0
                && let Some(c) = Self::prototype(world, camp, state.id)
            {
                pop.counts[category(c.species.behaviour)] += 1;
            }
        }
    }
    pub fn validate_populations(&self, world: &mut World, camp: &Camp) -> Result<(), &'static str> {
        for s in self.memory.values() {
            let Some(p) = self.populations.get(&region(s.id, camp)) else {
                return Err("Creature has no population region");
            };
            if s.id.2 >= 32 && u16::from(s.id.2) >= p.next_slot {
                return Err("Birth ID exceeds region sequence");
            }
        }
        for c in &self.active {
            if self.memory.get(&c.state.id) != Some(&c.state) {
                return Err("Active creature differs from persistent state");
            }
        }
        // Dormant desired counts can differ until activation; active regions
        // must agree exactly with all retained living residents.
        let mut living = BTreeMap::<Point, [u16; 4]>::new();
        for s in self.memory.values().filter(|s| s.health > 0) {
            let c = Self::prototype(world, camp, s.id).ok_or("Unknown species reference")?;
            living.entry(region(s.id, camp)).or_default()[category(c.species.behaviour)] += 1;
        }
        for key in self.regions.keys() {
            if let Some(p) = self.populations.get(&key)
                && p.counts != living.get(&key).copied().unwrap_or_default()
            {
                return Err("Active population differs from residents");
            }
        }
        for id in self.scanned.iter().filter(|id| id.2 >= 32) {
            if !self.memory.contains_key(id) {
                return Err("Discovered birth has no persistent record");
            }
        }
        Ok(())
    }
    pub fn prototype(world: &mut World, camp: &Camp, id: CreatureId) -> Option<Creature> {
        let seed = world.seed();
        let h = hash(seed ^ 0x4352454154555245 ^ u64::from(id.2), id.0, id.1);
        let behaviour = if (3..=5).contains(&id.2) {
            match id.2 {
                3 => Behaviour::Grazer,
                4 => Behaviour::Defender,
                5 => Behaviour::Predator,
                _ => return None,
            }
        } else if id.2 < 3 {
            match h % 3 {
                0 => Behaviour::Grazer,
                1 => Behaviour::Defender,
                _ => Behaviour::Predator,
            }
        } else {
            if (18..32).contains(&id.2) {
                return None;
            }
            match h % 8 {
                0..=4 => Behaviour::Grazer,
                5 => Behaviour::Defender,
                6 => Behaviour::Predator,
                _ => Behaviour::Scavenger,
            }
        };
        let mut sp = species(seed, id, behaviour);
        sp.speed = sp
            .speed
            .saturating_add((world.profile().gravity / 120) as u8);
        let home = if (3..=5).contains(&id.2) {
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
                needs: Needs::default(),
            },
            home,
            species: sp,
        })
    }
    fn emit(&mut self, event: EcosystemEvent) {
        if self.events.push(event).is_err() {
            self.event_overflows = self.event_overflows.saturating_add(1);
        }
    }
    pub fn notify(&mut self, event: EcosystemEvent) {
        self.emit(event);
    }
    /// Reconcile runtime handles at mutation boundaries. Handles survive movement,
    /// but disappear on dormancy; corpses leave occupancy immediately.
    /// Handles never enter persistence.
    pub fn rebuild_index(&mut self) {
        let wanted: BTreeSet<_> = self.active.iter().map(|c| c.state.id).collect();
        let removed: Vec<_> = self
            .handles
            .keys()
            .filter(|id| !wanted.contains(id))
            .copied()
            .collect();
        for id in removed {
            let handle = self.handles.remove(&id).unwrap();
            self.spatial.remove(handle);
            self.entities.remove(handle);
        }
        for (index, c) in self.active.iter().enumerate() {
            let handle = *self
                .handles
                .entry(c.state.id)
                .or_insert_with(|| self.entities.spawn(index).expect("active limit"));
            *self.entities.get_mut(handle).unwrap() = index;
            if c.state.health > 0 {
                if !self.spatial.move_to(handle, c.state.position.into()) {
                    assert!(self.spatial.insert(handle, c.state.position.into()));
                }
            } else {
                self.spatial.remove(handle);
            }
        }
    }
    pub fn entity(&self, id: CreatureId) -> Option<terragame_core::EntityId> {
        self.handles.get(&id).copied()
    }
    pub fn entity_storage_bytes(&self) -> usize {
        self.entities.storage_bytes()
    }
    pub fn nearby(&self, p: Point, radius: u64) -> Vec<usize> {
        let mut found: Vec<_> = self
            .spatial
            .query_radius(p.into(), radius)
            .into_iter()
            .filter_map(|id| self.entities.get(id).copied())
            .collect();
        found.sort_unstable_by_key(|&i| self.active[i].state.id);
        found
    }
    pub fn refresh(&mut self, world: &mut World, camp: &Camp, player: Point) {
        self.regions.update(
            (player.0.div_euclid(32), player.1.div_euclid(32)),
            self.time,
        );
        // Every resident is retained, not just injured animals. At the record
        // ceiling new regions stay empty rather than respawning forgotten kills.
        for c in &self.active {
            self.memory.insert(c.state.id, c.state);
        }
        self.active.clear();
        let keys: Vec<_> = self.regions.keys().collect();
        for key in keys {
            if !self.populations.contains_key(&key) {
                if self.populations.len() >= MAX_REGIONS || self.memory.len() + 24 > MAX_MEMORY {
                    continue;
                }
                let capacity = (u16::from(world.profile().vegetation) / 5 + 6).min(REGION_CAPACITY);
                let mut counts = [0; 4];
                for slot in (0..3).chain(6..18) {
                    let id = (key.0, key.1, slot);
                    if counts.iter().sum::<u16>() >= capacity.saturating_sub(3) {
                        break;
                    }
                    if let Some(c) = Self::prototype(world, camp, id) {
                        if self
                            .memory
                            .values()
                            .any(|s| s.health > 0 && s.position == c.home)
                        {
                            continue;
                        }
                        counts[category(c.species.behaviour)] += 1;
                        if self.memory.insert(id, c.state).is_none() {
                            self.emit(EcosystemEvent::CreatureSpawned(id));
                        }
                    }
                }
                self.populations.insert(
                    key,
                    Population {
                        counts,
                        food: 120,
                        capacity,
                        epoch: self.time / 240,
                        next_slot: 32,
                    },
                );
            }
        }
        if !world.profile().legacy
            && self
                .regions
                .contains((camp.origin.0.div_euclid(32), camp.origin.1.div_euclid(32)))
            && self
                .populations
                .contains_key(&(camp.origin.0.div_euclid(32), camp.origin.1.div_euclid(32)))
        {
            for slot in 3..=5 {
                let id = (camp.origin.0, camp.origin.1, slot);
                if !self.memory.contains_key(&id)
                    && self.memory.len() < MAX_MEMORY
                    && let Some(c) = Self::prototype(world, camp, id)
                {
                    if let Some(pop) = self.populations.get_mut(&region(id, camp)) {
                        pop.counts[category(c.species.behaviour)] += 1;
                    }
                    self.memory.insert(id, c.state);
                }
            }
        }
        // Summaries are reconciled to exact records in persistent ID order.
        for key in self.regions.keys().collect::<Vec<_>>() {
            self.materialize(world, camp, key);
        }
        let mut occupied = BTreeSet::new();
        for state in self.memory.values() {
            if self.active.len() == MAX_ACTIVE {
                break;
            }
            if !self.regions.contains(region(state.id, camp))
                || distance(state.position, player) > i128::from(ACTIVE_RADIUS)
                || state.health == 0 && state.looted
            {
                continue;
            }
            if state.health > 0
                && (state.position == player
                    || !Self::walkable(world, camp, state.position)
                    || !occupied.insert(state.position))
            {
                continue;
            }
            if let Some(mut c) = Self::prototype(world, camp, state.id) {
                c.state = *state;
                self.active.push(c);
            }
        }
        self.active.sort_by_key(|c| c.state.id);
        self.rebuild_index();
    }
    fn materialize(&mut self, world: &mut World, camp: &Camp, key: Point) {
        let Some(mut pop) = self.populations.get(&key).copied() else {
            return;
        };
        let mut counts = [0u16; 4];
        for state in self
            .memory
            .values()
            .filter(|s| region(s.id, camp) == key && s.health > 0)
        {
            if let Some(c) = Self::prototype(world, camp, state.id) {
                counts[category(c.species.behaviour)] += 1;
            }
        }
        // Coarse losses select the highest IDs, preserving old discoveries.
        let mut deaths = Vec::new();
        for state in self
            .memory
            .values_mut()
            .rev()
            .filter(|s| region(s.id, camp) == key && s.health > 0)
        {
            if let Some(c) = Self::prototype(world, camp, state.id) {
                let k = category(c.species.behaviour);
                if counts[k] > pop.counts[k] {
                    state.health = 0;
                    deaths.push(state.id);
                    counts[k] -= 1;
                }
            }
        }
        for id in deaths {
            self.emit(EcosystemEvent::CreatureDied(id));
        }
        while counts.iter().sum::<u16>() < pop.counts.iter().sum::<u16>()
            && pop.next_slot <= 255
            && self.memory.len() < MAX_MEMORY
        {
            let id = (key.0, key.1, pop.next_slot as u8);
            pop.next_slot += 1;
            if let Some(c) = Self::prototype(world, camp, id) {
                let k = category(c.species.behaviour);
                if counts[k] < pop.counts[k]
                    && !self
                        .memory
                        .values()
                        .any(|s| s.health > 0 && s.position == c.home)
                {
                    self.memory.insert(id, c.state);
                    counts[k] += 1;
                    self.emit(EcosystemEvent::CreatureSpawned(id));
                }
            }
        }
        pop.counts = counts;
        self.populations.insert(key, pop);
    }
    /// One bounded operation per remembered region per coarse dispatch, never
    /// one operation per dormant creature. No wall-clock/offline simulation.
    pub fn advance_dormant(&mut self, tick: u64) {
        self.time = tick;
        let epoch = tick / 240;
        for (&key, pop) in &mut self.populations {
            if self.regions.contains(key) || epoch <= pop.epoch {
                continue;
            }
            let elapsed = epoch - pop.epoch;
            let previous = pop.epoch;
            pop.epoch = epoch;
            // Rare coarse mortality, indexed by absolute epoch, independent of
            // materialisation and terrain-cache order. Minimum breeding reserve.
            if epoch / 100 > previous / 100
                && pop.counts.iter().sum::<u16>() > 4
                && let Some(n) = pop.counts.iter_mut().rev().find(|n| **n > 0)
            {
                *n -= 1;
            }
            pop.food = pop.food.saturating_add(elapsed.min(120) as u16).min(120);
            let total: u16 = pop.counts.iter().sum();
            if total >= 2 && total < pop.capacity && pop.food >= 8 && pop.next_slot <= 255 {
                pop.counts[0] += 1;
                pop.food -= 8;
            } else if total > pop.capacity
                && let Some(n) = pop.counts.iter_mut().rev().find(|n| **n > 0)
            {
                *n -= 1;
            }
            self.work.coarse_regions += 1;
        }
    }
    fn walkable(world: &mut World, camp: &Camp, p: Point) -> bool {
        !camp.clearing(p.0, p.1)
            && !camp.sheltered(p)
            && camp.tile(world, p.0, p.1).biome.passable()
    }
    pub fn occupied(&self, p: Point) -> bool {
        // During construction/load callers may fill the dense records directly.
        if self.handles.len() != self.active.len() {
            return self
                .active
                .iter()
                .any(|c| c.state.health > 0 && c.state.position == p);
        }
        !self.spatial.query_radius(p.into(), 0).is_empty()
    }
    pub fn tick(
        &mut self,
        world: &mut World,
        camp: &mut Camp,
        player: Point,
        ticks: u64,
    ) -> (u8, Vec<String>) {
        self.time = ticks;
        let mut due = [false; 2];
        self.schedule
            .dispatch(ticks, |job| due[job.system.0] = true);
        if due[1] {
            self.advance_dormant(ticks);
        }
        if !due[0] {
            return (0, Vec::new());
        }
        let index_start = std::time::Instant::now();
        self.active.sort_by_key(|c| c.state.id);
        self.rebuild_index();
        self.work.index_ns += index_start.elapsed().as_nanos();
        let ai_start = std::time::Instant::now();
        let second = ticks / 4;
        let seed = world.seed();
        let refuge = camp.clearing(player.0, player.1) || camp.sheltered(player);
        let mut damage = 0u8;
        let mut messages = Vec::new();
        let mut path_budget = PATH_BUDGET;
        for i in 0..self.active.len() {
            if self.active[i].state.health == 0 {
                continue;
            }
            self.work.decisions += 1;
            let mut c = self.active[i].clone();
            let s = &mut c.state;
            let sp = &c.species;
            let needs = &mut s.needs;
            needs.age = needs.age.saturating_add(1);
            if second.is_multiple_of(8) {
                needs.hunger = needs.hunger.saturating_add(1).min(100);
            }
            needs.fear = needs.fear.saturating_sub(5);
            let dist = distance(s.position, player);
            let was_angry = s.angry;
            s.angry = !refuge
                && match sp.behaviour {
                    Behaviour::Grazer | Behaviour::Scavenger => false,
                    Behaviour::Defender => (dist <= 2 || s.angry) && distance(player, c.home) <= 6,
                    Behaviour::Predator => {
                        (dist <= i128::from(sp.detection) || s.angry)
                            && dist <= i128::from(sp.detection) + 4
                            && distance(player, c.home) <= 12
                            && needs.hunger >= 25
                    }
                };
            if s.angry && !was_angry {
                messages.push(format!(
                    "{} turns hostile! Retreat; F attacks adjacent wildlife.",
                    sp.name
                ));
            }
            if s.angry && dist <= 1 {
                damage = damage.saturating_add(sp.damage);
            }
            let biome = camp.tile(world, s.position.0, s.position.1).biome;
            let weather = Weather::at(
                seed,
                world.profile(),
                biome,
                s.position.0,
                s.position.1,
                ticks,
            );
            if needs.age >= sp.lifespan || second.is_multiple_of(32) && needs.hunger == 100 {
                s.health = s.health.saturating_sub(1);
            }
            if second.is_multiple_of(8)
                && ((weather.current.dangerous() && sp.resistance < weather.severity * 20)
                    || (world.profile().temperature - 18).abs() > sp.temperature
                        && biome != sp.preferred)
            {
                s.health = s.health.saturating_sub(1);
            }
            if s.health == 0 {
                self.active[i] = c;
                self.died(i, camp);
                continue;
            }
            let query_start = std::time::Instant::now();
            let nearby = self.nearby(s.position, u64::from(sp.detection));
            self.work.query_ns += query_start.elapsed().as_nanos();
            self.work.queries += 1;
            let threat = nearby
                .iter()
                .map(|&j| &self.active[j])
                .filter(|a| a.state.id != s.id && a.species.behaviour == Behaviour::Predator)
                .min_by_key(|a| (distance(s.position, a.state.position), a.state.id));
            let mut target = c.home;
            let mut flee = false;
            let was_resting = needs.activity == Activity::Rest;
            needs.activity = Activity::Wander;
            if matches!(sp.behaviour, Behaviour::Grazer | Behaviour::Scavenger)
                && (threat.is_some() || dist <= 4 && sp.behaviour == Behaviour::Grazer)
            {
                flee = true;
                needs.fear = 100;
                needs.activity = Activity::Flee;
                target = threat.map_or(player, |a| a.state.position);
            } else if s.angry {
                target = player;
                needs.activity = Activity::Defend;
            } else if needs.energy <= 20 || was_resting && needs.energy < 80 || needs.hunger < 15 {
                needs.activity = Activity::Rest;
                needs.energy = needs.energy.saturating_add(8).min(100);
            } else if distance(s.position, c.home) >= i128::from(sp.territory) - 2 {
                needs.activity = Activity::Return;
            } else if needs.hunger >= 35 {
                match sp.behaviour {
                    Behaviour::Predator => {
                        if let Some(&j) = nearby
                            .iter()
                            .filter(|&&j| {
                                matches!(
                                    self.active[j].species.behaviour,
                                    Behaviour::Grazer | Behaviour::Scavenger
                                )
                            })
                            .min_by_key(|&&j| {
                                (
                                    distance(s.position, self.active[j].state.position),
                                    self.active[j].state.id,
                                )
                            })
                        {
                            target = self.active[j].state.position;
                            needs.activity = Activity::Hunt;
                            if distance(s.position, target) <= 1 {
                                let prey_id = self.active[j].state.id;
                                self.active[j].state.health =
                                    self.active[j].state.health.saturating_sub(sp.damage);
                                self.active[j].state.needs.fear = 100;
                                self.memory.insert(prey_id, self.active[j].state);
                                self.emit(EcosystemEvent::CreatureAttacked(s.id, prey_id));
                                if self.active[j].state.health == 0 {
                                    self.died(j, camp);
                                }
                            }
                        }
                    }
                    Behaviour::Scavenger => {
                        if let Some(j) = self
                            .active
                            .iter()
                            .enumerate()
                            .filter(|(_, a)| {
                                a.state.health == 0
                                    && !a.state.looted
                                    && distance(s.position, a.state.position) <= 10
                            })
                            .min_by_key(|(_, a)| {
                                (distance(s.position, a.state.position), a.state.id)
                            })
                            .map(|(j, _)| j)
                        {
                            target = self.active[j].state.position;
                            needs.activity = Activity::Forage;
                            if distance(s.position, target) <= 1 {
                                self.active[j].state.looted = true;
                                self.memory
                                    .insert(self.active[j].state.id, self.active[j].state);
                                needs.hunger = needs.hunger.saturating_sub(60);
                                needs.activity = Activity::Eat;
                                self.emit(EcosystemEvent::CreatureAte(s.id));
                            }
                        }
                    }
                    _ => {
                        needs.activity = Activity::Forage;
                        let mut food = Vec::new();
                        for dy in -4..=4 {
                            for dx in -4..=4 {
                                let (Some(x), Some(y)) =
                                    (s.position.0.checked_add(dx), s.position.1.checked_add(dy))
                                else {
                                    continue;
                                };
                                if let Some(crate::world::landmark::Landmark::Plant(kind)) =
                                    camp.tile(world, x, y).landmark
                                    && kind != crate::world::flora::Flora::Toxic
                                {
                                    food.push((distance(s.position, (x, y)), (x, y)));
                                }
                            }
                        }
                        food.sort_unstable();
                        if let Some(&(d, p)) = food.first() {
                            target = p;
                            if d <= 1 && camp.plants.len() < 100_000 {
                                camp.plants.insert(p);
                                needs.hunger = needs.hunger.saturating_sub(45);
                                needs.activity = Activity::Eat;
                                self.emit(EcosystemEvent::ResourceConsumed(p));
                                self.emit(EcosystemEvent::CreatureAte(s.id));
                            }
                        } else if matches!(
                            biome,
                            crate::world::biome::Biome::Grassland
                                | crate::world::biome::Biome::Forest
                        ) && let Some(pop) = self.populations.get_mut(&region(s.id, camp))
                            && pop.food > 0
                        {
                            pop.food -= 1;
                            needs.hunger = needs.hunger.saturating_sub(25);
                            needs.activity = Activity::Eat;
                            self.emit(EcosystemEvent::CreatureAte(s.id));
                        }
                    }
                }
                // Predators also consume remains from their own kills.
                if sp.behaviour == Behaviour::Predator
                    && let Some(j) = self.active.iter().position(|a| {
                        a.state.health == 0
                            && !a.state.looted
                            && distance(s.position, a.state.position) <= 1
                    })
                {
                    self.active[j].state.looted = true;
                    self.memory
                        .insert(self.active[j].state.id, self.active[j].state);
                    needs.hunger = needs.hunger.saturating_sub(65);
                    needs.activity = Activity::Eat;
                    self.emit(EcosystemEvent::CreatureAte(s.id));
                }
            }
            if s.health > 0
                && !matches!(needs.activity, Activity::Eat | Activity::Rest)
                && second.is_multiple_of(u64::from(sp.speed))
            {
                let mut moves = Vec::new();
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    let (Some(x), Some(y)) =
                        (s.position.0.checked_add(dx), s.position.1.checked_add(dy))
                    else {
                        continue;
                    };
                    let p = (x, y);
                    if p == player
                        || self.occupied(p)
                        || distance(p, c.home) > i128::from(sp.territory)
                        || !Self::walkable(world, camp, p)
                    {
                        continue;
                    }
                    let rank = if flee {
                        -distance(p, target)
                    } else if needs.activity != Activity::Wander || distance(s.position, c.home) > 2
                    {
                        distance(p, target)
                    } else {
                        i128::from(hash(seed ^ ticks ^ u64::from(s.id.2), x, y) % 16)
                            + if camp.tile(world, x, y).biome == sp.preferred {
                                0
                            } else {
                                3
                            }
                    };
                    moves.push((rank, p));
                }
                moves.sort_unstable();
                let mut next = moves.first().map(|&(_, p)| p);
                if !flee
                    && needs.activity != Activity::Wander
                    && next.is_none_or(|p| distance(p, target) >= distance(s.position, target))
                {
                    let mut budget = path_budget.min(64);
                    let initial = budget;
                    let path_start = std::time::Instant::now();
                    // A target occupied by prey/player is approached via a
                    // neighbouring cell rather than an impossible occupied goal.
                    let goal = if target == player || self.occupied(target) {
                        [(0, -1), (1, 0), (0, 1), (-1, 0)]
                            .into_iter()
                            .filter_map(|(dx, dy)| {
                                let p = (target.0.checked_add(dx)?, target.1.checked_add(dy)?);
                                (p != player && !self.occupied(p) && Self::walkable(world, camp, p))
                                    .then_some(p)
                            })
                            .min_by_key(|&p| (distance(s.position, p), p))
                            .unwrap_or(target)
                    } else {
                        target
                    };
                    let path = terragame_world::navigation::local_path(
                        s.position.into(),
                        goal.into(),
                        12,
                        &mut budget,
                        |p| {
                            let p: Point = p.into();
                            p != player
                                && !self.occupied(p)
                                && distance(p, c.home) <= i128::from(sp.territory)
                                && Self::walkable(world, camp, p)
                        },
                    );
                    self.work.path_ns += path_start.elapsed().as_nanos();
                    path_budget -= initial - budget;
                    self.work.path_expansions += path.expanded as u64;
                    if let Some(p) = path.steps.first() {
                        next = Some((*p).into());
                    }
                }
                if let Some(p) = next {
                    s.position = p;
                    needs.energy = needs.energy.saturating_sub(1);
                    self.spatial.move_to(self.handles[&s.id], p.into());
                } else {
                    needs.activity = Activity::Idle;
                }
            }
            let died = c.state.health == 0;
            self.memory.insert(c.state.id, c.state);
            self.active[i] = c;
            if died {
                self.died(i, camp);
            }
        }
        self.work.ai_ns += ai_start.elapsed().as_nanos();
        if due[1] {
            self.reproduce(world, camp, player);
        }
        (damage, messages)
    }
    fn died(&mut self, i: usize, camp: &Camp) {
        let s = self.active[i].state;
        self.memory.insert(s.id, s);
        if let Some(handle) = self.handles.get(&s.id) {
            self.spatial.remove(*handle);
        }
        if let Some(pop) = self.populations.get_mut(&region(s.id, camp)) {
            let k = category(self.active[i].species.behaviour);
            pop.counts[k] = pop.counts[k].saturating_sub(1);
        }
        self.emit(EcosystemEvent::CreatureDied(s.id));
    }
    fn reproduce(&mut self, world: &mut World, camp: &Camp, player: Point) {
        // One birth per region per minute, two fed mature parents, exact caps.
        let keys: Vec<_> = self.regions.keys().collect();
        for key in keys {
            let Some(mut pop) = self.populations.get(&key).copied() else {
                continue;
            };
            pop.epoch = self.time / 240;
            pop.food = pop.food.saturating_add(4).min(120);
            let parents = self.active.iter().filter(|c| {
                region(c.state.id, camp) == key
                    && c.state.health > 0
                    && c.state.needs.age >= c.species.maturity
                    && c.state.needs.hunger < 50
                    && c.state.needs.energy > 30
                    && self.time.is_multiple_of(c.species.birth_period)
            });
            let mut parent_counts = [0u16; 4];
            for c in parents {
                parent_counts[category(c.species.behaviour)] += 1;
            }
            if parent_counts.iter().any(|&n| n >= 2)
                && pop.counts.iter().sum::<u16>() < pop.capacity
                && pop.food >= 8
                && self.active.len() < MAX_ACTIVE
                && self.memory.len() < MAX_MEMORY
            {
                while pop.next_slot <= 255 {
                    let id = (key.0, key.1, pop.next_slot as u8);
                    pop.next_slot += 1;
                    if let Some(c) = Self::prototype(world, camp, id)
                        && parent_counts[category(c.species.behaviour)] >= 2
                        && !self.occupied(c.home)
                        && distance(c.home, player) <= i128::from(ACTIVE_RADIUS)
                        && Self::walkable(world, camp, c.home)
                        && c.home != player
                        && !self
                            .memory
                            .values()
                            .any(|s| s.health > 0 && s.position == c.home)
                    {
                        pop.counts[category(c.species.behaviour)] += 1;
                        pop.food -= 8;
                        self.memory.insert(id, c.state);
                        self.active.push(c);
                        self.emit(EcosystemEvent::CreatureSpawned(id));
                        self.emit(EcosystemEvent::PopulationChanged(
                            key,
                            pop.counts.iter().sum(),
                        ));
                        break;
                    }
                }
            }
            self.populations.insert(key, pop);
        }
        self.active.sort_by_key(|c| c.state.id);
        self.rebuild_index();
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
        c.state.angry = matches!(
            c.species.behaviour,
            Behaviour::Defender | Behaviour::Predator
        );
        c.state.needs.fear = 100;
        self.memory.insert(c.state.id, c.state);
        if c.state.health == 0 {
            if let Some(h) = self.handles.get(&c.state.id) {
                self.spatial.remove(*h);
            }
            // Camp IDs 3..5 use the camp world coordinates as their identity.
            let key = if (3..=5).contains(&c.state.id.2) {
                (c.state.id.0.div_euclid(32), c.state.id.1.div_euclid(32))
            } else {
                (c.state.id.0, c.state.id.1)
            };
            if let Some(pop) = self.populations.get_mut(&key) {
                let k = category(c.species.behaviour);
                pop.counts[k] = pop.counts[k].saturating_sub(1);
            }
        }
        let result = if c.state.health == 0 {
            format!(
                "{} defeated. Stand on x and E to recover organics.",
                c.species.name
            )
        } else {
            format!(
                "Hit {}: {} health remaining.",
                c.species.name, c.state.health
            )
        };
        let id = c.state.id;
        if c.state.health == 0 {
            self.emit(EcosystemEvent::CreatureDied(id));
        }
        Ok(result)
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
            && !(state.angry
                && matches!(
                    c.species.behaviour,
                    Behaviour::Grazer | Behaviour::Scavenger
                ))
            && state.needs.hunger <= 100
            && state.needs.energy <= 100
            && state.needs.fear <= 100
    }
}
