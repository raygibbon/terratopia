use crate::game::Game;
use terrabox::{Canvas, Color, Style};
pub fn style(foreground: u8, background: u8) -> Style {
    Style::new(Color::Indexed(foreground), Color::Indexed(background))
}
pub fn hud(canvas: &mut Canvas<'_>, game: &mut Game) {
    let biome = game.world.tile(game.player.x, game.player.y).biome;
    let size = canvas.size();
    canvas.put_str(
        0,
        0,
        &format!(" TERRATOPIA | {} | Seed {}", game.planet, game.world.seed()),
        style(14, 0),
    );
    canvas.put_str(
        0,
        1,
        &format!(
            " HP {}  O2 {}  Energy {} | Day {}",
            game.survival.health,
            game.survival.oxygen,
            game.survival.energy,
            game.survival.day()
        ),
        style(15, 0),
    );
    canvas.put_str(
        0,
        2,
        &format!(
            " X {}  Y {} | {}",
            game.player.x,
            game.player.y,
            biome.name()
        ),
        style(11, 0),
    );
    canvas.put_str(0, size.height.saturating_sub(2), game.message, style(7, 0));
    canvas.put_str(
        0,
        size.height.saturating_sub(1),
        "WASD/Arrows Move | P Pause | Q/Esc Quit",
        style(14, 0),
    );
}
