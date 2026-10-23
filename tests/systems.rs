use std::time::Duration;
use terrabox::{Key, KeyEvent, Modifiers, Rect, Size, Surface};
use terratopia::{
    camera::{Camera, usable, viewport},
    entities::player::Player,
    game::{Game, STEP, State},
    input::{Action, action},
    renderer,
    world::{World, biome::Biome, chunk::locate, noise, terrain::generate},
};

#[test]
fn generation_is_independent_of_order_cache_and_seed() {
    let points = [
        (-33, -32),
        (-1, 0),
        (31, 31),
        (32, 32),
        (911, -728),
        (i64::MIN, i64::MAX),
    ];
    let mut first = World::new(827193, 2);
    let expected: Vec<_> = points.iter().map(|&(x, y)| first.tile(x, y)).collect();
    let mut second = World::new(827193, 1);
    for (i, &(x, y)) in points.iter().enumerate().rev() {
        assert_eq!(second.tile(x, y), expected[i]);
        assert_eq!(second.tile(x, y), generate(827193, x, y));
        assert!(second.cached_chunks() <= 1);
    }
    assert!((0..200).any(|x| generate(1, x * 17, 999) != generate(2, x * 17, 999)));
}
#[test]
fn chunk_boundaries_match_direct_generation() {
    let mut world = World::new(u64::MAX, 8);
    for x in [-65, -64, -33, -32, -1, 0, 1, 31, 32, 63, 64] {
        for y in -35..35 {
            assert_eq!(world.tile(x, y), generate(u64::MAX, x, y));
        }
    }
    assert_eq!(locate(-1, -33), ((-1, -2), 31 * 32 + 31));
    assert_eq!(locate(32, 0), ((1, 0), 0));
}
#[test]
fn extreme_coordinates_do_not_overflow() {
    let mut world = World::new(u64::MAX, 4);
    for x in [i64::MIN, i64::MIN + 1, i64::MAX - 1, i64::MAX] {
        for y in [i64::MIN, i64::MAX] {
            assert_eq!(world.tile(x, y), generate(u64::MAX, x, y));
            let player = Player { x, y };
            let camera = Camera::following(player, Rect::new(0, 3, 100, 40));
            let (right, bottom) = camera.world_at(99, 39);
            assert!(right >= x && bottom >= y);
            assert!(camera.x <= x && camera.y <= y);
        }
    }
    let mut p = Player { x: i64::MAX, y: 0 };
    assert!(!p.step(1, 0, &mut world));
    p.x = i64::MIN;
    assert!(!p.step(-1, 0, &mut world));
}
#[test]
fn biome_classification_and_collision_rules() {
    for (e, t, m, want) in [
        (0, 32000, 32000, Biome::Water),
        (50000, 32000, 32000, Biome::Mountains),
        (32000, 0, 32000, Biome::Frozen),
        (32000, 50000, 24000, Biome::Desert),
        (32000, 32000, 0, Biome::Wasteland),
        (32000, 32000, 50000, Biome::Forest),
        (32000, 32000, 32000, Biome::Grassland),
    ] {
        assert_eq!(Biome::classify(e, t, m), want);
        assert_eq!(
            want.passable(),
            !matches!(want, Biome::Water | Biome::Mountains)
        );
    }
    let mut seen = std::collections::BTreeSet::new();
    for x in -40..40 {
        for y in -40..40 {
            seen.insert(generate(827193, x * 32, y * 32).biome.name());
        }
    }
    assert_eq!(
        seen.len(),
        7,
        "all biomes should occur in generated terrain"
    );
}
#[test]
fn terrain_is_spatially_correlated() {
    for x in -300..300 {
        assert!((noise::field(12, x, 45) - noise::field(12, x + 1, 45)).abs() < 2000);
    }
}
#[test]
fn cache_evicts_distant_chunks_and_reloads_identically() {
    let mut world = World::new(42, 3);
    let old = world.tile(0, 0);
    world.tile(32, 0);
    world.tile(320, 0);
    world.focus(0, 0);
    world.tile(0, 32);
    assert_eq!(world.cached_chunks(), 3);
    assert!(world.contains_chunk((0, 0)));
    assert!(!world.contains_chunk((10, 0)));
    for x in 0..100 {
        world.focus(x * 32, 0);
        world.tile(x * 32, 0);
        assert!(world.cached_chunks() <= 3);
    }
    assert_eq!(world.tile(0, 0), old);
    let mut minimum = World::new(0, 0);
    minimum.tile(-1, 0);
    minimum.tile(0, 0);
    assert_eq!(minimum.cached_chunks(), 1);
}
#[test]
fn movement_is_one_cardinal_tile_and_collides() {
    let mut world = World::new(827193, 4);
    let mut player = Player::default();
    assert!(player.step(1, 0, &mut world));
    assert_eq!(player, Player { x: 1, y: 0 });
    assert!(player.step(-1, 0, &mut world));
    assert!(player.step(0, -1, &mut world));
    assert_eq!(player, Player { x: 0, y: -1 });
    assert!(!player.step(1, 1, &mut world));
    assert!(!player.step(2, 0, &mut world));
    for y in -300..300 {
        for x in -300..300 {
            if !generate(827193, x, y).biome.passable() {
                let mut player = Player { x: x - 1, y };
                let before = player;
                assert!(!player.step(1, 0, &mut world));
                assert_eq!(player, before);
                return;
            }
        }
    }
    panic!("expected blocked terrain");
}
#[test]
fn camera_center_resize_and_tiny_layout() {
    let player = Player { x: -800, y: 951 };
    for size in [
        Size {
            width: 80,
            height: 24,
        },
        Size {
            width: 41,
            height: 13,
        },
    ] {
        let view = viewport(size);
        assert_eq!(view.height, size.height - 5);
        let c = Camera::following(player, view);
        assert_eq!(
            c.world_at(view.width / 2, view.height / 2),
            (player.x, player.y)
        );
    }
    for height in 0..12 {
        for width in 0..40 {
            let size = Size { width, height };
            let view = viewport(size);
            assert!(view.bottom() <= height);
            assert!(!usable(size));
        }
    }
    assert_eq!(player, Player { x: -800, y: 951 });
}
fn land(game: &mut Game) {
    game.handle(Action::NewWorld);
    assert_eq!(game.state, State::NewWorld);
    game.handle(Action::Accept);
    assert_eq!(game.state, State::Playing);
}
#[test]
fn game_states_pause_restart_and_bounded_fixed_updates() {
    let mut game = Game::new(123);
    let spawn = game.player;
    assert_eq!(game.state, State::MainMenu);
    game.handle(Action::Move(1, 0));
    assert_eq!(game.player, spawn);
    land(&mut game);
    game.handle(Action::Move(1, 0));
    game.handle(Action::Pause);
    assert_eq!(game.state, State::Paused);
    game.handle(Action::Move(1, 0));
    assert_eq!(game.player.x, spawn.x + 1);
    for _ in 0..500 {
        game.advance(STEP);
    }
    assert_eq!(game.survival.day(), 1);
    game.handle(Action::Accept);
    game.advance(Duration::from_secs(10000));
    assert_eq!(game.survival.day(), 1);
    for _ in 0..475 {
        game.advance(STEP);
    }
    assert_eq!(game.survival.day(), 1);
    assert!(game.advance(STEP));
    assert_eq!(game.survival.day(), 2);
    game.handle(Action::Pause);
    game.handle(Action::Menu);
    land(&mut game);
    assert_eq!(game.player, spawn);
    assert_eq!(game.survival.day(), 1);
    game.handle(Action::Quit);
    assert_eq!(game.state, State::Quit);
}
#[test]
fn input_mapping_and_modifiers() {
    let key = |key| action(KeyEvent::new(key, Modifiers::NONE));
    for k in [Key::Up, Key::Char('w'), Key::Char('W')] {
        assert_eq!(key(k), Some(Action::Move(0, -1)));
    }
    assert_eq!(key(Key::Escape), Some(Action::Quit));
    assert_eq!(key(Key::Char('q')), Some(Action::Quit));
    assert_eq!(
        action(KeyEvent::new(Key::Char('C'), Modifiers::CTRL)),
        Some(Action::Quit)
    );
    assert_eq!(action(KeyEvent::new(Key::Char('w'), Modifiers::ALT)), None);
}
#[test]
fn rendering_tracks_player_hud_pause_and_resizes_without_moving_world() {
    let mut game = Game::new(827193);
    land(&mut game);
    let spawn = game.player;
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
            width: 40,
            height: 12,
        },
        Size {
            width: 1,
            height: 1,
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
        renderer::render(&mut game, &mut surface, size);
        if usable(size) {
            let view = viewport(size);
            assert_eq!(
                surface
                    .cell(view.width / 2, view.y + view.height / 2)
                    .unwrap()
                    .scalar(),
                '@'
            );
            let text: String = (0..size.width)
                .map(|x| surface.cell(x, 0).unwrap().scalar())
                .collect();
            assert!(text.contains("TERRATOPIA"));
        }
        assert_eq!(game.player, spawn);
    }
    game.handle(Action::Pause);
    renderer::render(
        &mut game,
        &mut surface,
        Size {
            width: 80,
            height: 24,
        },
    );
    let line: String = (0..80)
        .map(|x| surface.cell(x, 12).unwrap().scalar())
        .collect();
    assert!(line.contains("PAUSED"));
}

#[test]
fn landing_sites_are_stable_and_support_initial_exploration() {
    for seed in [0, 1, 2, 42, 827193, u64::MAX] {
        let mut world = World::new(seed, 2);
        let (x, y) = world.landing_site();
        assert_eq!(world.landing_site(), (x, y));
        assert_eq!(
            world.cached_chunks(),
            0,
            "spawn search should not warm the cache"
        );
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            assert!(world.tile(x + dx, y + dy).biome.passable());
        }
    }
}
