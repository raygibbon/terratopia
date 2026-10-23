use crate::{
    camera::{Camera, usable, viewport},
    exploration::Visibility,
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
        Biome::Wasteland => (';', 13, 0),
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
                        let visibility = game.exploration.visibility(wx, wy);
                        if visibility == Visibility::Unknown {
                            map.put_char(x, y, '?', style(8, 0));
                            continue;
                        }
                        let tile = game.camp.tile(&mut game.world, wx, wy);
                        let (mut glyph, mut fg, mut bg) = appearance(tile.biome);
                        if let Some(site) = tile.landmark {
                            glyph = site.glyph();
                            fg = site.colour();
                            bg = 0;
                        }
                        if (wx, wy) == game.camp.ship() {
                            glyph = 'A';
                            fg = 15;
                            bg = 0;
                        }
                        if game.camp.sheltered((wx, wy)) {
                            glyph = 'H';
                            fg = 11;
                            bg = 0;
                        }
                        let tile_style = if visibility == Visibility::Remembered {
                            style(8, 0)
                        } else {
                            style(fg, bg)
                        };
                        map.put_char(x, y, glyph, tile_style);
                    }
                }
                for alive in [false, true] {
                    for c in &game.wildlife.active {
                        let p = c.state.position;
                        if (c.state.health > 0) != alive
                            || c.state.health == 0 && c.state.looted
                            || game.exploration.visibility(p.0, p.1) != Visibility::Visible
                        {
                            continue;
                        }
                        let sx = i128::from(p.0) - i128::from(camera.x);
                        let sy = i128::from(p.1) - i128::from(camera.y);
                        if sx >= 0
                            && sy >= 0
                            && sx < i128::from(view.width)
                            && sy < i128::from(view.height)
                        {
                            map.put_char(
                                sx as u16,
                                sy as u16,
                                if alive {
                                    c.species.behaviour.glyph()
                                } else {
                                    'x'
                                },
                                style(
                                    if alive {
                                        c.species.behaviour.colour()
                                    } else {
                                        8
                                    },
                                    0,
                                ),
                            );
                        }
                    }
                }
                let x = (i128::from(game.player.x) - i128::from(camera.x)) as u16;
                let y = (i128::from(game.player.y) - i128::from(camera.y)) as u16;
                map.put_char(x, y, '@', style(15, 1));
            }
            hud(&mut canvas, game);
            if let Some(report) = &game.last_scan
                && size.width >= 80
                && size.height >= 18
            {
                let width = 29;
                let px = (i128::from(game.player.x) - i128::from(camera.x)) as u16;
                let panel_x = if px >= size.width - width {
                    0
                } else {
                    size.width - width
                };
                let mut panel = canvas.clip(Rect::new(panel_x, 3, width, 10));
                panel.fill(style(7, 0));
                panel.put_str(0, 0, "LAST SCAN / R rescan", style(14, 0));
                panel.put_str(
                    0,
                    1,
                    &format!("+{} cells / +{} sites", report.new_cells, report.new_sites),
                    style(7, 0),
                );
                panel.put_str(0, 2, "i Iron k Carbon v Crystal", style(15, 0));
                panel.put_str(0, 3, "g/d/p Wildlife | M survey", style(11, 0));
                let wildlife: Vec<_> = game
                    .scan_contacts
                    .iter()
                    .filter(|s| {
                        s.contains("(grazer)")
                            || s.contains("(defender)")
                            || s.contains("(predator)")
                    })
                    .collect();
                if wildlife.is_empty() {
                    for (row, site) in report.sites.iter().take(3).enumerate() {
                        panel.put_str(
                            0,
                            4 + row as u16 * 2,
                            &format!("{} {}", site.kind.glyph(), site.kind.name()),
                            style(site.kind.colour(), 0),
                        );
                        panel.put_str(
                            0,
                            5 + row as u16 * 2,
                            &format!("  {},{}", site.x, site.y),
                            style(7, 0),
                        );
                    }
                } else {
                    for (row, contact) in wildlife.iter().take(3).enumerate() {
                        panel.put_str(0, 4 + row as u16 * 2, contact, style(7, 0));
                    }
                    panel.put_str(0, 9, "M: species / flora / anomalies", style(14, 0));
                }
            }
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
                canvas.put_str(3, 7, "[L / F9] Load   [Q / Esc] Quit", style(7, 0));
            } else {
                canvas.put_str(3, 6, &format!("Planet {}", game.planet), style(11, 0));
                canvas.put_str(3, 7, &format!("Seed {}", game.world.seed()), style(7, 0));
                canvas.put_str(3, 9, "[Enter] Land  [M] Menu  [Q] Quit", style(15, 0));
            }
            canvas.put_str(
                3,
                size.height - 2,
                "Living alien worlds / Phase 3",
                style(8, 0),
            );
        }
        State::Inventory | State::Crafting | State::Help | State::Survey | State::Dead => {
            crate::ui::screen(&mut canvas, game)
        }
        State::Quit => {}
    }
}
