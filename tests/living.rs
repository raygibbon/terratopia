use std::{
    collections::BTreeSet,
    fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use terrabox::{Size, Surface};
use terratopia::{
    game::{Game, STEP, State},
    input::Action,
    inventory::Item,
    persistence,
    planet::{Camp, Shelter},
    survival::{Conditions, Environment, Survival},
    weather::{Condition, Weather},
    wildlife::{Behaviour, MAX_ACTIVE, Wildlife, distance, species},
    world::{
        World,
        biome::Biome,
        flora::Flora,
        landmark::Landmark,
        profile::{Archetype, Profile},
        terrain,
    },
};
fn game(seed: u64) -> Game {
    let mut g = Game::new(seed);
    g.handle(Action::NewWorld);
    g.handle(Action::Accept);
    g
}
fn position(g: &mut Game, dx: i64, dy: i64) {
    let p = g.camp.offset(dx, dy);
    g.player.x = p.0;
    g.player.y = p.1;
    g.world.focus(p.0, p.1);
    g.refresh_wildlife();
    g.observe(false);
}
#[test]
fn profile_archetypes_are_deterministic_and_change_generation_and_resources() {
    let mut archetypes = BTreeSet::new();
    let mut differences = 0;
    for seed in 0..100 {
        let p = Profile::generate(seed);
        assert_eq!(p, Profile::generate(seed));
        archetypes.insert(p.archetype.name());
        let mut world = World::new(seed, 1);
        let mut legacy = World::legacy(seed, 1);
        for x in -5..5 {
            if world.tile(x * 30, 97) != legacy.tile(x * 30, 97) {
                differences += 1;
            }
        }
        let camp = Camp::new((0, 0));
        assert_eq!(
            camp.deposit_capacity(&world, (100, 100)),
            (u32::from(p.minerals) * 12 / 100).clamp(6, 24) as u8
        );
    }
    assert_eq!(archetypes.len(), 7);
    assert!(differences > 100);
}
#[test]
fn new_and_legacy_chunks_match_their_coordinate_samplers_at_edges() {
    for seed in [0, 42, u64::MAX] {
        for legacy in [false, true] {
            let mut world = if legacy {
                World::legacy(seed, 1)
            } else {
                World::new(seed, 1)
            };
            for (bx, by) in [(-32, -64), (32, 0), (i64::MIN, i64::MAX - 31)] {
                for y in 0..32 {
                    for x in 0..32 {
                        let expected = if legacy {
                            terrain::generate_legacy(seed, bx + x, by + y)
                        } else {
                            terrain::generate(seed, bx + x, by + y)
                        };
                        assert_eq!(world.tile(bx + x, by + y), expected);
                    }
                }
            }
        }
    }
}
#[test]
fn all_planet_types_can_finish_the_starter_loop() {
    let mut seen = BTreeSet::new();
    for seed in 0..100 {
        let profile = Profile::generate(seed);
        if !seen.insert(profile.archetype.name()) {
            continue;
        }
        let mut g = game(seed);
        g.handle(Action::Interact);
        for (p, _) in g.camp.starter_nodes().into_iter().take(2) {
            for _ in 0..6 {
                g.camp.harvest(&mut g.world, &mut g.inventory, p).unwrap();
            }
        }
        terratopia::crafting::craft(&mut g.inventory, 0).unwrap();
        let p = g.camp.starter_nodes()[2].0;
        for _ in 0..6 {
            g.camp.harvest(&mut g.world, &mut g.inventory, p).unwrap();
        }
        terratopia::crafting::craft(&mut g.inventory, 1).unwrap();
        let p = (g.player.x, g.player.y);
        g.camp.build(&mut g.world, &mut g.inventory, p).unwrap();
        g.camp.power_station(&mut g.inventory, p).unwrap();
        for _ in 0..480 {
            g.advance(STEP);
        }
        assert!(g.camp.completed, "seed {seed}");
        assert_eq!(g.survival.health, 100);
    }
    assert_eq!(seen.len(), 7);
}
#[test]
fn wildlife_spawn_and_species_are_deterministic_with_bounded_active_simulation() {
    let mut a = game(827193);
    let b = game(827193);
    assert_eq!(a.wildlife.active, b.wildlife.active);
    assert_ne!(
        species(1, (0, 0, 0), Behaviour::Grazer),
        species(2, (0, 0, 0), Behaviour::Grazer)
    );
    assert_eq!(
        [
            Behaviour::Grazer.glyph(),
            Behaviour::Defender.glyph(),
            Behaviour::Predator.glyph()
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
        .len(),
        3
    );
    let origin = a.camp.origin;
    for x in -12..12 {
        a.wildlife
            .refresh(&mut a.world, &a.camp, (origin.0 + x * 32, origin.1));
        assert!(a.wildlife.active.len() <= MAX_ACTIVE);
    }
    let mut world = World::new(827193, 1);
    let camp = Camp::new(origin);
    for id in [
        (origin.0, origin.1, 3),
        (origin.0, origin.1, 4),
        (origin.0, origin.1, 5),
    ] {
        let expected = Wildlife::prototype(&mut world, &camp, id).unwrap();
        for x in 1..6 {
            world.tile(x * 256, -900);
        }
        assert_eq!(
            Wildlife::prototype(&mut world, &camp, id).unwrap(),
            expected
        );
    }
}
#[test]
fn grazers_flee_defenders_guard_and_predators_pursue_then_disengage() {
    let mut g = game(827193);
    let origin = g.camp.origin;
    let mut grazers = Wildlife::default();
    let grazer = Wildlife::prototype(&mut g.world, &g.camp, (origin.0, origin.1, 3)).unwrap();
    let player = g.camp.offset(5, 0);
    let before = distance(grazer.state.position, player);
    grazers.active.push(grazer);
    for tick in (4..=32).step_by(4) {
        grazers.tick(&mut g.world, &g.camp, player, tick);
    }
    assert!(distance(grazers.active[0].state.position, player) > before);
    assert!(!grazers.active[0].state.angry);
    let mut guards = Wildlife::default();
    guards
        .active
        .push(Wildlife::prototype(&mut g.world, &g.camp, (origin.0, origin.1, 4)).unwrap());
    let (damage, _) = guards.tick(&mut g.world, &g.camp, g.camp.offset(10, 0), 4);
    assert!(damage > 0);
    assert!(guards.active[0].state.angry);
    let mut predators = Wildlife::default();
    predators
        .active
        .push(Wildlife::prototype(&mut g.world, &g.camp, (origin.0, origin.1, 5)).unwrap());
    let player = g.camp.offset(13, 0);
    let before = distance(predators.active[0].state.position, player);
    for tick in (4..=40).step_by(4) {
        predators.tick(&mut g.world, &g.camp, player, tick);
    }
    assert!(distance(predators.active[0].state.position, player) < before);
    assert!(predators.active[0].state.angry);
    let (damage, _) = predators.tick(&mut g.world, &g.camp, g.camp.origin, 44);
    assert_eq!(damage, 0);
    assert!(!predators.active[0].state.angry);
}
#[test]
fn creatures_never_overlap_or_enter_blocked_ground_player_or_shelter() {
    let mut g = game(827193);
    let player = g.camp.offset(13, 0);
    g.camp.shelters.push(Shelter {
        position: player,
        charge: 0,
    });
    for tick in (4..=240).step_by(4) {
        let (damage, _) = g.wildlife.tick(&mut g.world, &g.camp, player, tick);
        assert_eq!(damage, 0);
        let mut positions = BTreeSet::new();
        for c in &g.wildlife.active {
            if c.state.health == 0 {
                continue;
            }
            assert!(positions.insert(c.state.position));
            assert_ne!(c.state.position, player);
            assert!(
                g.camp
                    .tile(&mut g.world, c.state.position.0, c.state.position.1)
                    .biome
                    .passable()
            );
            assert!(!g.camp.sheltered(c.state.position));
        }
    }
    let c = g
        .wildlife
        .active
        .iter()
        .find(|c| c.state.health > 0)
        .unwrap();
    let p = c.state.position;
    g.player.x = p.0 - 1;
    g.player.y = p.1;
    let before = g.player;
    g.handle(Action::Move(1, 0));
    assert_eq!(g.player, before);
}
#[test]
fn forecasts_warn_then_build_storm_severity_and_reduce_sensor_radius() {
    let mut profile = Profile::generate(1);
    profile.archetype = Archetype::Toxic;
    let seed = (0..100)
        .find(|&s| Weather::at(s, profile, Biome::Forest, 0, 0, 32 * 4).next != Condition::Clear)
        .unwrap();
    let clear = Weather::at(seed, profile, Biome::Forest, 0, 0, 31 * 4);
    let warning = Weather::at(seed, profile, Biome::Forest, 0, 0, 32 * 4);
    let storm = Weather::at(seed, profile, Biome::Forest, 0, 0, 40 * 4);
    assert_eq!(clear.current, Condition::Clear);
    assert_eq!(warning.warning, Some(8));
    assert_eq!(storm.current, Condition::Toxic);
    assert_eq!(storm.severity, 1);
    assert!(storm.radius(true) < 20);
    assert!(storm.radius(false) < 7);
    assert_eq!(
        Weather::at(seed, profile, Biome::Forest, 0, 0, 64 * 4).current,
        Condition::Clear
    );
    assert_eq!(
        storm,
        Weather::at(seed, profile, Biome::Forest, 0, 0, 40 * 4)
    );
    // Compare identical seed/region streams: profile and biome must alter
    // both clear-period frequency and the maximum severity, not just labels.
    let mut mild = profile;
    mild.archetype = Archetype::Temperate;
    let mut clear_counts = [0; 3];
    let mut max_severity = [0; 3];
    for epoch in 0..256 {
        for (i, (p, biome)) in [
            (mild, Biome::Forest),
            (mild, Biome::Mountains),
            (profile, Biome::Forest),
        ]
        .into_iter()
        .enumerate()
        {
            let w = Weather::at(42, p, biome, 0, 0, (epoch * 64 + 52) * 4);
            clear_counts[i] += usize::from(w.current == Condition::Clear);
            max_severity[i] = max_severity[i].max(w.severity);
        }
    }
    assert!(clear_counts[0] > clear_counts[1]);
    assert!(clear_counts[1] > clear_counts[2]);
    assert_eq!(max_severity, [2, 3, 3]);
}
fn base_env() -> Environment {
    Environment {
        biome: Biome::Grassland,
        sheltered: false,
        protected: false,
        powered: false,
    }
}
#[test]
fn exposure_combines_climate_oxygen_weather_equipment_and_shelter() {
    let s = Survival::from_parts(100, 20, 100, 320).unwrap();
    let mut p = Profile::generate(4);
    p.temperature = -10;
    p.toxicity = 70;
    p.radiation = 70;
    p.oxygen = 0;
    let context = Conditions {
        planet: p,
        weather: Weather {
            current: Condition::Snow,
            next: Condition::Snow,
            severity: 3,
            warning: None,
        },
        ..Conditions::default()
    };
    let raw = s.exposure(base_env(), context);
    let mut protected = base_env();
    protected.protected = true;
    let mut filtered = context;
    filtered.filtered = true;
    let suit = s.exposure(protected, filtered);
    assert!(raw.damage > suit.damage);
    let mut home = base_env();
    home.sheltered = true;
    assert_eq!(s.exposure(home, context).damage, 0);
    assert_eq!(s.exposure(home, context).temperature, 18);
    let mut breathable = context;
    breathable.weather = Weather::default();
    breathable.planet.oxygen = 22;
    breathable.planet.toxicity = 0;
    let mut shielded = breathable;
    shielded.crash_shield = true;
    assert!(s.exposure(base_env(), shielded).breathable);
    let mut healthy = Survival::from_parts(100, 20, 100, 0).unwrap();
    for _ in 0..24 {
        healthy.tick_living(base_env(), breathable);
    }
    assert!(healthy.oxygen > 20);
    let mut vacuum = Survival::from_parts(100, 20, 100, 0).unwrap();
    for _ in 0..24 {
        vacuum.tick_living(base_env(), context);
    }
    assert!(vacuum.oxygen < 20);
}
#[test]
fn flora_yields_are_atomic_finite_and_survive_eviction_and_save_load() {
    let mut g = game(827193);
    let p = g.camp.offset(5, 0);
    assert_eq!(
        g.camp.tile(&mut g.world, p.0, p.1).landmark,
        Some(Landmark::Plant(Flora::Medicinal))
    );
    position(&mut g, 5, 0);
    g.handle(Action::Interact);
    assert_eq!(g.inventory.count(Item::Herbs), 2);
    assert!(g.camp.plants.contains(&p));
    g.handle(Action::Interact);
    assert_eq!(g.inventory.count(Item::Herbs), 2);
    for x in 0..150 {
        g.world.tile(x * 100, 300);
    }
    assert_eq!(g.camp.tile(&mut g.world, p.0, p.1).landmark, None);
    let mut loaded = persistence::decode(&persistence::encode(&g).unwrap()).unwrap();
    assert!(loaded.camp.plants.contains(&p));
    assert_eq!(loaded.camp.tile(&mut loaded.world, p.0, p.1).landmark, None);
    let mut kinds = BTreeSet::new();
    for y in -100..100 {
        for x in -100..100 {
            if let Some(kind) = Flora::at(1, x, y, Biome::Forest, Profile::generate(1)) {
                kinds.insert(kind);
            }
        }
    }
    assert!(kinds.len() >= 3);
}
#[test]
fn ruin_salvage_is_deterministic_and_looted_only_once_across_saves() {
    let mut a = game(827193);
    let mut b = game(827193);
    position(&mut a, 12, 0);
    position(&mut b, 12, 0);
    a.resting = true;
    a.handle(Action::Interact);
    assert!(!a.resting);
    b.handle(Action::Interact);
    assert_eq!(a.inventory, b.inventory);
    assert_eq!(a.inventory.count(Item::Technology), 1);
    let inventory = a.inventory.clone();
    a.handle(Action::Interact);
    assert_eq!(a.inventory, inventory);
    let mut loaded = persistence::decode(&persistence::encode(&a).unwrap()).unwrap();
    loaded.handle(Action::Pause);
    loaded.handle(Action::Interact);
    assert_eq!(loaded.inventory, inventory);
    assert_eq!(loaded.camp.looted_ruins.len(), 1);
}
#[test]
fn injuries_corpses_and_loot_persist_while_ambient_population_is_bounded() {
    let mut g = game(827193);
    let id = (g.camp.origin.0, g.camp.origin.1, 5);
    position(&mut g, 15, 0);
    assert!(g.wildlife.active.iter().any(|c| c.state.id == id));
    for _ in 0..4 {
        g.handle(Action::Attack);
    }
    let state = *g.wildlife.memory.get(&id).unwrap();
    assert_eq!(state.health, 0);
    let text = persistence::encode(&g).unwrap();
    let mut loaded = persistence::decode(&text).unwrap();
    assert_eq!(loaded.wildlife.memory[&id], state);
    assert_eq!(persistence::encode(&loaded).unwrap(), text);
    loaded.handle(Action::Pause);
    loaded.player.x = state.position.0;
    loaded.player.y = state.position.1;
    loaded.handle(Action::Interact);
    assert_eq!(loaded.inventory.count(Item::Organics), 2);
    loaded.handle(Action::Interact);
    assert_eq!(loaded.inventory.count(Item::Organics), 2);
    loaded
        .wildlife
        .refresh(&mut loaded.world, &loaded.camp, (9000, 9000));
    loaded
        .wildlife
        .refresh(&mut loaded.world, &loaded.camp, loaded.camp.offset(15, 0));
    assert!(!loaded.wildlife.active.iter().any(|c| c.state.id == id));
    assert!(loaded.wildlife.memory[&id].looted);
}
#[test]
fn exhaustion_can_recover_by_rest_and_consumables_work_with_no_waste() {
    let mut g = game(827193);
    g.survival.energy = 0;
    let before = g.player;
    g.handle(Action::Move(1, 0));
    assert_eq!(g.player, before);
    assert!(g.resting);
    for _ in 0..8 {
        g.advance(STEP);
    }
    assert!(g.survival.energy >= 10);
    g.handle(Action::Move(1, 0));
    assert_ne!(g.player, before);
    assert!(!g.resting);
    for item in [Item::MedicalKit, Item::EnergyPack, Item::HazardFilter] {
        g.inventory = terratopia::inventory::Inventory::default();
        g.inventory.add(item, 1).unwrap();
        g.survival.health = 20;
        g.survival.energy = 20;
        g.handle(Action::Inventory);
        g.handle(Action::Accept);
        g.handle(Action::Back);
        match item {
            Item::MedicalKit => assert_eq!(g.survival.health, 55),
            Item::EnergyPack => assert_eq!(g.survival.energy, 60),
            _ => assert_eq!(g.filter_ticks, 480),
        }
        assert_eq!(g.inventory.count(item), 0);
    }
}
#[test]
fn survey_and_other_modals_freeze_all_living_systems_and_handle_resize() {
    let mut g = game(827193);
    g.handle(Action::Scan);
    assert!(!g.wildlife.scanned.is_empty());
    assert!(g.scan_contacts.iter().any(|s| s.contains("grazer")));
    g.handle(Action::Menu);
    assert_eq!(g.state, State::Survey);
    let ticks = g.survival.ticks();
    let creatures = g.wildlife.active.clone();
    let fuel = g.camp.ship_charge;
    g.advance(Duration::from_secs(100));
    assert_eq!(g.survival.ticks(), ticks);
    assert_eq!(g.wildlife.active, creatures);
    assert_eq!(g.camp.ship_charge, fuel);
    let mut surface = Surface::new(Size {
        width: 0,
        height: 0,
    });
    for size in [
        Size {
            width: 80,
            height: 24,
        },
        Size {
            width: 20,
            height: 6,
        },
        Size {
            width: 0,
            height: 0,
        },
        Size {
            width: 100,
            height: 30,
        },
    ] {
        terratopia::renderer::render(&mut g, &mut surface, size);
    }
    g.handle(Action::Back);
    g.advance(Duration::from_secs(100));
    assert_eq!(g.survival.ticks(), 4);
}
#[test]
fn genuine_v1_fixture_retains_progress_and_exact_legacy_generation() {
    let text = include_str!("fixtures/phase2.save");
    let mut g = persistence::decode(text).unwrap();
    assert_eq!(g.world.generator(), 2);
    assert_eq!(g.survival.ticks(), 350);
    assert_eq!(g.camp.shelters.len(), 1);
    assert_eq!(g.camp.harvested.len(), 3);
    assert_eq!(g.camp.night_progress, 31);
    assert_eq!(g.inventory.count(Item::MiningTool), 1);
    for x in -35..35 {
        assert_eq!(g.world.tile(x, -31), terrain::generate_legacy(42, x, -31));
    }
    let upgraded = persistence::encode(&g).unwrap();
    assert!(upgraded.starts_with("TERRATOPIA_SAVE 2\nGENERATOR 2\n"));
    assert_eq!(
        persistence::decode(&upgraded).unwrap().inventory,
        g.inventory
    );
    g.handle(Action::Pause);
    for _ in 350..480 {
        g.advance(STEP);
    }
    assert!(g.camp.completed);
    let dir = std::env::temp_dir().join(format!(
        "terratopia-migration-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&dir).unwrap();
    let path = dir.join("legacy.save");
    let on_disk = text.replace('\n', "\r\n");
    fs::write(&path, &on_disk).unwrap();
    assert!(persistence::load(&path).is_ok());
    persistence::save(&g, &path).unwrap();
    assert_eq!(
        fs::read_to_string(dir.join("legacy.save.v1.bak")).unwrap(),
        on_disk
    );
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .starts_with("TERRATOPIA_SAVE 2")
    );
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn v2_roundtrip_preserves_weather_effects_records_and_rejects_invalid_extensions() {
    let mut g = game(827193);
    g.filter_ticks = 400;
    g.resting = true;
    g.handle(Action::Scan);
    let text = persistence::encode(&g).unwrap();
    let mut loaded = persistence::decode(&text).unwrap();
    assert_eq!(loaded.filter_ticks, 400);
    assert!(loaded.resting);
    assert_eq!(loaded.wildlife.active, g.wildlife.active);
    assert_eq!(loaded.weather(), g.weather());
    for invalid in [
        text.replace("SAVE 2", "SAVE 3"),
        text.replace("GENERATOR 3", "GENERATOR 9"),
        text.replace("LIVING 400 1", "LIVING 9999 1"),
        text.replace("ACTORS ", "ACTORS 999999 "),
    ] {
        assert!(persistence::decode(&invalid).is_err());
    }
    let legacy = include_str!("fixtures/phase2.save").replace("GENERATOR 2", "GENERATOR 3");
    assert!(persistence::decode(&legacy).is_err());
}

#[test]
fn a_shelter_built_on_a_harvested_plant_roundtrips() {
    let mut g = game(827193);
    position(&mut g, 5, 0);
    g.handle(Action::Interact);
    g.inventory.add(Item::ShelterKit, 1).unwrap();
    g.handle(Action::Build);
    assert_eq!(g.camp.shelters.len(), 1);
    let mut loaded = persistence::decode(&persistence::encode(&g).unwrap()).unwrap();
    assert!(loaded.camp.sheltered((loaded.player.x, loaded.player.y)));
    assert_eq!(
        loaded
            .camp
            .tile(&mut loaded.world, loaded.player.x, loaded.player.y)
            .landmark,
        None
    );
}

#[test]
fn lethal_plant_contact_enters_death_before_a_modal_can_pause_it() {
    let mut g = game(827193);
    position(&mut g, 9, -1);
    g.survival.health = 2;
    g.handle(Action::Interact);
    assert_eq!(g.state, State::Dead);
    assert_eq!(g.survival.health, 0);
    g.handle(Action::Inventory);
    assert_eq!(g.state, State::Dead);
}
