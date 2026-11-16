use input::Input;
use loader::*;
use mat::Vec3;
use renderer::Screen;
use std::env;
use std::fs;
use std::path::PathBuf;

mod camera;
mod enemies;
mod game;
mod input;
mod loader;
mod mat;
mod records;
mod renderer;
mod screens;

/// The level folder: the one named on the command line, else `levels` beside
/// the program (where the release archive puts it), else `levels` here.
fn level_dir() -> PathBuf {
    if let Some(dir) = env::args_os().nth(1) {
        return PathBuf::from(dir);
    }
    if let Some(dir) = env::current_exe()
        .ok()
        .and_then(|exe| fs::canonicalize(exe).ok())
        .and_then(|exe| exe.parent().map(|d| d.join("levels")))
        .filter(|d| d.is_dir())
    {
        return dir;
    }
    PathBuf::from("levels")
}

/// Puts the terminal back as it was.
pub fn restore_terminal() {
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::event::PopKeyboardEnhancementFlags,
        crossterm::cursor::Show
    );
    let _ = crossterm::terminal::disable_raw_mode();
    print!("\x1b[0m\x1b[2J\x1b[H");
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

fn main() {
    // session log: RUST_LOG=debug termtrack 2>termtrack.log, or
    // TERMTRACK_LOG=termtrack.log termtrack (debug unless RUST_LOG says
    // otherwise; trace adds a line for every frame)
    let log_file = env::var_os("TERMTRACK_LOG")
        .filter(|f| !f.is_empty())
        .and_then(|f| fs::OpenOptions::new().create(true).append(true).open(f).ok());
    let default_filter = if log_file.is_some() { "debug" } else { "off" };
    let mut logger = env_logger::Builder::new();
    logger
        .parse_filters(&env::var("RUST_LOG").unwrap_or_else(|_| default_filter.into()))
        .format_timestamp_millis();
    if let Some(file) = log_file {
        logger.target(env_logger::Target::Pipe(Box::new(file)));
        logger.write_style(env_logger::WriteStyle::Never);
    }
    logger.init();

    let dir = level_dir();
    let mut levels: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect(),
        Err(err) => {
            eprintln!("termtrack: cannot read the level folder {}: {err}", dir.display());
            eprintln!("usage: termtrack [level folder]");
            std::process::exit(1);
        }
    };
    levels.sort();
    if levels.is_empty() {
        eprintln!("termtrack: no levels in {}", dir.display());
        std::process::exit(1);
    }
    log::info!("{} levels from {}", levels.len(), dir.display());

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        log::error!("panic: {info}");
        default_hook(info);
    }));

    crossterm::terminal::enable_raw_mode().unwrap();
    let input = Input::start();

    loop {
        log::info!("state: menu");
        let chosen_level = screens::menu(levels.clone(), &input);
        let map = loader::load(&levels[chosen_level]);
        log::info!("state: level {} ({})", map.level_name, levels[chosen_level].display());

        loop {
            let mut game = game::Game {
                renderer: Screen::new(),
                camera: camera::Camera {
                    pos: Vec3 {
                        x: 0.,
                        y: 0.,
                        z: 0.,
                    },
                    focus_length: 1.5, //2
                    rotation: Vec3 {
                        x: 1.75,
                        y: 0.0,
                        z: 0.0,
                    },
                    vel: Vec3 {
                        x: 0.,
                        y: 0.,
                        z: 0.,
                    },
                },
            };
            input.clear_presses();
            let result = game.run(map.clone(), &input);
            log::info!("state: run ended: {:?}", result);
            match result {
                Ok(time) => {
                    if screens::finish(time, &map.level_name, &map.map_string, &input) == 1 {
                        continue;
                    }
                }
                Err(e) => match e {
                    "void" => {
                        if screens::game_over("You fell into the void!", &input) {
                            continue;
                        }
                    }
                    "angry_pixel" => {
                        if screens::game_over("Angry pixel killed you!", &input) {
                            continue;
                        }
                    }
                    "spike" => {
                        if screens::game_over("You died of spike!", &input) {
                            continue;
                        }
                    }
                    "menu" => (),
                    "retry" => continue,
                    _ => {
                        if screens::game_over("You failed!", &input) {
                            continue;
                        }
                    }
                },
            }
            break; // break loop if not try again
        }
    }
}
