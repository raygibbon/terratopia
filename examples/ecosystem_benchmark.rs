//! Release workloads with real terrain, feeding, hunting, weather and bounded A*.
//! The 1000 fixture deliberately activates more regions than normal gameplay.
use std::{collections::BTreeSet, hint::black_box, mem::size_of, time::Instant};
use terrabox::{Size, Surface};
use terratopia::{
    game::Game,
    input::Action,
    planet::Camp,
    renderer,
    wildlife::{Creature, CreatureState, Population, Wildlife},
    world::World,
};
fn main() {
    for count in [100, 1000] {
        let mut g = Game::new(827193);
        g.handle(Action::NewWorld);
        g.handle(Action::Accept);
        let mut w = Wildlife::default();
        let mut occupied = BTreeSet::new();
        // Deterministic distinct residents in passable natural habitats.
        'fill: for ring in 0i64..100 {
            for y in -ring..=ring {
                for x in -ring..=ring {
                    if x.abs().max(y.abs()) != ring {
                        continue;
                    }
                    for slot in (0..3).chain(6..18) {
                        if let Some(mut c) =
                            Wildlife::prototype(&mut g.world, &g.camp, (x, y, slot))
                            && occupied.insert(c.home)
                        {
                            c.state.needs.hunger = 55;
                            c.state.needs.age = 600;
                            w.active.push(c);
                            if w.active.len() == count {
                                break 'fill;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(w.active.len(), count);
        w.active.sort_by_key(|c| c.state.id);
        for c in &w.active {
            w.memory.insert(c.state.id, c.state);
        }
        w.migrate_populations(&mut g.world, &g.camp);
        w.rebuild_index();
        let regions = w.populations.len();
        let cache_before = g.world.cache_stats();
        let mut samples = Vec::new();
        for tick in 1..=240 {
            let start = Instant::now();
            black_box(w.tick(&mut g.world, &mut g.camp, (g.player.x, g.player.y), tick));
            samples.push(start.elapsed().as_nanos());
            w.events.drain().for_each(drop);
        }
        let total: u128 = samples.iter().sum();
        samples.sort_unstable();
        let alive = w.active.iter().filter(|c| c.state.health > 0).count();
        println!(
            "scenario {count}: {regions} regions; 240 ticks (60 decisions/animal max), initial {count}, final {} / {alive} alive",
            w.active.len()
        );
        println!(
            "tick total_ms={:.3} mean_ms={:.4} p95_ms={:.4} max_ms={:.4}",
            total as f64 / 1e6,
            total as f64 / 240e6,
            samples[227] as f64 / 1e6,
            samples[239] as f64 / 1e6
        );
        println!(
            "index_ms={:.3} ai_inclusive_ms={:.3} query_ms={:.3} path_ms={:.3} decisions={} queries={} expansions={} overflows={}",
            w.work.index_ns as f64 / 1e6,
            w.work.ai_ns as f64 / 1e6,
            w.work.query_ns as f64 / 1e6,
            w.work.path_ns as f64 / 1e6,
            w.work.decisions,
            w.work.queries,
            w.work.path_expansions,
            w.event_overflows
        );
        let cache_after = g.world.cache_stats();
        println!(
            "terrain_cache_misses={} evictions={}",
            cache_after.misses - cache_before.misses,
            cache_after.evictions - cache_before.evictions
        );
        println!(
            "entity_arena_bytes={} dense_capacity_bytes={} remembered_payload_bytes={} (excludes tree nodes, strings, allocator)",
            w.entity_storage_bytes(),
            w.active.capacity() * size_of::<Creature>(),
            w.memory.len() * (size_of::<CreatureState>())
        );
        let start = Instant::now();
        for c in w.active.iter().cycle().take(10000) {
            black_box(w.nearby(c.state.position, 10));
        }
        println!(
            "10000 indexed nearby queries_ms={:.3}",
            start.elapsed().as_secs_f64() * 1e3
        );
        g.wildlife = w;
        let size = Size {
            width: 100,
            height: 30,
        };
        let mut surface = Surface::new(size);
        renderer::render(&mut g, &mut surface, size);
        let start = Instant::now();
        for _ in 0..300 {
            renderer::render(&mut g, &mut surface, size);
            black_box(&surface);
        }
        println!(
            "300 logical redraws 100x30_ms={:.3} (no terminal I/O)",
            start.elapsed().as_secs_f64() * 1e3
        );
    }
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
    let mut world = World::new(827193, 1);
    let mut camp = Camp::new((0, 0));
    let start = Instant::now();
    for tick in 1..=24000 {
        black_box(w.tick(&mut world, &mut camp, (0, 0), tick));
    }
    println!(
        "scenario 10000 dormant: 500 summaries, 24000 ticks_ms={:.3} coarse_region_updates={} decisions={} individual_records={} summary_payload_bytes={}",
        start.elapsed().as_secs_f64() * 1e3,
        w.work.coarse_regions,
        w.work.decisions,
        w.memory.len(),
        500 * size_of::<((i64, i64), Population)>()
    );
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status
            .lines()
            .filter(|l| l.starts_with("VmHWM:") || l.starts_with("VmRSS:"))
        {
            println!("{line}");
        }
    }
}
