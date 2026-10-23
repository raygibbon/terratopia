//! Expedition knowledge is never owned by the evictable terrain cache.
use crate::world::{
    World,
    chunk::{ChunkKey, locate},
    landmark::{self, Site},
};
use std::collections::BTreeMap;
pub const SIGHT_RADIUS: i64 = 7;
pub const SCAN_RADIUS: i64 = 20;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Unknown,
    Remembered,
    Visible,
}
#[derive(Default)]
pub struct Exploration {
    pub(crate) explored: BTreeMap<ChunkKey, [u32; 32]>,
    pub(crate) sites: BTreeMap<(i64, i64), Site>,
    cells: usize,
    center: (i64, i64),
    radius: i64,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanReport {
    pub new_cells: usize,
    pub new_sites: usize,
    pub sites: Vec<Site>,
}
impl Exploration {
    pub(crate) fn recount(&mut self) {
        self.cells = self
            .explored
            .values()
            .flat_map(|rows| rows.iter())
            .map(|row| row.count_ones() as usize)
            .sum();
    }
    pub fn explored_cells(&self) -> usize {
        self.cells
    }
    pub fn discovered_sites(&self) -> usize {
        self.sites.len()
    }
    pub fn visibility(&self, x: i64, y: i64) -> Visibility {
        let (key, index) = locate(x, y);
        if self
            .explored
            .get(&key)
            .is_none_or(|bits| bits[index / 32] & (1 << (index % 32)) == 0)
        {
            return Visibility::Unknown;
        }
        let dx = i128::from(x) - i128::from(self.center.0);
        let dy = i128::from(y) - i128::from(self.center.1);
        let r = i128::from(self.radius);
        if dx.abs() <= r && dy.abs() <= r && dx * dx + dy * dy <= r * r {
            Visibility::Visible
        } else {
            Visibility::Remembered
        }
    }
    /// Scan visibility lasts until the next successful move. Terrain does not
    /// occlude the sensor; mountains and water remain impassable.
    pub fn reveal(&mut self, world: &mut World, x: i64, y: i64, scan: bool) -> ScanReport {
        self.reveal_with(x, y, scan, |wx, wy| {
            world
                .tile(wx, wy)
                .landmark
                .and_then(|_| landmark::at(world.seed(), wx, wy))
        })
    }
    pub fn reveal_with(
        &mut self,
        x: i64,
        y: i64,
        scan: bool,
        query: impl FnMut(i64, i64) -> Option<Site>,
    ) -> ScanReport {
        self.reveal_radius(x, y, if scan { SCAN_RADIUS } else { SIGHT_RADIUS }, query)
    }
    pub fn reveal_radius(
        &mut self,
        x: i64,
        y: i64,
        radius: i64,
        mut query: impl FnMut(i64, i64) -> Option<Site>,
    ) -> ScanReport {
        self.center = (x, y);
        self.radius = radius.clamp(1, SCAN_RADIUS);
        let old_cells = self.cells;
        let old_sites = self.sites.len();
        let mut nearby = BTreeMap::new();
        for dy in -self.radius..=self.radius {
            for dx in -self.radius..=self.radius {
                if dx * dx + dy * dy > self.radius * self.radius {
                    continue;
                }
                let (Some(wx), Some(wy)) = (x.checked_add(dx), y.checked_add(dy)) else {
                    continue;
                };
                let (key, index) = locate(wx, wy);
                let bits = self.explored.entry(key).or_insert([0; 32]);
                let mask = 1 << (index % 32);
                if bits[index / 32] & mask == 0 {
                    self.cells += 1;
                    bits[index / 32] |= mask;
                }
                if let Some(site) = query(wx, wy) {
                    self.sites.insert((site.x, site.y), site);
                    nearby.insert((site.x, site.y), site);
                }
            }
        }
        let mut sites: Vec<_> = nearby.into_values().collect();
        sites.sort_by_key(|site| {
            let dx = i128::from(site.x) - i128::from(x);
            let dy = i128::from(site.y) - i128::from(y);
            (dx * dx + dy * dy, site.x, site.y)
        });
        ScanReport {
            new_cells: self.cells - old_cells,
            new_sites: self.sites.len() - old_sites,
            sites,
        }
    }
}
