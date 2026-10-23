//! Mutable expedition effects overlay, never edit, deterministic base terrain.
use crate::world::flora::Flora;
use crate::{
    entities::player::Player,
    inventory::{Inventory, Item},
    world::{
        World,
        biome::Biome,
        landmark::{self, Landmark, Resource, Site},
        terrain::Tile,
    },
};
use std::collections::{BTreeMap, BTreeSet};
pub type Point = (i64, i64);
pub const DEPOSIT_UNITS: u8 = 12;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shelter {
    pub position: Point,
    pub charge: u16,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Camp {
    pub origin: Point,
    pub cache_claimed: bool,
    pub ship_charge: u16,
    pub shelters: Vec<Shelter>,
    pub harvested: BTreeMap<Point, u8>,
    pub night_progress: u16,
    pub completed: bool,
    pub plants: BTreeSet<Point>,
    pub looted_ruins: BTreeSet<Point>,
}
pub fn near(a: Point, b: Point) -> bool {
    (i128::from(a.0) - i128::from(b.0)).abs() + (i128::from(a.1) - i128::from(b.1)).abs() <= 1
}
impl Camp {
    pub fn new(origin: Point) -> Self {
        Self {
            origin,
            cache_claimed: false,
            ship_charge: 0,
            shelters: Vec::new(),
            harvested: BTreeMap::new(),
            night_progress: 0,
            completed: false,
            plants: BTreeSet::new(),
            looted_ruins: BTreeSet::new(),
        }
    }
    pub fn offset(&self, dx: i64, dy: i64) -> Point {
        (
            self.origin.0.saturating_add(dx),
            self.origin.1.saturating_add(dy),
        )
    }
    pub fn trail(&self, x: i64, y: i64) -> bool {
        (5..=16).contains(&(i128::from(x) - i128::from(self.origin.0)))
            && (i128::from(y) - i128::from(self.origin.1)).abs() <= 1
    }
    pub fn deposit_capacity(&self, world: &World, p: Point) -> u8 {
        if self.clearing(p.0, p.1) || world.profile().legacy {
            DEPOSIT_UNITS
        } else {
            (u32::from(world.profile().minerals) * 12 / 100).clamp(6, 24) as u8
        }
    }
    pub fn ship(&self) -> Point {
        (self.origin.0.saturating_add(1), self.origin.1)
    }
    pub fn clearing(&self, x: i64, y: i64) -> bool {
        (i128::from(x) - i128::from(self.origin.0))
            .abs()
            .max((i128::from(y) - i128::from(self.origin.1)).abs())
            <= 4
    }
    pub fn starter_nodes(&self) -> [(Point, Resource); 3] {
        [
            (
                (self.origin.0.saturating_sub(2), self.origin.1),
                Resource::Iron,
            ),
            (
                (self.origin.0, self.origin.1.saturating_sub(2)),
                Resource::Carbon,
            ),
            (
                (
                    self.origin.0.saturating_add(2),
                    self.origin.1.saturating_add(2),
                ),
                Resource::Crystal,
            ),
        ]
    }
    pub(crate) fn initial_tile(&self, world: &mut World, x: i64, y: i64) -> Tile {
        let mut tile = world.tile(x, y);
        if self.clearing(x, y) {
            tile = Tile {
                biome: Biome::Grassland,
                landmark: None,
            };
            if let Some((_, resource)) = self.starter_nodes().iter().find(|(p, _)| *p == (x, y)) {
                tile.landmark = Some(Landmark::Deposit(*resource));
            }
        }
        if !world.profile().legacy && self.trail(x, y) {
            tile = Tile {
                biome: Biome::Grassland,
                landmark: None,
            };
        }
        if !self.clearing(x, y) && tile.landmark.is_none() {
            tile.landmark =
                Flora::at(world.seed(), x, y, tile.biome, world.profile()).map(Landmark::Plant);
        }
        if !world.profile().legacy {
            if (x, y) == self.offset(5, 0) {
                tile.landmark = Some(Landmark::Plant(Flora::Medicinal));
            }
            if (x, y) == self.offset(7, 1) {
                tile.landmark = Some(Landmark::Plant(Flora::Fungus));
            }
            if (x, y) == self.offset(9, -1) {
                tile.landmark = Some(Landmark::Plant(Flora::Toxic));
            }
            if (x, y) == self.offset(12, 0) {
                tile.landmark = Some(Landmark::Ruin);
            }
        }
        tile
    }
    pub fn tile(&self, world: &mut World, x: i64, y: i64) -> Tile {
        let mut tile = self.initial_tile(world, x, y);
        if self
            .harvested
            .get(&(x, y))
            .is_some_and(|&n| n >= self.deposit_capacity(world, (x, y)))
            && matches!(tile.landmark, Some(Landmark::Deposit(_)))
        {
            tile.landmark = None;
        }
        if self.plants.contains(&(x, y)) && matches!(tile.landmark, Some(Landmark::Plant(_))) {
            tile.landmark = None;
        }
        if world.profile().legacy
            && self.sheltered((x, y))
            && matches!(tile.landmark, Some(Landmark::Plant(_)))
        {
            tile.landmark = None;
        }
        tile
    }
    pub fn site(&self, world: &mut World, x: i64, y: i64) -> Option<Site> {
        let kind = self.tile(world, x, y).landmark?;
        if self.clearing(x, y)
            || matches!(kind, Landmark::Plant(_))
            || !world.profile().legacy && (x, y) == self.offset(12, 0)
        {
            Some(Site { x, y, kind })
        } else {
            landmark::at(world.seed(), x, y)
        }
    }
    pub fn sheltered(&self, p: Point) -> bool {
        self.shelters.iter().any(|s| s.position == p)
    }
    pub fn power_remaining(&self, p: Point) -> u16 {
        let ship = if near(p, self.ship()) {
            self.ship_charge
        } else {
            0
        };
        self.shelters
            .iter()
            .filter(|s| near(p, s.position))
            .map(|s| s.charge)
            .max()
            .unwrap_or(0)
            .max(ship)
    }
    pub fn powered(&self, p: Point) -> bool {
        self.power_remaining(p) > 0
    }
    pub fn spend_power(&mut self) {
        self.ship_charge = self.ship_charge.saturating_sub(1);
        for s in &mut self.shelters {
            s.charge = s.charge.saturating_sub(1);
        }
    }
    pub fn scavenge(&mut self, inventory: &mut Inventory) -> Result<(), &'static str> {
        if self.cache_claimed {
            return Err("Cache already collected; use a battery to power oxygen");
        }
        inventory.transact(
            &[],
            &[
                (Item::BasicPick, 1),
                (Item::OxygenCanister, 2),
                (Item::Battery, 1),
            ],
        )?;
        self.cache_claimed = true;
        Ok(())
    }
    pub fn harvest(
        &mut self,
        world: &mut World,
        inventory: &mut Inventory,
        p: Point,
    ) -> Result<(Resource, u8), &'static str> {
        let Some(Landmark::Deposit(resource)) = self.tile(world, p.0, p.1).landmark else {
            return Err("Stand on i/k/v/c/s and press E to harvest");
        };
        if resource == Resource::Crystal && inventory.count(Item::MiningTool) == 0 {
            return Err("Crystal requires a crafted mining tool (C)");
        }
        if resource != Resource::Carbon
            && inventory.count(Item::BasicPick) + inventory.count(Item::MiningTool) == 0
        {
            return Err("Scavenge the spacecraft for a salvage pick first (E)");
        }
        let used = self.harvested.get(&p).copied().unwrap_or(0);
        let amount = 2.min(self.deposit_capacity(world, p) - used);
        inventory.add(resource.into(), amount)?;
        self.harvested.insert(p, used + amount);
        Ok((resource, amount))
    }
    pub fn harvest_plant(
        &mut self,
        world: &mut World,
        inventory: &mut Inventory,
        p: Point,
    ) -> Result<Flora, &'static str> {
        let Some(Landmark::Plant(plant)) = self.tile(world, p.0, p.1).landmark else {
            return Err("No unharvested plant here");
        };
        if self.plants.len() >= 100_000 {
            return Err("Plant record limit reached");
        }
        let output = if plant == Flora::Medicinal {
            Item::Herbs
        } else {
            Item::Organics
        };
        inventory.add(output, 2)?;
        self.plants.insert(p);
        Ok(plant)
    }
    pub fn loot_ruin(
        &mut self,
        world: &mut World,
        inventory: &mut Inventory,
        p: Point,
    ) -> Result<String, &'static str> {
        let Some(site) = self
            .site(world, p.0, p.1)
            .filter(|s| s.kind == Landmark::Ruin)
        else {
            return Err("Stand on # to explore a ruin");
        };
        let anchor = (site.x, site.y);
        if self.looted_ruins.contains(&anchor) {
            return Err("Ruin already salvaged; its planetary record is in M survey");
        }
        if self.looted_ruins.len() >= 10_000 {
            return Err("Ruin record limit reached");
        }
        let h = crate::world::seed::hash(world.seed() ^ 0x4c4f4f54, site.x, site.y);
        let supplies = if h & 1 == 0 {
            Item::OxygenCanister
        } else {
            Item::Battery
        };
        inventory.transact(
            &[],
            &[(Item::Technology, 1), (supplies, 1), (Item::Crystal, 1)],
        )?;
        self.looted_ruins.insert(anchor);
        Ok(format!(
            "Ruin salvage: technology, {}, crystal. Record: {} / {}.",
            supplies.name(),
            world.profile().archetype.name(),
            world.profile().atmosphere()
        ))
    }
    pub fn build(
        &mut self,
        world: &mut World,
        inventory: &mut Inventory,
        p: Point,
    ) -> Result<(), &'static str> {
        if p == self.ship()
            || self.sheltered(p)
            || self.tile(world, p.0, p.1).landmark.is_some()
            || !self.tile(world, p.0, p.1).biome.passable()
        {
            return Err("Build on clear walkable ground away from sites");
        }
        if self.shelters.len() >= 1000 {
            return Err("Shelter limit reached");
        }
        inventory.remove(Item::ShelterKit, 1)?;
        self.shelters.push(Shelter {
            position: p,
            charge: 0,
        });
        Ok(())
    }
    pub fn power_station(
        &mut self,
        inventory: &mut Inventory,
        p: Point,
    ) -> Result<(), &'static str> {
        let charge = if let Some(s) = self.shelters.iter_mut().find(|s| near(p, s.position)) {
            &mut s.charge
        } else if near(p, self.ship()) {
            &mut self.ship_charge
        } else {
            return Err("Stand beside A or H to use a battery");
        };
        if *charge > 480 {
            return Err("Station has enough power; battery preserved");
        }
        inventory.remove(Item::Battery, 1)?;
        *charge += 120;
        Ok(())
    }
    pub fn point(player: Player) -> Point {
        (player.x, player.y)
    }
}
