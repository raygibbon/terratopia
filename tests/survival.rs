use std::{fs, path::PathBuf, time::Duration};
use terratopia::{
    crafting,
    game::{Game, STEP, State},
    input::Action,
    inventory::{CAPACITY, Inventory, Item},
    persistence,
    planet::{Camp, DEPOSIT_UNITS},
    survival::{Environment, Survival},
    world::{
        biome::Biome,
        landmark::{Landmark, Resource},
        terrain::generate,
    },
};
fn start(seed: u64) -> Game {
    let mut game = Game::new(seed);
    game.handle(Action::NewWorld);
    game.handle(Action::Accept);
    game
}
fn walk(game: &mut Game, p: (i64, i64)) {
    while game.player.x != p.0 {
        let before = game.player;
        game.handle(Action::Move((p.0 - game.player.x).signum(), 0));
        assert_ne!(game.player, before);
    }
    while game.player.y != p.1 {
        let before = game.player;
        game.handle(Action::Move(0, (p.1 - game.player.y).signum()));
        assert_ne!(game.player, before);
    }
}
fn recipe(game: &mut Game, index: usize) {
    game.handle(Action::Craft);
    for _ in 0..index {
        game.handle(Action::Move(0, 1));
    }
    game.handle(Action::Accept);
    game.handle(Action::Back);
}
fn equipped_camp() -> Game {
    let mut game = start(827193);
    let origin = game.camp.origin;
    game.handle(Action::Interact);
    assert!(game.camp.cache_claimed);
    let nodes = game.camp.starter_nodes();
    for &(p, _) in &nodes[..2] {
        walk(&mut game, p);
        for _ in 0..6 {
            game.handle(Action::Interact);
        }
    }
    recipe(&mut game, 0);
    assert_eq!(game.inventory.count(Item::MiningTool), 1);
    walk(&mut game, nodes[2].0);
    for _ in 0..6 {
        game.handle(Action::Interact);
    }
    recipe(&mut game, 1);
    assert_eq!(game.inventory.count(Item::ShelterKit), 1);
    walk(&mut game, origin);
    game.handle(Action::Build);
    assert!(game.camp.sheltered(origin));
    game.handle(Action::Interact);
    assert_eq!(game.camp.shelters[0].charge, 120);
    game
}
#[test]
fn complete_crash_scavenge_gather_craft_build_survive_loop() {
    let mut game = equipped_camp();
    for _ in 0..480 {
        game.advance(STEP);
    }
    assert!(game.camp.completed);
    assert_eq!(game.survival.day(), 2);
    assert_eq!(game.survival.health, 100);
    assert_eq!(game.survival.oxygen, 100);
    assert!(
        game.messages
            .iter()
            .any(|s| s.contains("OBJECTIVE COMPLETE"))
    );
}
#[test]
fn crash_sites_are_safe_and_all_starter_deposits_are_reachable() {
    for seed in [0, 1, 42, 827193, u64::MAX] {
        let mut game = start(seed);
        let (x, y) = game.camp.origin;
        assert!(terratopia::planet::near(
            (game.player.x, game.player.y),
            game.camp.ship()
        ));
        for dx in -4..=4 {
            for dy in -4..=4 {
                assert!(
                    game.camp
                        .tile(&mut game.world, x + dx, y + dy)
                        .biome
                        .passable()
                );
            }
        }
        let nodes = game.camp.starter_nodes();
        for (p, resource) in nodes {
            walk(&mut game, p);
            assert_eq!(
                game.camp.tile(&mut game.world, p.0, p.1).landmark,
                Some(Landmark::Deposit(resource))
            );
        }
    }
}
#[test]
fn stacks_capacity_and_crafting_failures_are_atomic() {
    let mut inventory = Inventory::default();
    inventory.add(Item::Carbon, 21).unwrap();
    assert_eq!(inventory.stacks().len(), 2);
    inventory.remove(Item::Carbon, 20).unwrap();
    assert_eq!(inventory.count(Item::Carbon), 1);
    assert_eq!(inventory.stacks().len(), 1);
    let before = inventory.clone();
    assert!(crafting::craft(&mut inventory, 0).is_err());
    assert_eq!(inventory, before);
    inventory.add(Item::Iron, 200).unwrap_err();
    assert_eq!(inventory, before);
    let mut full = Inventory::default();
    for _ in 0..CAPACITY {
        full.add(Item::BasicPick, 1).unwrap();
    }
    let before = full.clone();
    assert!(full.add(Item::Iron, 1).is_err());
    assert_eq!(full, before);
    let mut ingredients = Inventory::default();
    ingredients.add(Item::Iron, 20).unwrap();
    ingredients.add(Item::Carbon, 20).unwrap();
    for _ in 0..8 {
        ingredients.add(Item::BasicPick, 1).unwrap();
    }
    let before = ingredients.clone();
    assert!(crafting::craft(&mut ingredients, 0).is_err());
    assert_eq!(ingredients, before);
}
#[test]
fn all_recipes_consume_exact_ingredients_and_create_one_output() {
    for (i, recipe) in crafting::RECIPES.iter().enumerate() {
        let mut inventory = Inventory::default();
        for &(item, n) in recipe.ingredients {
            inventory.add(item, n).unwrap();
        }
        assert_eq!(crafting::craft(&mut inventory, i).unwrap(), recipe.output);
        assert_eq!(inventory.count(recipe.output), 1);
        assert_eq!(inventory.stacks().len(), 1);
    }
}
#[test]
fn cache_harvesting_tool_gates_and_depletion_are_independent_from_base_world() {
    let mut game = start(42);
    let nodes = game.camp.starter_nodes();
    let p = nodes[0].0;
    assert!(
        game.camp
            .harvest(&mut game.world, &mut game.inventory, p)
            .is_err()
    );
    game.camp.scavenge(&mut game.inventory).unwrap();
    let before = game.inventory.clone();
    assert!(game.camp.scavenge(&mut game.inventory).is_err());
    assert_eq!(game.inventory, before);
    assert!(
        game.camp
            .harvest(&mut game.world, &mut game.inventory, nodes[2].0)
            .is_err()
    );
    let original = game.world.tile(p.0, p.1);
    for _ in 0..6 {
        game.camp
            .harvest(&mut game.world, &mut game.inventory, p)
            .unwrap();
    }
    assert_eq!(game.camp.harvested[&p], DEPOSIT_UNITS);
    assert!(
        game.camp
            .harvest(&mut game.world, &mut game.inventory, p)
            .is_err()
    );
    assert_eq!(game.camp.tile(&mut game.world, p.0, p.1).landmark, None);
    assert_eq!(game.world.tile(p.0, p.1), original);
    assert_eq!(game.world.tile(p.0, p.1), generate(42, p.0, p.1));
    for x in 100..300 {
        game.world.tile(x * 32, 0);
    }
    assert_eq!(game.camp.tile(&mut game.world, p.0, p.1).landmark, None);
}
#[test]
fn full_inventory_does_not_consume_cache_or_deposit() {
    let mut game = start(0);
    for _ in 0..CAPACITY {
        game.inventory.add(Item::BasicPick, 1).unwrap();
    }
    assert!(game.camp.scavenge(&mut game.inventory).is_err());
    assert!(!game.camp.cache_claimed);
    let p = game.camp.starter_nodes()[0].0;
    assert!(
        game.camp
            .harvest(&mut game.world, &mut game.inventory, p)
            .is_err()
    );
    assert!(!game.camp.harvested.contains_key(&p));
}
fn env(biome: Biome) -> Environment {
    Environment {
        biome,
        sheltered: false,
        protected: false,
        powered: false,
    }
}
fn ticks(s: &mut Survival, e: Environment, n: u64) {
    for _ in 0..n {
        s.tick(e);
    }
}
#[test]
fn oxygen_depletion_canisters_and_powered_stations() {
    let mut survival = Survival::default();
    ticks(&mut survival, env(Biome::Grassland), 12);
    assert_eq!(survival.oxygen, 99);
    let mut depleted = Survival::from_parts(100, 0, 100, 0).unwrap();
    ticks(&mut depleted, env(Biome::Grassland), 4);
    assert_eq!(depleted.health, 97);
    let mut powered = env(Biome::Grassland);
    powered.powered = true;
    ticks(&mut depleted, powered, 4);
    assert_eq!(depleted.oxygen, 3);
    assert_eq!(depleted.health, 97);
    let mut game = start(1);
    game.handle(Action::Interact);
    game.survival.oxygen = 10;
    game.handle(Action::UseOxygen);
    assert_eq!(game.survival.oxygen, 60);
    assert_eq!(game.inventory.count(Item::OxygenCanister), 1);
    game.survival.oxygen = 100;
    game.handle(Action::UseOxygen);
    assert_eq!(game.inventory.count(Item::OxygenCanister), 1);
    game.camp
        .power_station(&mut game.inventory, Camp::point(game.player))
        .unwrap();
    assert!(game.camp.powered(Camp::point(game.player)));
    assert_eq!(game.inventory.count(Item::Battery), 0);
}
#[test]
fn biome_temperature_equipment_shelter_and_night_change_danger() {
    let mut cold = Survival::default();
    let mut protected = Survival::default();
    let mut safe = Survival::default();
    let frozen = env(Biome::Frozen);
    let mut suit = frozen;
    suit.protected = true;
    let mut shelter = frozen;
    shelter.sheltered = true;
    ticks(&mut cold, frozen, 128);
    ticks(&mut protected, suit, 128);
    ticks(&mut safe, shelter, 128);
    assert!(cold.health < protected.health);
    assert_eq!(protected.health, 100);
    assert_eq!(safe.health, 100);
    let night = Survival::from_parts(100, 100, 100, 320).unwrap();
    let day = Survival::default();
    assert_eq!(
        day.temperature(env(Biome::Forest)) - night.temperature(env(Biome::Forest)),
        20
    );
    assert!(!day.hazardous(env(Biome::Forest)));
    assert!(night.hazardous(env(Biome::Forest)));
    assert!(day.hazardous(env(Biome::Wasteland)));
    let mut suit = env(Biome::Wasteland);
    suit.protected = true;
    assert!(!day.hazardous(suit));
}
#[test]
fn menus_pause_time_and_death_stops_simulation() {
    let mut game = start(3);
    for action in [
        Action::Inventory,
        Action::Craft,
        Action::Help,
        Action::Pause,
    ] {
        game.handle(action);
        let before = game.survival.clone();
        game.advance(Duration::from_secs(100));
        assert_eq!(game.survival, before);
        game.handle(if action == Action::Pause {
            Action::Pause
        } else {
            Action::Back
        });
        assert_eq!(game.state, State::Playing);
    }
    game.survival = Survival::from_parts(1, 0, 100, 0).unwrap();
    game.advance(STEP * 4);
    assert_eq!(game.state, State::Dead);
    let before = game.survival.clone();
    game.advance(STEP * 4);
    assert_eq!(game.survival, before);
}
#[test]
fn objective_requires_an_entire_sheltered_night() {
    let mut game = equipped_camp();
    for _ in 0..320 {
        game.advance(STEP);
    }
    game.handle(Action::Move(0, 1));
    game.advance(STEP);
    game.handle(Action::Move(0, -1));
    for _ in 321..480 {
        game.advance(STEP);
    }
    assert!(!game.camp.completed);
    for _ in 0..480 {
        game.advance(STEP);
    }
    assert!(game.camp.completed);
}
#[test]
fn versioned_save_roundtrip_preserves_state_modifications_and_exploration() {
    let mut game = equipped_camp();
    game.handle(Action::Scan);
    for _ in 0..350 {
        game.advance(STEP);
    }
    let text = persistence::encode(&game).unwrap();
    let loaded = persistence::decode(&text).unwrap();
    assert_eq!(loaded.state, State::Paused);
    assert_eq!(loaded.world.seed(), game.world.seed());
    assert_eq!(loaded.player, game.player);
    assert_eq!(loaded.survival, game.survival);
    assert_eq!(loaded.inventory, game.inventory);
    assert_eq!(loaded.camp, game.camp);
    assert_eq!(
        loaded.exploration.explored_cells(),
        game.exploration.explored_cells()
    );
    assert_eq!(
        loaded.exploration.discovered_sites(),
        game.exploration.discovered_sites()
    );
    assert_eq!(persistence::encode(&loaded).unwrap(), text);
    let mut loaded = loaded;
    loaded.handle(Action::Pause);
    for _ in 350..480 {
        loaded.advance(STEP);
    }
    assert!(loaded.camp.completed);
}
#[test]
fn malformed_truncated_future_and_oversized_saves_are_rejected() {
    let game = equipped_camp();
    let text = persistence::encode(&game).unwrap();
    for bad in [
        text.replace("SAVE 1", "SAVE 9"),
        text.replace("GENERATOR 2", "GENERATOR 9"),
        text.replace("ITEM 5 1", "ITEM 500 1"),
        text.replace("VITALS 100", "VITALS 101"),
        text.replace("INVENTORY 7", "INVENTORY 999999"),
    ] {
        if bad != text {
            assert!(persistence::decode(&bad).is_err());
        }
    }
    for i in (0..text.find("END").unwrap()).step_by(97) {
        assert!(persistence::decode(&text[..i]).is_err());
    }
    assert!(persistence::decode(&"X".repeat(8 * 1024 * 1024 + 1)).is_err());
    assert!(persistence::decode(&(text + "junk\n")).is_err());
}
#[test]
fn save_file_replacement_and_failure_leave_previous_save_intact() {
    let folder: PathBuf =
        std::env::temp_dir().join(format!("terratopia-save-test-{}", std::process::id()));
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("expedition.save");
    let mut game = equipped_camp();
    persistence::save(&game, &path).unwrap();
    game.handle(Action::Scan);
    persistence::save(&game, &path).unwrap();
    let old = fs::read(&path).unwrap();
    assert_eq!(persistence::load(&path).unwrap().inventory, game.inventory);
    let temp = folder.join(format!("expedition.save.tmp.{}", std::process::id()));
    fs::write(&temp, b"occupied").unwrap();
    assert!(persistence::save(&game, &path).is_err());
    assert_eq!(fs::read(&path).unwrap(), old);
    assert_eq!(fs::read(&temp).unwrap(), b"occupied");
    fs::remove_dir_all(folder).unwrap();
}
#[test]
fn generated_carbon_and_crystal_exist_with_distinct_glyphs() {
    let mut found = [false; 2];
    for y in -200..200 {
        for x in -200..200 {
            match generate(827193, x, y).landmark {
                Some(Landmark::Deposit(Resource::Carbon)) => found[0] = true,
                Some(Landmark::Deposit(Resource::Crystal)) => found[1] = true,
                _ => {}
            }
        }
    }
    assert_eq!(found, [true, true]);
    assert_ne!(Resource::Carbon.glyph(), Resource::Crystal.glyph());
}
