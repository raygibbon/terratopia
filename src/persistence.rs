//! Strict versioned text saves. Parse completely before replacing a live game.
use crate::{
    game::{Game, State},
    inventory::{Inventory, Item, Stack},
    planet::Shelter,
    survival::{NIGHT_TICKS, Survival},
    world::landmark::{Landmark, Resource, Site},
};
use std::{
    fmt::Write as _,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::Path,
};
const LIMIT: u64 = 8 * 1024 * 1024;
const HEADER: &str = "TERRATOPIA_SAVE 3";
fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
fn kind_id(kind: Landmark) -> u8 {
    match kind {
        Landmark::Ruin => 0,
        Landmark::Plant(p) => match p {
            crate::world::flora::Flora::Fern => 8,
            crate::world::flora::Flora::Fungus => 9,
            crate::world::flora::Flora::Toxic => 10,
            crate::world::flora::Flora::Medicinal => 11,
        },
        Landmark::Arch => 1,
        Landmark::Crater => 2,
        Landmark::Deposit(r) => match r {
            Resource::Iron => 3,
            Resource::Copper => 4,
            Resource::Silica => 5,
            Resource::Carbon => 6,
            Resource::Crystal => 7,
        },
    }
}
fn kind(id: u8) -> io::Result<Landmark> {
    Ok(match id {
        0 => Landmark::Ruin,
        1 => Landmark::Arch,
        2 => Landmark::Crater,
        3 => Landmark::Deposit(Resource::Iron),
        4 => Landmark::Deposit(Resource::Copper),
        5 => Landmark::Deposit(Resource::Silica),
        6 => Landmark::Deposit(Resource::Carbon),
        7 => Landmark::Deposit(Resource::Crystal),
        8 => Landmark::Plant(crate::world::flora::Flora::Fern),
        9 => Landmark::Plant(crate::world::flora::Flora::Fungus),
        10 => Landmark::Plant(crate::world::flora::Flora::Toxic),
        11 => Landmark::Plant(crate::world::flora::Flora::Medicinal),
        _ => return Err(invalid("Unknown discovery kind")),
    })
}
pub fn encode(game: &Game) -> io::Result<String> {
    if !game.started {
        return Err(invalid("Land before saving an expedition"));
    }
    if game.camp.shelters.len() > 1000
        || game.camp.harvested.len() > 100_000
        || game.exploration.explored.len() > 20_000
        || game.exploration.sites.len() > 100_000
        || game.camp.plants.len() > 100_000
        || game.camp.looted_ruins.len() > 10_000
        || game.wildlife.active.len() > crate::wildlife::MAX_ACTIVE
        || game.wildlife.memory.len() > crate::wildlife::MAX_MEMORY
        || game.wildlife.scanned.len() > 100_000
        || game.wildlife.populations.len() > crate::wildlife::MAX_REGIONS
    {
        return Err(invalid("Save entry limit exceeded"));
    }
    let mut out = format!(
        "{HEADER}\nGENERATOR {}\nSEED {}\nPLAYER {} {}\nVITALS {} {} {} {}\nCAMP {} {} {} {}\n",
        game.world.generator(),
        game.world.seed(),
        game.player.x,
        game.player.y,
        game.survival.health,
        game.survival.oxygen,
        game.survival.energy,
        game.survival.ticks(),
        u8::from(game.camp.cache_claimed),
        game.camp.ship_charge,
        game.camp.night_progress,
        u8::from(game.camp.completed)
    );
    writeln!(out, "INVENTORY {}", game.inventory.stacks().len()).unwrap();
    for stack in game.inventory.stacks() {
        writeln!(out, "ITEM {} {}", stack.item.id(), stack.count).unwrap();
    }
    writeln!(out, "SHELTERS {}", game.camp.shelters.len()).unwrap();
    for s in &game.camp.shelters {
        writeln!(
            out,
            "SHELTER {} {} {}",
            s.position.0, s.position.1, s.charge
        )
        .unwrap();
    }
    writeln!(out, "HARVESTED {}", game.camp.harvested.len()).unwrap();
    for (&(x, y), &n) in &game.camp.harvested {
        writeln!(out, "NODE {x} {y} {n}").unwrap();
    }
    writeln!(out, "FOG {}", game.exploration.explored.len()).unwrap();
    for (&(x, y), rows) in &game.exploration.explored {
        write!(out, "MASK {x} {y}").unwrap();
        for n in rows {
            write!(out, " {n}").unwrap();
        }
        out.push('\n');
    }
    writeln!(out, "DISCOVERIES {}", game.exploration.sites.len()).unwrap();
    for site in game.exploration.sites.values() {
        writeln!(out, "SITE {} {} {}", site.x, site.y, kind_id(site.kind)).unwrap();
    }
    writeln!(
        out,
        "LIVING {} {}",
        game.filter_ticks,
        u8::from(game.resting)
    )
    .unwrap();
    writeln!(out, "PLANTS {}", game.camp.plants.len()).unwrap();
    for &(x, y) in &game.camp.plants {
        writeln!(out, "PLANT {x} {y}").unwrap();
    }
    writeln!(out, "RUINS {}", game.camp.looted_ruins.len()).unwrap();
    for &(x, y) in &game.camp.looted_ruins {
        writeln!(out, "RUIN {x} {y}").unwrap();
    }
    writeln!(out, "MEMORY {}", game.wildlife.memory.len()).unwrap();
    for state in game.wildlife.memory.values() {
        write_actor(&mut out, "CREATURE", *state);
    }
    writeln!(out, "ACTORS {}", game.wildlife.active.len()).unwrap();
    for creature in &game.wildlife.active {
        write_actor(&mut out, "ACTOR", creature.state);
    }
    writeln!(out, "SCANNED {}", game.wildlife.scanned.len()).unwrap();
    for &(x, y, slot) in &game.wildlife.scanned {
        writeln!(out, "ID {x} {y} {slot}").unwrap();
    }
    writeln!(out, "ECOSYSTEM {}", game.survival.ticks()).unwrap();
    writeln!(out, "POPULATIONS {}", game.wildlife.populations.len()).unwrap();
    for (&(x, y), p) in &game.wildlife.populations {
        writeln!(
            out,
            "POPULATION {x} {y} {} {} {} {} {} {} {} {}",
            p.counts[0],
            p.counts[1],
            p.counts[2],
            p.counts[3],
            p.food,
            p.capacity,
            p.epoch,
            p.next_slot
        )
        .unwrap();
    }
    out.push_str("END\n");
    if out.len() as u64 > LIMIT {
        return Err(invalid("Save exceeds 8 MiB limit"));
    }
    Ok(out)
}
fn write_actor(out: &mut String, name: &str, state: crate::wildlife::CreatureState) {
    writeln!(
        out,
        "{name} {} {} {} {} {} {} {} {} {} {} {} {} {}",
        state.id.0,
        state.id.1,
        state.id.2,
        state.position.0,
        state.position.1,
        state.health,
        u8::from(state.angry),
        u8::from(state.looted),
        state.needs.hunger,
        state.needs.energy,
        state.needs.fear,
        state.needs.age,
        state.needs.activity as u8
    )
    .unwrap();
}
fn read_actor(
    parser: &mut Parser<'_>,
    name: &str,
    version: u8,
) -> io::Result<crate::wildlife::CreatureState> {
    let r = parser.row(name, if version >= 3 { 13 } else { 8 })?;
    Ok(crate::wildlife::CreatureState {
        id: (num(r[0])?, num(r[1])?, num(r[2])?),
        position: (num(r[3])?, num(r[4])?),
        health: num(r[5])?,
        angry: boolean(r[6])?,
        looted: boolean(r[7])?,
        needs: if version >= 3 {
            crate::wildlife::Needs {
                hunger: num(r[8])?,
                energy: num(r[9])?,
                fear: num(r[10])?,
                age: num(r[11])?,
                activity: crate::wildlife::Activity::from_id(num(r[12])?)
                    .ok_or_else(|| invalid("Unknown creature activity"))?,
            }
        } else {
            crate::wildlife::Needs::default()
        },
    })
}
struct Parser<'a> {
    lines: std::str::Lines<'a>,
}
impl<'a> Parser<'a> {
    fn row(&mut self, name: &str, fields: usize) -> io::Result<Vec<&'a str>> {
        let parts: Vec<_> = self
            .lines
            .next()
            .ok_or_else(|| invalid("Truncated save"))?
            .split_whitespace()
            .collect();
        if parts.first() != Some(&name) || parts.len() != fields + 1 {
            return Err(invalid(format!("Invalid {name} record")));
        }
        Ok(parts[1..].to_vec())
    }
    fn count(&mut self, name: &str, max: usize) -> io::Result<usize> {
        let n = num(self.row(name, 1)?[0])?;
        if n > max {
            Err(invalid(format!("Too many {name} entries")))
        } else {
            Ok(n)
        }
    }
}
fn num<T: std::str::FromStr>(s: &str) -> io::Result<T> {
    s.parse().map_err(|_| invalid("Invalid numeric save field"))
}
fn boolean(s: &str) -> io::Result<bool> {
    match s {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(invalid("Invalid boolean")),
    }
}
pub fn decode(text: &str) -> io::Result<Game> {
    if text.len() as u64 > LIMIT {
        return Err(invalid("Save exceeds 8 MiB limit"));
    }
    let mut parser = Parser {
        lines: text.lines(),
    };
    let version = match parser.lines.next() {
        Some("TERRATOPIA_SAVE 1") => 1,
        Some("TERRATOPIA_SAVE 2") => 2,
        Some(HEADER) => 3,
        _ => return Err(invalid("Unsupported save version; expected 1, 2 or 3")),
    };
    let generator = num::<u8>(parser.row("GENERATOR", 1)?[0])?;
    if !matches!(
        (version, generator),
        (1, 2) | (2, 2) | (2, 3) | (3, 2) | (3, 3)
    ) {
        return Err(invalid(
            "Incompatible generator: v1 requires generator 2; v2 supports 2 and 3",
        ));
    }
    let seed = num(parser.row("SEED", 1)?[0])?;
    let mut game = if generator == 2 {
        Game::legacy(seed)
    } else {
        Game::new(seed)
    };
    let p = parser.row("PLAYER", 2)?;
    game.player.x = num(p[0])?;
    game.player.y = num(p[1])?;
    let v = parser.row("VITALS", 4)?;
    game.survival = Survival::from_parts(num(v[0])?, num(v[1])?, num(v[2])?, num(v[3])?)
        .ok_or_else(|| invalid("Vitals out of range"))?;
    let c = parser.row("CAMP", 4)?;
    game.camp.cache_claimed = boolean(c[0])?;
    game.camp.ship_charge = num(c[1])?;
    game.camp.night_progress = num(c[2])?;
    game.camp.completed = boolean(c[3])?;
    if game.camp.ship_charge > 600 || game.camp.night_progress > NIGHT_TICKS {
        return Err(invalid("Invalid camp state"));
    }
    let mut stacks = Vec::new();
    for _ in 0..parser.count("INVENTORY", crate::inventory::CAPACITY)? {
        let r = parser.row("ITEM", 2)?;
        if version == 1 && num::<usize>(r[0])? > 10 {
            return Err(invalid("Item is not valid in v1"));
        }
        stacks.push(Stack {
            item: *Item::ALL
                .get(num::<usize>(r[0])?)
                .ok_or_else(|| invalid("Unknown item"))?,
            count: num(r[1])?,
        });
    }
    game.inventory = Inventory::from_stacks(stacks).map_err(invalid)?;
    let mut positions = std::collections::BTreeSet::new();
    for _ in 0..parser.count("SHELTERS", 1000)? {
        let r = parser.row("SHELTER", 3)?;
        let position = (num(r[0])?, num(r[1])?);
        let charge = num(r[2])?;
        if charge > 600 || !positions.insert(position) || position == game.camp.ship() {
            return Err(invalid("Invalid or duplicate shelter"));
        }
        game.camp.shelters.push(Shelter { position, charge });
    }
    for _ in 0..parser.count("HARVESTED", 100_000)? {
        let r = parser.row("NODE", 3)?;
        let p = (num(r[0])?, num(r[1])?);
        let n = num(r[2])?;
        if n == 0
            || n > game.camp.deposit_capacity(&game.world, p)
            || !matches!(
                game.camp.tile(&mut game.world, p.0, p.1).landmark,
                Some(Landmark::Deposit(_))
            )
            || game.camp.harvested.insert(p, n).is_some()
        {
            return Err(invalid("Invalid or duplicate harvested node"));
        }
    }
    for _ in 0..parser.count("FOG", 20_000)? {
        let r = parser.row("MASK", 34)?;
        let x = num::<i64>(r[0])?;
        let y = num::<i64>(r[1])?;
        if !(i64::MIN / 32..=i64::MAX / 32).contains(&x)
            || !(i64::MIN / 32..=i64::MAX / 32).contains(&y)
        {
            return Err(invalid("Fog chunk out of range"));
        }
        let mut rows = [0; 32];
        for (row, field) in rows.iter_mut().zip(&r[2..]) {
            *row = num(field)?;
        }
        if game.exploration.explored.insert((x, y), rows).is_some() {
            return Err(invalid("Duplicate fog chunk"));
        }
    }
    for _ in 0..parser.count("DISCOVERIES", 100_000)? {
        let r = parser.row("SITE", 3)?;
        let x = num(r[0])?;
        let y = num(r[1])?;
        if version == 1 && num::<u8>(r[2])? > 7 {
            return Err(invalid("Discovery is not valid in v1"));
        }
        let kind = kind(num(r[2])?)?;
        if game
            .exploration
            .sites
            .insert((x, y), Site { x, y, kind })
            .is_some()
        {
            return Err(invalid("Duplicate discovery"));
        }
    }
    if version >= 2 {
        let r = parser.row("LIVING", 2)?;
        game.filter_ticks = num(r[0])?;
        game.resting = boolean(r[1])?;
        if game.filter_ticks > 960 {
            return Err(invalid("Filter timer exceeds limit"));
        }
        for _ in 0..parser.count("PLANTS", 100_000)? {
            let r = parser.row("PLANT", 2)?;
            let p = (num(r[0])?, num(r[1])?);
            if !matches!(
                game.camp.initial_tile(&mut game.world, p.0, p.1).landmark,
                Some(Landmark::Plant(_))
            ) || !game.camp.plants.insert(p)
            {
                return Err(invalid("Invalid or duplicate harvested plant"));
            }
        }
        for _ in 0..parser.count("RUINS", 10_000)? {
            let r = parser.row("RUIN", 2)?;
            let p = (num::<i64>(r[0])?, num::<i64>(r[1])?);
            let mut found = false;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if let (Some(x), Some(y)) = (p.0.checked_add(dx), p.1.checked_add(dy))
                        && game
                            .camp
                            .site(&mut game.world, x, y)
                            .is_some_and(|s| s.kind == Landmark::Ruin && (s.x, s.y) == p)
                    {
                        found = true;
                    }
                }
            }
            if !found || !game.camp.looted_ruins.insert(p) {
                return Err(invalid("Invalid or duplicate looted ruin"));
            }
        }
        for _ in 0..parser.count("MEMORY", crate::wildlife::MAX_MEMORY)? {
            let state = read_actor(&mut parser, "CREATURE", version)?;
            if version < 3 && state.id.2 > 5
                || !crate::wildlife::Wildlife::validate_record(&mut game.world, &game.camp, state)
                || game.wildlife.memory.insert(state.id, state).is_some()
            {
                return Err(invalid("Invalid or duplicate persistent creature"));
            }
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut positions = std::collections::BTreeSet::new();
        for _ in 0..parser.count("ACTORS", crate::wildlife::MAX_ACTIVE)? {
            let state = read_actor(&mut parser, "ACTOR", version)?;
            if version < 3 && state.id.2 > 5
                || !crate::wildlife::Wildlife::validate_record(&mut game.world, &game.camp, state)
                || !ids.insert(state.id)
                || state.health > 0
                    && (!positions.insert(state.position)
                        || state.position == (game.player.x, game.player.y))
                || crate::wildlife::distance(state.position, (game.player.x, game.player.y))
                    > i128::from(crate::wildlife::ACTIVE_RADIUS + 12)
                || game
                    .wildlife
                    .memory
                    .get(&state.id)
                    .is_some_and(|s| s != &state)
            {
                return Err(invalid("Invalid active creature position or state"));
            }
            let mut creature =
                crate::wildlife::Wildlife::prototype(&mut game.world, &game.camp, state.id)
                    .ok_or_else(|| invalid("Invalid creature identity"))?;
            creature.state = state;
            game.wildlife.active.push(creature);
        }
        for _ in 0..parser.count("SCANNED", 100_000)? {
            let r = parser.row("ID", 3)?;
            let id = (num(r[0])?, num(r[1])?, num(r[2])?);
            if crate::wildlife::Wildlife::prototype(&mut game.world, &game.camp, id).is_none()
                || !game.wildlife.scanned.insert(id)
            {
                return Err(invalid("Invalid scanned wildlife identity"));
            }
        }
    }
    if version >= 3 {
        game.wildlife.time = num(parser.row("ECOSYSTEM", 1)?[0])?;
        if game.wildlife.time != game.survival.ticks() {
            return Err(invalid("Ecosystem time differs from simulation time"));
        }
        for _ in 0..parser.count("POPULATIONS", crate::wildlife::MAX_REGIONS)? {
            let r = parser.row("POPULATION", 10)?;
            let key = (num::<i64>(r[0])?, num::<i64>(r[1])?);
            let p = crate::wildlife::Population {
                counts: [num(r[2])?, num(r[3])?, num(r[4])?, num(r[5])?],
                food: num(r[6])?,
                capacity: num(r[7])?,
                epoch: num(r[8])?,
                next_slot: num(r[9])?,
            };
            if !(i64::MIN / 32..=i64::MAX / 32).contains(&key.0)
                || !(i64::MIN / 32..=i64::MAX / 32).contains(&key.1)
                || p.counts
                    .iter()
                    .any(|&n| n > crate::wildlife::REGION_CAPACITY)
                || p.counts.iter().map(|&n| u32::from(n)).sum::<u32>()
                    > u32::from(crate::wildlife::REGION_CAPACITY)
                || p.food > 120
                || p.capacity > crate::wildlife::REGION_CAPACITY
                || p.capacity == 0
                || p.epoch > game.wildlife.time / 240
                || !(32..=256).contains(&p.next_slot)
                || game.wildlife.populations.insert(key, p).is_some()
            {
                return Err(invalid("Invalid population summary"));
            }
        }
        game.wildlife
            .restore_regions((game.player.x, game.player.y));
        game.wildlife
            .validate_populations(&mut game.world, &game.camp)
            .map_err(invalid)?;
    } else {
        game.wildlife.time = game.survival.ticks();
        // Upgrade old ambient actors into retained residents, preserving tuple IDs.
        for c in &game.wildlife.active {
            game.wildlife.memory.insert(c.state.id, c.state);
        }
        game.wildlife
            .migrate_populations(&mut game.world, &game.camp);
    }
    for shelter in &game.camp.shelters {
        let tile = game
            .camp
            .tile(&mut game.world, shelter.position.0, shelter.position.1);
        if !tile.biome.passable() || tile.landmark.is_some() {
            return Err(invalid("Shelter must occupy cleared walkable ground"));
        }
    }
    parser.row("END", 0)?;
    if parser.lines.next().is_some() {
        return Err(invalid("Trailing save data"));
    }
    if !game
        .camp
        .tile(&mut game.world, game.player.x, game.player.y)
        .biome
        .passable()
    {
        return Err(invalid("Player occupies impassable terrain"));
    }
    game.wildlife
        .active
        .sort_by_key(|creature| creature.state.id);
    game.wildlife.rebuild_index();
    game.wildlife
        .restore_regions((game.player.x, game.player.y));
    game.exploration.recount();
    game.world.focus(game.player.x, game.player.y);
    game.started = true;
    if version == 1 {
        game.refresh_wildlife();
    }
    game.observe(false);
    game.state = if game.survival.health == 0 {
        State::Dead
    } else {
        State::Paused
    };
    game.say(if version==1 {"Expedition loaded. Legacy terrain retained; next save upgrades to v3 with .v1.bak backup. P resumes."}else{"Expedition loaded. P resumes; M surveys living-world conditions."});
    Ok(game)
}
pub fn save(game: &Game, path: &Path) -> io::Result<()> {
    let text = encode(game)?;
    if path.exists() {
        let mut old = String::new();
        File::open(path)?.take(LIMIT + 1).read_to_string(&mut old)?;
        if old.len() as u64 > LIMIT {
            return Err(invalid(
                "Existing save is oversized; choose another path to preserve it",
            ));
        }
        if let Some(version) = old.lines().next().and_then(|line| match line {
            "TERRATOPIA_SAVE 1" => Some(1),
            "TERRATOPIA_SAVE 2" => Some(2),
            _ => None,
        }) {
            let mut backup = path.as_os_str().to_os_string();
            backup.push(format!(".v{version}.bak"));
            let backup = std::path::PathBuf::from(backup);
            if backup.exists() {
                if fs::metadata(&backup)?.len() != old.len() as u64
                    || fs::read(&backup)? != old.as_bytes()
                {
                    return Err(invalid(
                        "Legacy backup differs; choose a different save path to preserve both files",
                    ));
                }
            } else {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&backup)?;
                if let Err(e) = file
                    .write_all(old.as_bytes())
                    .and_then(|()| file.sync_all())
                {
                    drop(file);
                    let _ = fs::remove_file(&backup);
                    return Err(e);
                }
            }
        }
    }
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(format!(".tmp.{}", std::process::id()));
    let temporary = std::path::PathBuf::from(temporary);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
pub fn load(path: &Path) -> io::Result<Game> {
    let file = File::open(path)?;
    if file.metadata()?.len() > LIMIT {
        return Err(invalid("Save exceeds 8 MiB limit"));
    }
    let mut text = String::new();
    file.take(LIMIT + 1).read_to_string(&mut text)?;
    decode(&text)
}
