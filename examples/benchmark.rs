//! Reproducible CPU workloads; excludes terminal I/O and emulator rasterization.
use std::{hint::black_box, time::Instant};
use terrabox::{Size, Surface};
use terratopia::{game::Game, renderer, world::World};
fn main() {
    let start = Instant::now();
    for round in 0..5 {
        let mut world = World::new(827193, 128);
        for x in 0..100 {
            black_box(world.tile(x * 32, round * 32));
        }
    }
    println!("cold chunks: 500 in {:?}", start.elapsed());
    let mut world = World::new(827193, 128);
    world.tile(0, 0);
    let start = Instant::now();
    for n in 0..1_000_000 {
        black_box(world.tile(n % 32, (n / 32) % 32));
    }
    println!("warm lookups: 1000000 in {:?}", start.elapsed());
    let start = Instant::now();
    for x in 0..500 {
        world.focus(x * 32, 0);
        black_box(world.tile(x * 32, 0));
    }
    println!(
        "cache travel: 500 chunks in {:?}, retained {}",
        start.elapsed(),
        world.cached_chunks()
    );
    println!("cache counters: {:?}", world.cache_stats());
    let mut game = Game::new(827193);
    game.handle(terratopia::input::Action::NewWorld);
    game.handle(terratopia::input::Action::Accept);
    let size = Size {
        width: 100,
        height: 30,
    };
    let mut surface = Surface::new(size);
    renderer::render(&mut game, &mut surface, size);
    let start = Instant::now();
    for _ in 0..300 {
        game.player.x += 1;
        game.observe(false);
        game.refresh_wildlife();
        game.world.focus(game.player.x, game.player.y);
        renderer::render(&mut game, &mut surface, size);
        black_box(&surface);
    }
    println!(
        "scrolling logical redraws: 300 at 100x30 in {:?}",
        start.elapsed()
    );
    let mut scanner = Game::new(827193);
    scanner.handle(terratopia::input::Action::NewWorld);
    scanner.handle(terratopia::input::Action::Accept);
    let start = Instant::now();
    for _ in 0..500 {
        black_box(scanner.handle(terratopia::input::Action::Scan));
    }
    println!("scans: 500 full scan actions in {:?}", start.elapsed());
    let mut simulation = Game::new(827193);
    simulation.handle(terratopia::input::Action::NewWorld);
    simulation.handle(terratopia::input::Action::Accept);
    let point = simulation.camp.offset(12, 0);
    let initial = simulation.wildlife.active.len();
    let start = Instant::now();
    for tick in (4..=4000).step_by(4) {
        black_box(simulation.wildlife.tick(
            &mut simulation.world,
            &mut simulation.camp,
            point,
            tick,
        ));
    }
    println!(
        "creature simulation: 1000 one-second updates in {:?}; {} active, {} alive at end",
        start.elapsed(),
        initial,
        simulation
            .wildlife
            .active
            .iter()
            .filter(|c| c.state.health > 0)
            .count()
    );
    let mut movement = Game::new(827193);
    movement.handle(terratopia::input::Action::NewWorld);
    movement.handle(terratopia::input::Action::Accept);
    let start = Instant::now();
    for i in 0..1000 {
        movement.survival.energy = 100;
        black_box(movement.handle(terratopia::input::Action::Move(
            if i % 2 == 0 { 1 } else { -1 },
            0,
        )));
    }
    println!(
        "game movement: 1000 clear-ground steps in {:?}",
        start.elapsed()
    );
}
