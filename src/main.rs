use std::{
    env, io,
    process::ExitCode,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use terrabox::{Event, FrameClock, Surface, Terminal};
use terratopia::{
    game::{Game, STEP, State},
    input, renderer,
};

fn seed_from_args() -> Result<Option<u64>, String> {
    let mut args = env::args().skip(1);
    let mut seed = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--seed" if seed.is_none() => {
                seed = Some(
                    args.next()
                        .ok_or("--seed requires an unsigned 64-bit integer")?
                        .parse::<u64>()
                        .map_err(|_| "seed must be an unsigned 64-bit integer")?,
                );
            }
            _ => return Err(format!("unknown or repeated argument: {arg}")),
        }
    }
    Ok(Some(seed.unwrap_or_else(|| {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        terratopia::world::seed::mix(time.as_secs() ^ u64::from(time.subsec_nanos()))
    })))
}
fn run(seed: u64) -> io::Result<()> {
    let mut game = Game::new(seed);
    let mut terminal = Terminal::new()?;
    terminal.hide_cursor();
    let mut surface = Surface::new(terminal.size());
    let mut clock = FrameClock::new(STEP, Instant::now()).expect("valid fixed timestep");
    clock.pause();
    let mut redraw = true;
    while game.state != State::Quit {
        if let Some(timing) = clock.poll(Instant::now()) {
            redraw |= game.advance(timing.elapsed);
        }
        if redraw {
            renderer::render(&mut game, &mut surface, terminal.size());
            terminal.draw_surface(&surface, Default::default());
            terminal.present()?;
            redraw = false;
        }
        let event = terminal.wait_event(clock.deadline().map(|d| d.remaining(Instant::now())))?;
        match event {
            Some(Event::Key(key)) => {
                if let Some(action) = input::action(key) {
                    let before = game.state;
                    redraw |= game.handle(action);
                    if before != game.state {
                        if game.state == State::Playing {
                            clock.resume(Instant::now());
                        } else {
                            clock.pause();
                        }
                    }
                }
            }
            Some(Event::Resize(_)) => redraw = true,
            _ => {}
        }
    }
    Ok(())
}
fn main() -> ExitCode {
    match seed_from_args() {
        Ok(None) => {
            println!(
                "TerraTopia\nUsage: terratopia [--seed <u64>]\nEnter/N: new world; Enter: land; WASD/arrows: move; R: scan; P: pause; Q/Esc: quit"
            );
            ExitCode::SUCCESS
        }
        Ok(Some(seed)) => match run(seed) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("TerraTopia: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("TerraTopia: {error}\nUsage: terratopia [--seed <u64>]");
            ExitCode::FAILURE
        }
    }
}
