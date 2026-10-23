use std::{
    env, io,
    path::PathBuf,
    process::ExitCode,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use terrabox::{Event, FrameClock};
use terragame_terminal::Session;
use terratopia::{
    game::{Game, STEP, State},
    input::{self, Action},
    persistence, renderer,
};
struct Options {
    seed: u64,
    save: PathBuf,
    load: bool,
    help: bool,
}
fn options() -> Result<Options, String> {
    let mut args = env::args().skip(1);
    let mut seed = None;
    let mut save = None;
    let mut load = false;
    let mut help = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => help = true,
            "--seed" if seed.is_none() => {
                seed = Some(
                    args.next()
                        .ok_or("--seed requires a u64")?
                        .parse::<u64>()
                        .map_err(|_| "seed must be an unsigned 64-bit integer")?,
                )
            }
            "--save" if save.is_none() => {
                save = Some(PathBuf::from(args.next().ok_or("--save requires a path")?))
            }
            "--load" if !load => load = true,
            _ => return Err(format!("unknown or repeated argument: {arg}")),
        }
    }
    let seed = seed.unwrap_or_else(|| {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        terratopia::world::seed::mix(time.as_secs() ^ u64::from(time.subsec_nanos()))
    });
    Ok(Options {
        seed,
        save: save.unwrap_or_else(|| PathBuf::from("terratopia.save")),
        load,
        help,
    })
}
fn run(options: Options) -> io::Result<()> {
    let mut game = if options.load {
        persistence::load(&options.save)?
    } else {
        Game::new(options.seed)
    };
    let mut terminal = Session::new()?;
    let mut clock = FrameClock::new(STEP, Instant::now()).expect("valid timestep");
    clock.pause();
    let mut redraw = true;
    while game.state != State::Quit {
        if let Some(timing) = clock.poll(Instant::now()) {
            redraw |= game.advance(timing.elapsed);
        }
        // Simulation can enter Dead without an input event.
        if game.state != State::Playing && !clock.paused() {
            clock.pause();
        }
        if redraw {
            let size = terminal.size();
            renderer::render(&mut game, &mut terminal.surface, size);
            terminal.present()?;
            redraw = false;
        }
        match terminal.wait_event(clock.deadline().map(|d| d.remaining(Instant::now())))? {
            Some(Event::Key(key)) => {
                if let Some(action) = input::action(key) {
                    let before = game.state;
                    match action {
                        Action::Save => {
                            match persistence::save(&game, &options.save) {
                                Ok(()) => game.say("Expedition saved."),
                                Err(e) => game.say(format!("Save failed: {e}")),
                            };
                            redraw = true;
                        }
                        Action::Load => {
                            match persistence::load(&options.save) {
                                Ok(loaded) => game = loaded,
                                Err(e) => game.say(format!("Load failed: {e}")),
                            };
                            redraw = true;
                        }
                        Action::Quit if game.started => {
                            match persistence::save(&game, &options.save) {
                                Ok(()) => {
                                    game.handle(Action::Quit);
                                }
                                Err(e) => {
                                    game.say(format!(
                                        "Save failed: Ctrl+Q quits unsaved; F5 retries. {e}"
                                    ));
                                    redraw = true;
                                }
                            }
                        }
                        Action::QuitWithoutSave => {
                            game.handle(Action::Quit);
                        }
                        _ => redraw |= game.handle(action),
                    }
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
    match options() {
        Ok(options) if options.help => {
            println!(
                "TerraTopia Phase 3\nUsage: terratopia [--seed <u64>] [--save <path>] [--load]\nE interact; F fight; Z rest; M survey; I inventory; C craft; B build; O oxygen; H help; R scan\nP pause; F5 save; F9/L load; Q/Esc save and quit; Ctrl+Q quit without saving"
            );
            ExitCode::SUCCESS
        }
        Ok(options) => match run(options) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("TerraTopia: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("TerraTopia: {e}\nUsage: terratopia [--seed <u64>] [--save <path>] [--load]");
            ExitCode::FAILURE
        }
    }
}
