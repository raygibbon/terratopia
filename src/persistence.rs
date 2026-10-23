//! Strict versioned text saves. Parse completely before replacing a live game.
use crate::{
    game::{Game, State},
    inventory::{Inventory, Item, Stack},
    planet::{DEPOSIT_UNITS, Shelter},
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
const HEADER: &str = "TERRATOPIA_SAVE 1";
fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
fn kind_id(kind: Landmark) -> u8 {
    match kind {
        Landmark::Ruin => 0,
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
    {
        return Err(invalid("Save entry limit exceeded"));
    }
    let mut out = format!(
        "{HEADER}\nGENERATOR 2\nSEED {}\nPLAYER {} {}\nVITALS {} {} {} {}\nCAMP {} {} {} {}\n",
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
    out.push_str("END\n");
    if out.len() as u64 > LIMIT {
        return Err(invalid("Save exceeds 8 MiB limit"));
    }
    Ok(out)
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
    if parser.lines.next() != Some(HEADER) {
        return Err(invalid("Unsupported save version"));
    }
    if parser.row("GENERATOR", 1)?[0] != "2" {
        return Err(invalid("Unsupported world generator version"));
    }
    let seed = num(parser.row("SEED", 1)?[0])?;
    let mut game = Game::new(seed);
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
            || n > DEPOSIT_UNITS
            || !matches!(
                game.camp.tile(&mut game.world, p.0, p.1).landmark,
                Some(Landmark::Deposit(_))
            )
            || game.camp.harvested.insert(p, n).is_some()
        {
            return Err(invalid("Invalid or duplicate harvested node"));
        }
    }
    for shelter in &game.camp.shelters {
        let tile = game
            .camp
            .tile(&mut game.world, shelter.position.0, shelter.position.1);
        if !tile.biome.passable() || tile.landmark.is_some() {
            return Err(invalid("Shelter must occupy cleared walkable ground"));
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
    game.exploration.recount();
    game.world.focus(game.player.x, game.player.y);
    game.started = true;
    game.observe(false);
    game.state = if game.survival.health == 0 {
        State::Dead
    } else {
        State::Paused
    };
    game.say("Expedition loaded. P resumes; H shows the survival guide.");
    Ok(game)
}
pub fn save(game: &Game, path: &Path) -> io::Result<()> {
    let text = encode(game)?;
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
