use crate::{
    camera::{Camera, usable, viewport},
    game::{Game, State},
    ui::{hud, style},
    world::biome::Biome,
};
use terrabox::{Rect, Size, Surface};
fn appearance(biome: Biome) -> (char, u8, u8) {
    match biome {
        Biome::Grassland => ('.', 10, 0),
        Biome::Forest => ('T', 2, 0),
        Biome::Desert => (':', 11, 3),
        Biome::Mountains => ('^', 7, 8),
        Biome::Water => ('~', 14, 4),
        Biome::Frozen => ('*', 15, 6),
        Biome::Wasteland => (';', 13, 5),
    }
}
pub fn render(game: &mut Game, surface: &mut Surface, size: Size) {
    if surface.size() != size {
        surface.resize(size);
    }
    surface.clear(style(7, 0));
    let mut canvas = surface.canvas(Rect::new(0, 0, size.width, size.height));
    if !usable(size) {
        canvas.put_str(0, 0, "Need 40x12 cells", style(11, 0));
        canvas.put_str(0, 1, "Resize | Q/Esc quit", style(7, 0));
        return;
    }
    match game.state {
        State::Playing | State::Paused => {
            let view = viewport(size);
            let camera = Camera::following(game.player, view);
            {
                let mut map = canvas.clip(view);
                for y in 0..view.height {
                    for x in 0..view.width {
                        let (wx, wy) = camera.world_at(x, y);
                        let (glyph, fg, bg) = appearance(game.world.tile(wx, wy).biome);
                        map.put_char(x, y, glyph, style(fg, bg));
                    }
                }
                let x = (i128::from(game.player.x) - i128::from(camera.x)) as u16;
                let y = (i128::from(game.player.y) - i128::from(camera.y)) as u16;
                map.put_char(x, y, '@', style(15, 1));
            }
            hud(&mut canvas, game);
            if game.state == State::Paused {
                let text = " PAUSED | P/Enter Resume | M Menu ";
                let x = size.width.saturating_sub(text.len() as u16) / 2;
                canvas.put_str(x, size.height / 2, text, style(15, 4));
            }
        }
        State::MainMenu | State::NewWorld => {
            let mut title = style(14, 0);
            title.bold = true;
            canvas.put_str(3, 2, "T E R R A T O P I A", title);
            canvas.put_str(3, 4, "A world beyond the last signal.", style(7, 0));
            if game.state == State::MainMenu {
                canvas.put_str(3, 6, "[Enter / N] New world", style(15, 0));
                canvas.put_str(3, 7, "[Q / Esc]   Quit", style(7, 0));
            } else {
                canvas.put_str(3, 6, &format!("Planet {}", game.planet), style(11, 0));
                canvas.put_str(3, 7, &format!("Seed {}", game.world.seed()), style(7, 0));
                canvas.put_str(3, 9, "[Enter] Land  [M] Menu  [Q] Quit", style(15, 0));
            }
            canvas.put_str(
                3,
                size.height - 2,
                "Terminal planetary exploration / Phase 1",
                style(8, 0),
            );
        }
        State::Quit => {}
    }
}
