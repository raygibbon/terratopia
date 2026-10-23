use crate::game::Game;
use terrabox::{Canvas, Color, Style};
pub fn style(foreground: u8, background: u8) -> Style {
    Style::new(Color::Indexed(foreground), Color::Indexed(background))
}
pub fn hud(canvas: &mut Canvas<'_>, game: &mut Game) {
    let env = game.environment();
    let biome = env.biome;
    let context = game.conditions();
    let exposure = game.survival.exposure(env, context);
    let weather_label = context.weather.warning.map_or_else(
        || context.weather.current.name().to_string(),
        |s| format!("!{} {s}s", context.weather.next.name()),
    );
    let near_creatures = game
        .wildlife
        .active
        .iter()
        .filter(|c| {
            c.state.health > 0
                && game
                    .exploration
                    .visibility(c.state.position.0, c.state.position.1)
                    == crate::exploration::Visibility::Visible
        })
        .count();
    let size = canvas.size();
    canvas.put_str(
        0,
        0,
        &format!(
            " TERRATOPIA | {} | {} | Seed {}",
            game.planet,
            game.world.profile().archetype.name(),
            game.world.seed()
        ),
        style(14, 0),
    );
    canvas.put_str(
        0,
        1,
        &format!(
            " HP {}  O2 {}  Energy {} | D{} {} {}s | {} | NPC {}",
            game.survival.health,
            game.survival.oxygen,
            game.survival.energy,
            game.survival.day(),
            if game.survival.night() {
                "NIGHT"
            } else {
                "DAY"
            },
            game.survival.ticks() / 4 % 120,
            weather_label,
            near_creatures
        ),
        style(
            if game.survival.oxygen <= 25 || game.survival.health <= 25 {
                9
            } else {
                15
            },
            0,
        ),
    );
    canvas.put_str(
        0,
        2,
        &format!(
            " X {}  Y {} | {} {}C {} F{}s | {}",
            game.player.x,
            game.player.y,
            biome.name(),
            exposure.temperature,
            if env.sheltered {
                "SHELTER"
            } else if context.crash_shield {
                "SHIELD"
            } else if exposure.damage > 0 {
                "HAZARD"
            } else {
                "SAFE"
            },
            game.camp.power_remaining((game.player.x, game.player.y)),
            if game.camp.completed {
                "Night survived!"
            } else if !game.camp.cache_claimed {
                "E: scavenge A"
            } else if game.inventory.count(crate::inventory::Item::MiningTool) == 0 {
                "C: mining tool"
            } else if game.camp.shelters.is_empty() {
                "C: kit / B: shelter"
            } else {
                "Stay on H all night"
            }
        ),
        style(11, 0),
    );
    let warning = if game.survival.oxygen == 0 {
        "SUFFOCATING! O: oxygen. "
    } else if game.survival.oxygen <= 25 {
        "LOW OXYGEN! O: canister. "
    } else {
        ""
    };
    canvas.put_str(
        0,
        size.height.saturating_sub(2),
        &format!("{warning}{}", game.message),
        style(if warning.is_empty() { 7 } else { 9 }, 0),
    );
    canvas.put_str(
        0,
        size.height.saturating_sub(1),
        "E Interact | F Fight | Z Rest | R Scan | M Survey | I/C/B | H Help | P | Q Quit",
        style(14, 0),
    );
}

pub fn screen(canvas: &mut Canvas<'_>, game: &mut Game) {
    use crate::{crafting::RECIPES, game::State, inventory::CAPACITY};
    let size = canvas.size();
    canvas.fill(style(7, 0));
    let height = size.height.saturating_sub(6).max(1) as usize;
    match game.state {
        State::Inventory => {
            canvas.put_str(
                0,
                0,
                &format!(
                    "INVENTORY {}/{} stacks (time paused)",
                    game.inventory.stacks().len(),
                    CAPACITY
                ),
                style(14, 0),
            );
            let top = game.selected.saturating_sub(height - 1);
            for (i, stack) in game
                .inventory
                .stacks()
                .iter()
                .enumerate()
                .skip(top)
                .take(height)
            {
                canvas.put_str(
                    1,
                    2 + (i - top) as u16,
                    &format!(
                        "{} {} x{}",
                        if i == game.selected { ">" } else { " " },
                        stack.item.name(),
                        stack.count
                    ),
                    style(if i == game.selected { 11 } else { 7 }, 0),
                );
            }
            canvas.put_str(
                0,
                size.height - 4,
                "Enter Use | X Discard stack | I/Back Close",
                style(14, 0),
            );
            canvas.put_str(
                0,
                size.height - 3,
                "Tools and thermal suits work while carried.",
                style(7, 0),
            );
        }
        State::Crafting => {
            canvas.put_str(0, 0, "CRAFTING (time paused)", style(14, 0));
            let top = game.selected.saturating_sub(height - 1);
            for (i, recipe) in RECIPES.iter().enumerate().skip(top).take(height) {
                canvas.put_str(
                    1,
                    2 + (i - top) as u16,
                    &format!(
                        "{} {}",
                        if i == game.selected { ">" } else { " " },
                        recipe.output.name()
                    ),
                    style(if i == game.selected { 11 } else { 7 }, 0),
                );
            }
            if let Some(recipe) = RECIPES.get(game.selected) {
                for (i, &(item, n)) in recipe.ingredients.iter().enumerate() {
                    canvas.put_str(
                        0,
                        size.height - 5 + i as u16,
                        &format!("{}: {}/{}", item.name(), game.inventory.count(item), n),
                        style(
                            if game.inventory.count(item) >= u16::from(n) {
                                10
                            } else {
                                9
                            },
                            0,
                        ),
                    );
                }
            }
        }
        State::Help => {
            canvas.put_str(
                0,
                0,
                "SURVIVAL GUIDE (time paused; Up/Down scroll)",
                style(14, 0),
            );
            for (i, line) in game
                .help_lines()
                .iter()
                .skip(game.selected)
                .take(size.height.saturating_sub(4) as usize)
                .enumerate()
            {
                canvas.put_str(0, 2 + i as u16, line, style(7, 0));
            }
        }
        State::Survey => {
            canvas.put_str(0, 0, "PLANET SURVEY (paused; M/Back closes)", style(14, 0));
            for (i, line) in game
                .survey_lines()
                .iter()
                .skip(game.selected)
                .take(size.height.saturating_sub(4) as usize)
                .enumerate()
            {
                canvas.put_str(0, 2 + i as u16, line, style(7, 0));
            }
        }
        State::Dead => {
            canvas.put_str(1, 2, "LIFE SUPPORT FAILED", style(9, 0));
            canvas.put_str(
                1,
                4,
                "N/Enter: new expedition | F9: load save",
                style(15, 0),
            );
            canvas.put_str(1, 6, "Q/Esc: save this state and quit", style(7, 0));
        }
        _ => {}
    }
    canvas.put_str(0, size.height - 2, &game.message, style(11, 0));
    canvas.put_str(
        0,
        size.height - 1,
        "Up/Down Select | Enter Confirm | Back Close | Q/Esc Quit",
        style(14, 0),
    );
}
