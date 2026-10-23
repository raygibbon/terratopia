use std::collections::BTreeSet;
use terratopia::{
    game::{Game, STEP},
    input::Action,
    persistence,
    planet::Camp,
    wildlife::{
        Activity, Behaviour, EcosystemEvent, MAX_MEMORY, PATH_BUDGET, Population, Wildlife,
    },
    world::World,
};
fn game() -> Game {
    let mut g = Game::new(827193);
    g.handle(Action::NewWorld);
    g.handle(Action::Accept);
    g
}
fn animal(g: &mut Game, slot: u8) -> terratopia::wildlife::Creature {
    Wildlife::prototype(
        &mut g.world,
        &g.camp,
        (g.camp.origin.0, g.camp.origin.1, slot),
    )
    .unwrap()
}
#[test]
fn runtime_generation_changes_but_persistent_identity_and_movement_survive() {
    let mut g = game();
    let id = g.wildlife.active[0].state.id;
    let handle = g.wildlife.entity(id).unwrap();
    let state = g.wildlife.memory[&id];
    g.wildlife.refresh(&mut g.world, &g.camp, (9000, 9000));
    assert!(g.wildlife.entity(id).is_none());
    g.wildlife.refresh(&mut g.world, &g.camp, g.camp.origin);
    assert_eq!(g.wildlife.memory[&id], state);
    assert_ne!(g.wildlife.entity(id).unwrap(), handle);
    let i = g
        .wildlife
        .active
        .iter()
        .position(|c| c.state.id == id)
        .unwrap();
    let p = g.wildlife.active[i].state.position;
    assert!(g.wildlife.nearby(p, 0).contains(&i));
}
#[test]
fn hungry_predator_kills_eats_and_scavenger_consumes_one_corpse() {
    let mut g = game();
    let mut prey = animal(&mut g, 3);
    let mut predator = animal(&mut g, 5);
    prey.state.position = g.camp.offset(13, 0);
    prey.home = prey.state.position;
    prey.state.health = 1;
    predator.state.position = g.camp.offset(14, 0);
    predator.home = predator.state.position;
    predator.state.needs.hunger = 80;
    let mut w = Wildlife::default();
    w.active = vec![prey, predator];
    w.tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), 4);
    assert_eq!(w.active[0].state.health, 0);
    assert!(w.active[0].state.looted);
    assert_eq!(w.active[1].state.needs.activity, Activity::Eat);
    let events: Vec<_> = w.events.drain().collect();
    let attacked = events
        .iter()
        .position(|e| matches!(e, EcosystemEvent::CreatureAttacked(..)))
        .unwrap();
    let died = events
        .iter()
        .position(|e| matches!(e, EcosystemEvent::CreatureDied(..)))
        .unwrap();
    let ate = events
        .iter()
        .position(|e| matches!(e, EcosystemEvent::CreatureAte(..)))
        .unwrap();
    assert!(attacked < died && died < ate);
    w.active[0].state.looted = false;
    w.active[1].species.behaviour = Behaviour::Scavenger;
    w.active[1].state.needs.hunger = 80;
    w.tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), 8);
    assert!(w.active[0].state.looted);
    let hunger = w.active[1].state.needs.hunger;
    w.tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), 12);
    assert!(w.active[1].state.needs.hunger >= hunger);
    let p = w.active[1].state.position;
    w.attack((p.0 - 1, p.1)).unwrap();
    assert!(
        !w.active[1].state.angry,
        "scavengers flee instead of gaining an invalid hostile flag"
    );
}
#[test]
fn feeding_uses_shared_depletion_and_rest_recovers_energy() {
    let mut g = game();
    let mut c = animal(&mut g, 3);
    c.state.position = g.camp.offset(6, 0);
    c.state.needs.hunger = 80;
    let food = g.camp.offset(5, 0);
    let mut w = Wildlife::default();
    w.active.push(c);
    w.tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), 4);
    assert!(g.camp.plants.contains(&food));
    assert_eq!(w.active[0].state.needs.activity, Activity::Eat);
    w.active[0].state.needs.energy = 0;
    w.tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), 8);
    assert_eq!(w.active[0].state.needs.activity, Activity::Rest);
    assert!(w.active[0].state.needs.energy > 0);
    assert!(
        g.camp
            .harvest_plant(&mut g.world, &mut g.inventory, food)
            .is_err()
    );
}
#[test]
fn deterministic_ai_and_paths_have_a_shared_tick_budget() {
    let mut a = game();
    let mut b = game();
    for tick in 1..=1200 {
        a.wildlife
            .tick(&mut a.world, &mut a.camp, (a.player.x, a.player.y), tick);
        b.wildlife
            .tick(&mut b.world, &mut b.camp, (b.player.x, b.player.y), tick);
        a.wildlife.events.drain().for_each(drop);
        b.wildlife.events.drain().for_each(drop);
    }
    assert_eq!(a.wildlife.active, b.wildlife.active);
    assert_eq!(a.camp.plants, b.camp.plants);
    assert!(a.wildlife.work.path_expansions <= 300 * PATH_BUDGET as u64);
    assert_eq!(a.wildlife.populations, b.wildlife.populations);
}
#[test]
fn dormant_ten_thousand_population_work_is_per_region() {
    let mut w = Wildlife::default();
    for x in 0..500 {
        w.populations.insert(
            (x, 0),
            Population {
                counts: [16, 1, 2, 1],
                food: 120,
                capacity: 24,
                epoch: 0,
                next_slot: 32,
            },
        );
    }
    w.advance_dormant(240);
    assert_eq!(w.work.coarse_regions, 500);
    assert_eq!(w.work.decisions, 0);
    assert!(w.active.is_empty());
    assert!(w.memory.is_empty());
    assert!(
        w.populations
            .values()
            .all(|p| p.counts.iter().sum::<u16>() <= 24)
    );
}
#[test]
fn kills_food_and_needs_survive_dormancy_eviction_and_save_resume() {
    let mut g = game();
    let p = g.camp.offset(15, 0);
    g.player.x = p.0;
    g.player.y = p.1;
    g.refresh_wildlife();
    let id = (g.camp.origin.0, g.camp.origin.1, 5);
    for _ in 0..4 {
        g.handle(Action::Attack);
    }
    assert_eq!(g.wildlife.memory[&id].health, 0);
    let food = g.camp.offset(5, 0);
    g.camp.plants.insert(food);
    g.wildlife.refresh(&mut g.world, &g.camp, (9000, 9000));
    for tick in 1..=960 {
        g.wildlife
            .tick(&mut g.world, &mut g.camp, (9000, 9000), tick);
    }
    for x in 0..200 {
        g.world.tile(x * 256, 0);
    }
    g.refresh_wildlife();
    assert_eq!(g.wildlife.memory[&id].health, 0);
    assert!(g.camp.plants.contains(&food));
    // Save through the real Game clock; the survival time is authoritative.
    g.survival = terratopia::survival::Survival::from_parts(100, 100, 100, 960).unwrap();
    let text = persistence::encode(&g).unwrap();
    let loaded = persistence::decode(&text).unwrap();
    assert_eq!(loaded.wildlife.memory, g.wildlife.memory);
    assert_eq!(
        loaded
            .wildlife
            .active
            .iter()
            .map(|c| c.state.id)
            .collect::<BTreeSet<_>>()
            .len(),
        loaded.wildlife.active.len()
    );
}
#[test]
fn save_resume_decisions_match_uninterrupted_and_resize_is_read_only() {
    let mut g = game();
    for _ in 0..240 {
        g.advance(STEP);
    }
    let text = persistence::encode(&g).unwrap();
    let mut resumed = persistence::decode(&text).unwrap();
    resumed.handle(Action::Pause);
    for _ in 0..200 {
        g.advance(STEP);
        resumed.advance(STEP);
    }
    assert_eq!(g.wildlife.memory, resumed.wildlife.memory);
    assert_eq!(g.wildlife.populations, resumed.wildlife.populations);
    let before = persistence::encode(&g).unwrap();
    let mut surface = terrabox::Surface::new(terrabox::Size {
        width: 0,
        height: 0,
    });
    for (width, height) in [(0, 0), (80, 24), (20, 6), (120, 40)] {
        terratopia::renderer::render(&mut g, &mut surface, terrabox::Size { width, height });
    }
    assert_eq!(persistence::encode(&g).unwrap(), before);
}
#[test]
fn accelerated_populations_are_bounded_and_births_use_new_ids() {
    let mut g = game();
    // Controlled favourable habitat and hardy animals isolate demographic rules
    // from the independent hostile-weather mortality tests.
    for c in &mut g.wildlife.active {
        c.species.resistance = 100;
        c.species.temperature = 1000;
        c.species.lifespan = 100_000;
        c.species.maturity = 0;
        c.species.birth_period = 240;
        c.state.needs.age = 600;
    }
    let original: BTreeSet<_> = g.wildlife.memory.keys().copied().collect();
    g.wildlife.events.drain().for_each(drop);
    let mut births = 0;
    for tick in (4..=4_800).step_by(4) {
        for c in &mut g.wildlife.active {
            c.state.needs.hunger = 20;
            c.state.needs.energy = 80;
        }
        g.wildlife
            .tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), tick);
        births += g
            .wildlife
            .events
            .drain()
            .filter(|e| matches!(e, EcosystemEvent::CreatureSpawned(..)))
            .count();
        assert!(g.wildlife.memory.len() <= MAX_MEMORY);
        assert!(
            g.wildlife
                .populations
                .values()
                .all(|p| p.counts.iter().sum::<u16>() <= 24)
        );
    }
    assert!(births > 0);
    assert!(
        g.wildlife
            .memory
            .keys()
            .filter(|id| !original.contains(id))
            .all(|id| id.2 >= 32)
    );
}
#[test]
fn death_and_weather_have_persistent_consequences() {
    let mut g = game();
    let mut c = animal(&mut g, 3);
    c.state.health = 1;
    c.species.lifespan = 0;
    c.state.needs.hunger = 80;
    let id = c.state.id;
    let mut w = Wildlife::default();
    w.active.push(c);
    w.tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), 4);
    assert_eq!(w.memory[&id].health, 0);
    assert!(!w.memory[&id].looted);
    assert!(
        !w.events
            .drain()
            .any(|e| matches!(e, EcosystemEvent::CreatureAte(..)))
    );
    let mut exposed = animal(&mut g, 3);
    exposed.species.resistance = 0;
    exposed.species.temperature = 0;
    let before = exposed.state.health;
    w = Wildlife::default();
    w.active.push(exposed);
    for tick in (32..=1024).step_by(32) {
        w.tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), tick);
    }
    assert!(w.active[0].state.health < before);
}
#[test]
fn loaded_resource_and_reference_limits_are_checked() {
    let g = game();
    let text = persistence::encode(&g).unwrap();
    let mut rows: Vec<String> = text.lines().map(str::to_owned).collect();
    for row in &mut rows {
        if row.starts_with("POPULATION ") {
            let mut fields: Vec<_> = row.split_whitespace().map(str::to_owned).collect();
            if fields[3..7].iter().any(|v| v != "0") {
                for field in &mut fields[3..7] {
                    *field = "0".to_string();
                }
                *row = fields.join(" ");
                break;
            }
        }
    }
    assert!(persistence::decode(&rows.join("\n")).is_err());
    for bad in [
        text.replace("POPULATIONS ", "POPULATIONS 999999 "),
        text.replace("ECOSYSTEM 0", "ECOSYSTEM 999"),
        text.replace("POPULATION ", "BAD "),
    ] {
        assert!(persistence::decode(&bad).is_err());
    }
}
#[test]
fn unvisited_region_activation_order_does_not_change_generation() {
    let mut a = Wildlife::default();
    let mut b = Wildlife::default();
    let mut wa = World::new(827193, 1);
    let mut wb = World::new(827193, 128);
    let camp = Camp::new(wa.landing_site());
    let points = [(200, 0), (-200, 0), (0, 200)];
    for p in points {
        a.refresh(&mut wa, &camp, p);
    }
    for p in points.into_iter().rev() {
        b.refresh(&mut wb, &camp, p);
    }
    a.refresh(&mut wa, &camp, camp.origin);
    b.refresh(&mut wb, &camp, camp.origin);
    assert_eq!(a.memory, b.memory);
    assert_eq!(a.populations, b.populations);
    assert_eq!(a.active, b.active);
}

