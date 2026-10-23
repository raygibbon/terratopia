use std::collections::BTreeSet;
use terrabox::{Color, Key, KeyEvent, Modifiers, Size, Surface};
use terratopia::{
    exploration::{Exploration, SCAN_RADIUS, SIGHT_RADIUS, Visibility},
    game::{Game, State},
    input::{self, Action},
    renderer,
    world::{
        World,
        landmark::{self, Landmark, Resource},
        terrain::generate,
    },
};
fn playing() -> Game {
    let mut game = Game::new(827193);
    game.handle(Action::NewWorld);
    game.handle(Action::Accept);
    game
}
#[test]
fn optimized_chunks_match_reference_at_every_cell_and_world_edge() {
    for seed in [0, 827193, u64::MAX] {
        let mut world = World::new(seed, 1);
        for (bx, by) in [
            (-64, -32),
            (-32, 0),
            (0, 0),
            (32, 64),
            (96, -128),
            (i64::MIN, i64::MIN),
            (i64::MAX - 31, i64::MAX - 31),
        ] {
            for y in 0..32 {
                for x in 0..32 {
                    assert_eq!(
                        world.tile(bx + x, by + y),
                        generate(seed, bx + x, by + y),
                        "seed={seed} at {},{}",
                        bx + x,
                        by + y
                    );
                }
            }
        }
    }
}
#[test]
fn all_landmark_types_exist_and_have_non_colour_resource_identity() {
    let mut kinds = BTreeSet::new();
    for y in -120..120 {
        for x in -120..120 {
            if let Some(kind) = generate(827193, x, y).landmark {
                kinds.insert(kind);
            }
        }
    }
    for kind in [
        Landmark::Ruin,
        Landmark::Arch,
        Landmark::Crater,
        Landmark::Deposit(Resource::Iron),
        Landmark::Deposit(Resource::Copper),
        Landmark::Deposit(Resource::Silica),
    ] {
        assert!(kinds.contains(&kind), "missing {kind:?}");
    }
    let resources = [Resource::Iron, Resource::Copper, Resource::Silica];
    assert_eq!(
        resources
            .map(|r| r.glyph())
            .into_iter()
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
    assert_eq!(
        resources
            .map(|r| r.colour())
            .into_iter()
            .collect::<BTreeSet<_>>()
            .len(),
        3
    );
}
#[test]
fn landmark_footprints_cross_positive_and_negative_chunk_boundaries() {
    for boundary in [-64, -32, 32, 64] {
        let mut found = false;
        let mut first = World::new(827193, 1);
        let mut reverse = World::new(827193, 1);
        for y in -300..300 {
            let left = generate(827193, boundary - 1, y);
            let right = generate(827193, boundary, y);
            if left.landmark.is_some()
                && right.landmark.is_some()
                && landmark::at(827193, boundary - 1, y) == landmark::at(827193, boundary, y)
            {
                assert_eq!(first.tile(boundary - 1, y), left);
                assert_eq!(first.tile(boundary, y), right);
                assert_eq!(reverse.tile(boundary, y), right);
                assert_eq!(reverse.tile(boundary - 1, y), left);
                found = true;
                break;
            }
        }
        assert!(found, "no footprint crossing {boundary}");
    }
}
#[test]
fn generated_is_not_explored_and_memory_survives_eviction() {
    let mut world = World::new(42, 1);
    let mut fog = Exploration::default();
    world.tile(-32, -32);
    assert_eq!(fog.visibility(-32, -32), Visibility::Unknown);
    fog.reveal(&mut world, -32, -32, false);
    assert_eq!(fog.visibility(-32, -32), Visibility::Visible);
    assert_eq!(fog.visibility(-32 + SIGHT_RADIUS, -32), Visibility::Visible);
    assert_eq!(
        fog.visibility(-32 + SIGHT_RADIUS + 1, -32),
        Visibility::Unknown
    );
    for x in 100..110 {
        world.focus(x * 32, 0);
        world.tile(x * 32, 0);
    }
    assert_eq!(fog.visibility(-32, -32), Visibility::Visible);
    fog.reveal(&mut world, 200, 200, false);
    assert_eq!(fog.visibility(-32, -32), Visibility::Remembered);
    assert_eq!(world.cached_chunks(), 1);
}
#[test]
fn scan_reveals_a_circle_and_reports_deduplicated_discoveries() {
    let mut world = World::new(827193, 2);
    let mut fog = Exploration::default();
    let normal = fog.reveal(&mut world, 0, 0, false);
    assert_eq!(normal.new_cells, 149);
    let scan = fog.reveal(&mut world, 0, 0, true);
    assert_eq!(fog.explored_cells(), 1257);
    assert_eq!(scan.new_cells, 1257 - 149);
    assert_eq!(fog.visibility(SCAN_RADIUS, 0), Visibility::Visible);
    assert_eq!(fog.visibility(SCAN_RADIUS, 1), Visibility::Unknown);
    assert_eq!(fog.visibility(SCAN_RADIUS + 1, 0), Visibility::Unknown);
    let unique: BTreeSet<_> = scan.sites.iter().map(|s| (s.x, s.y)).collect();
    assert!(!unique.is_empty());
    assert_eq!(unique.len(), scan.sites.len());
    assert_eq!(fog.discovered_sites(), unique.len());
    let again = fog.reveal(&mut world, 0, 0, true);
    assert_eq!(again.new_cells, 0);
    assert_eq!(again.new_sites, 0);
    assert_eq!(again.sites, scan.sites);
    fog.reveal(&mut world, 1, 0, false);
    assert_eq!(fog.visibility(-SCAN_RADIUS, 0), Visibility::Remembered);
}
#[test]
fn visibility_handles_extreme_coordinates_and_crosses_fog_chunks() {
    let mut world = World::new(u64::MAX, 4);
    let mut fog = Exploration::default();
    for center in [(i64::MIN, i64::MIN), (i64::MAX, i64::MAX), (-32, -32)] {
        fog.reveal(&mut world, center.0, center.1, true);
        assert_eq!(fog.visibility(center.0, center.1), Visibility::Visible);
    }
    assert_eq!(fog.visibility(i64::MIN, i64::MIN), Visibility::Remembered);
    assert_eq!(fog.visibility(-33, -33), Visibility::Visible);
    assert_eq!(fog.visibility(-31, -31), Visibility::Visible);
}
#[test]
fn scanner_is_keyboard_driven_ignored_while_paused_and_resets_on_new_world() {
    assert_eq!(
        input::action(KeyEvent::new(Key::Char('R'), Modifiers::NONE)),
        Some(Action::Scan)
    );
    let mut game = playing();
    let initial = game.exploration.explored_cells();
    game.handle(Action::Pause);
    assert!(!game.handle(Action::Scan));
    assert!(game.last_scan.is_none());
    game.handle(Action::Pause);
    assert!(game.handle(Action::Scan));
    assert!(game.exploration.explored_cells() > initial);
    assert!(game.message.starts_with("Scan:"));
    game.handle(Action::Pause);
    game.handle(Action::Menu);
    game.handle(Action::NewWorld);
    game.handle(Action::Accept);
    assert_eq!(game.state, State::Playing);
    assert!(game.last_scan.is_none());
    assert_eq!(game.exploration.explored_cells(), initial);
}
#[test]
fn renderer_distinguishes_unknown_remembered_visible_without_revealing_cells() {
    let mut game = playing();
    let mut surface = Surface::new(Size {
        width: 100,
        height: 30,
    });
    let size = surface.size();
    let count = game.exploration.explored_cells();
    renderer::render(&mut game, &mut surface, size);
    assert_eq!(surface.cell(0, 3).unwrap().scalar(), '?');
    assert_eq!(
        surface.cell(0, 3).unwrap().style.foreground,
        Color::Indexed(8)
    );
    assert_eq!(game.exploration.explored_cells(), count);
    game.handle(Action::Scan);
    game.handle(Action::Move(1, 0));
    renderer::render(&mut game, &mut surface, size);
    // Old scan's left rim is now remembered; normal sight is coloured.
    let remembered = surface.cell(29, 15).unwrap();
    assert_ne!(remembered.scalar(), '?');
    assert_eq!(remembered.style.foreground, Color::Indexed(8));
    assert_eq!(surface.cell(50, 15).unwrap().scalar(), '@');
    assert_ne!(
        surface.cell(49, 15).unwrap().style.foreground,
        Color::Indexed(8)
    );
}
#[test]
fn cache_statistics_measure_hits_misses_and_evictions() {
    let mut world = World::new(5, 1);
    world.tile(0, 0);
    world.tile(1, 0);
    world.tile(32, 0);
    world.tile(0, 0);
    let stats = world.cache_stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 3);
    assert_eq!(stats.evictions, 2);
}

#[test]
fn scan_panel_keeps_the_player_visible_at_the_world_boundary() {
    let mut game = playing();
    game.player.x = i64::MAX;
    game.player.y = 0;
    game.world.focus(game.player.x, game.player.y);
    game.handle(Action::Scan);
    let size = Size {
        width: 80,
        height: 18,
    };
    let mut surface = Surface::new(size);
    renderer::render(&mut game, &mut surface, size);
    assert_eq!(surface.cell(79, 9).unwrap().scalar(), '@');
}