#[test]
fn genuine_v2_fixture_migrates_identity_and_keeps_a_non_overwriting_backup() {
    let source = include_str!("fixtures/phase3.save");
    let g = persistence::decode(source).unwrap();
    assert!(g.wildlife.active.iter().all(|c| c.state.id.2 <= 5));
    assert!(
        g.wildlife
            .active
            .iter()
            .all(|c| g.wildlife.memory.get(&c.state.id) == Some(&c.state))
    );
    let upgraded = persistence::encode(&g).unwrap();
    assert!(upgraded.starts_with("TERRATOPIA_SAVE 3"));
    let loaded = persistence::decode(&upgraded).unwrap();
    assert_eq!(loaded.wildlife.active, g.wildlife.active);
    let dir = std::env::temp_dir().join(format!("terratopia-v2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("old.save");
    std::fs::write(&path, source).unwrap();
    persistence::save(&g, &path).unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("old.save.v2.bak")).unwrap(),
        source
    );
    std::fs::write(&path, source).unwrap();
    std::fs::write(dir.join("old.save.v2.bak"), "different").unwrap();
    assert!(persistence::save(&g, &path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn event_overflow_is_bounded_and_observable() {
    let mut w = Wildlife::default();
    for _ in 0..5000 {
        w.notify(EcosystemEvent::WeatherChanged(0));
    }
    assert_eq!(w.events.len(), 4096);
    assert_eq!(w.event_overflows, 904);
}
#[test]
fn starting_far_away_does_not_remove_starter_animals() {
    let mut a = Wildlife::default();
    let mut b = Wildlife::default();
    let mut wa = World::new(827193, 128);
    let mut wb = World::new(827193, 1);
    let camp = Camp::new(wa.landing_site());
    for p in [camp.origin, (200, 0), camp.origin] {
        a.refresh(&mut wa, &camp, p);
    }
    for p in [(200, 0), camp.origin] {
        b.refresh(&mut wb, &camp, p);
    }
    assert_eq!(a.memory, b.memory);
    assert_eq!(a.populations, b.populations);
    assert_eq!(a.active, b.active);
}

#[test]
fn hunting_routes_around_a_new_shelter_using_a_bounded_local_search() {
    let mut g = game();
    let mut c = animal(&mut g, 5);
    c.state.position = g.camp.offset(10, 0);
    c.home = c.state.position;
    c.species.speed = 1;
    c.state.needs.hunger = 30;
    g.camp.shelters.push(terratopia::planet::Shelter {
        position: g.camp.offset(9, 0),
        charge: 0,
    });
    let player = g.camp.offset(6, 0);
    let mut w = Wildlife::default();
    w.active.push(c);
    w.tick(&mut g.world, &mut g.camp, player, 4);
    assert!(w.work.path_expansions > 0);
    assert!(w.work.path_expansions <= 64);
    assert_ne!(w.active[0].state.position, g.camp.offset(9, 0));
    assert_eq!(w.active[0].state.position.1.abs_diff(g.camp.origin.1), 1);
}

#[test]
fn unassisted_ecosystem_survives_thirty_accelerated_minutes() {
    let mut g = game();
    let start = g.wildlife.active.len();
    g.wildlife.events.drain().for_each(drop);
    let mut births = 0;
    for tick in (4..=7200).step_by(4) {
        g.wildlife
            .tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), tick);
        births += g
            .wildlife
            .events
            .drain()
            .filter(|e| matches!(e, EcosystemEvent::CreatureSpawned(..)))
            .count();
        if tick == 240 {
            assert!(
                g.wildlife.populations.values().all(|p| p.next_slot == 32),
                "immature populations must not exhaust birth IDs"
            );
        }
        assert!(g.wildlife.memory.len() <= MAX_MEMORY);
        assert!(
            g.wildlife
                .populations
                .values()
                .all(|p| p.counts.iter().sum::<u16>() <= 24)
        );
    }
    let alive = g
        .wildlife
        .active
        .iter()
        .filter(|c| c.state.health > 0)
        .count();
    println!(
        "Unassisted: initial {start}, alive {alive}, births {births}, retained {}",
        g.wildlife.memory.len()
    );
    assert!(
        births > 0,
        "normal fed adults should reproduce without artificial feeding"
    );
    assert!(
        alive > start / 10,
        "healthy starter habitat should retain a viable population"
    );
    g.survival = terratopia::survival::Survival::from_parts(100, 100, 100, 7200).unwrap();
    let saved = persistence::encode(&g).unwrap();
    let loaded = persistence::decode(&saved).unwrap();
    assert_eq!(loaded.wildlife.memory, g.wildlife.memory);
    assert_eq!(loaded.wildlife.populations, g.wildlife.populations);
}
