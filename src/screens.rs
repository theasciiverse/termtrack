use std::{ffi::OsStr, fs, path::PathBuf};

use crate::input::{Input, Key};
use crate::{records, screens};

use crate::renderer;

const TITLE_L: &str = r#"                  ___           ___           ___                       ___           ___           ___           ___     
      ___        /  /\         /  /\         /__/\          ___        /  /\         /  /\         /  /\         /__/|    
     /  /\      /  /:/_       /  /::\       |  |::\        /  /\      /  /::\       /  /::\       /  /:/        |  |:|    
    /  /:/     /  /:/ /\     /  /:/\:\      |  |:|:\      /  /:/     /  /:/\:\     /  /:/\:\     /  /:/         |  |:|    
   /  /:/     /  /:/ /:/_   /  /:/~/:/    __|__|:|\:\    /  /:/     /  /:/~/:/    /  /:/~/::\   /  /:/  ___   __|  |:|    
  /  /::\    /__/:/ /:/ /\ /__/:/ /:/___ /__/::::| \:\  /  /::\    /__/:/ /:/___ /__/:/ /:/\:\ /__/:/  /  /\ /__/\_|:|____
 /__/:/\:\   \  \:\/:/ /:/ \  \:\/:::::/ \  \:\~~\__\/ /__/:/\:\   \  \:\/:::::/ \  \:\/:/__\/ \  \:\ /  /:/ \  \:\/:::::/
 \__\/  \:\   \  \::/ /:/   \  \::/~~~~   \  \:\       \__\/  \:\   \  \::/~~~~   \  \::/       \  \:\  /:/   \  \::/~~~~ 
      \  \:\   \  \:\/:/     \  \:\        \  \:\           \  \:\   \  \:\        \  \:\        \  \:\/:/     \  \:\     
       \__\/    \  \::/       \  \:\        \  \:\           \__\/    \  \:\        \  \:\        \  \::/       \  \:\    
                 \__\/         \__\/         \__\/                     \__\/         \__\/         \__\/         \__\/    "#; // min 140

const TITLE_S: &str = r#" _____ ______________  ______________  ___  _____  _   __
|_   _|  ___| ___ \  \/  |_   _| ___ \/ _ \/  __ \| | / /
  | | | |__ | |_/ / .  . | | | | |_/ / /_\ \ /  \/| |/ / 
  | | |  __||    /| |\/| | | | |    /|  _  | |    |    \ 
  | | | |___| |\ \| |  | | | | | |\ \| | | | \__/\| |\  \
  \_/ \____/\_| \_\_|  |_/ \_/ \_| \_\_| |_/\____/\_| \_/"#; // min 100

const TITLE_XS: &str = r#"___ ____ ____ _  _ ___ ____ ____ ____ _  _ 
 |  |___ |__/ |\/|  |  |__/ |__| |    |_/  
 |  |___ |  \ |  |  |  |  \ |  | |___ | \_ "#; // min 70

const TITLE_M: &str = r#" ________  ________  _______   __       __  ________  _______    ______    ______   __    __ 
/        |/        |/       \ /  \     /  |/        |/       \  /      \  /      \ /  |  /  |
$$$$$$$$/ $$$$$$$$/ $$$$$$$  |$$  \   /$$ |$$$$$$$$/ $$$$$$$  |/$$$$$$  |/$$$$$$  |$$ | /$$/ 
   $$ |   $$ |__    $$ |__$$ |$$$  \ /$$$ |   $$ |   $$ |__$$ |$$ |__$$ |$$ |  $$/ $$ |/$$/  
   $$ |   $$    |   $$    $$< $$$$  /$$$$ |   $$ |   $$    $$< $$    $$ |$$ |      $$  $$<   
   $$ |   $$$$$/    $$$$$$$  |$$ $$ $$/$$ |   $$ |   $$$$$$$  |$$$$$$$$ |$$ |   __ $$$$$  \  
   $$ |   $$ |_____ $$ |  $$ |$$ |$$$/ $$ |   $$ |   $$ |  $$ |$$ |  $$ |$$ \__/  |$$ |$$  \ 
   $$ |   $$       |$$ |  $$ |$$ | $/  $$ |   $$ |   $$ |  $$ |$$ |  $$ |$$    $$/ $$ | $$  |
   $$/    $$$$$$$$/ $$/   $$/ $$/      $$/    $$/    $$/   $$/ $$/   $$/  $$$$$$/  $$/   $$/ "#; // min 130

pub fn menu_print() {
    let (screen_width, screen_height) = renderer::get_terminal_size();
    let screen_width = screen_width as u16;
    let screen_height = screen_height as u16;
    let message = vec![
        "Use |↑| and |↓| to navigate menu.",
        "Press enter to play.\n",
        " ",
        "How to win:",
        "Find the end.",
        " ",
        "|L| - your best times",
    ];
    // print background image
    print!("{esc}[H{esc}[48;2;0;0;0m", esc = 27 as char);
    for _row in 0..=screen_height {
        println!("{}\r", " ".repeat(screen_width as usize),)
    }

    let (title, gap) = match screen_width {
        125..=u16::MAX => (TITLE_L, 2),
        95..125 => (TITLE_M, 2),
        59..95 => (TITLE_S, 1),
        45..59 => (TITLE_XS, 1),
        0..45 => ("", 0),
    };

    if title.is_empty() {
        panic!("please expand your terminal and try again.");
    };

    // print title
    let lines: Vec<&str> = title.split("\n").collect();
    let menu_width = lines[1].len() as u16;
    let mut y: u16 = 0;
    let x = screen_width / 2 - menu_width / 2;
    y += gap;
    for line in &lines {
        println!(
            "{esc}[{};{}H{}",
            y,
            screen_width / 2 - (line.len() as u16) / 2,
            line,
            esc = 27 as char
        );
        y += 1;
    }

    // print controls

    y += gap;
    println!(
        "{esc}[{};{}H{:-^3$}",
        y,
        screen_width / 2 - menu_width / 2,
        "",
        menu_width as usize,
        esc = 27 as char
    );
    y += 1;

    if title == TITLE_L {
        println!(
        "{esc}[{};{}H{:^3$}",
        y,
        screen_width / 2 - menu_width / 2,
        "|W| |A| |S| |D| - move   |←| |↑| |↓| |→| - rotate camera   | [SPACEBAR] | - jump   |M| - view map   |E| - exit",
        menu_width as usize,
        esc = 27 as char
    );
    } else if title == TITLE_M {
        println!(
        "{esc}[{};{}H{:^3$}",
        y,
        screen_width / 2 - menu_width / 2,
        "|W| |A| |S| |D| - move   |←| |↑| |↓| |→| - rotate camera   | [SPACEBAR] | - jump",
        menu_width as usize,
        esc = 27 as char
    );
        y += 1;
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "|M| - view map   |E| - exit",
            menu_width as usize,
            esc = 27 as char
        );
    } else if title == TITLE_S {
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "WASD - move | ← ↑ ↓ →  - rotate camera | [SPACE] - jump",
            menu_width as usize,
            esc = 27 as char
        );
        y += 1;
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "M - map | E - exit",
            menu_width as usize,
            esc = 27 as char
        );
    } else if title == TITLE_XS {
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "WASD - move | ← ↑ ↓ →  - rotate camera",
            menu_width as usize,
            esc = 27 as char
        );
        y += 1;
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "[SPACE] - jump | M - map | E - exit",
            menu_width as usize,
            esc = 27 as char
        );
    }

    y += 1;
    println!(
        "{esc}[{};{}H{:-^3$}",
        y,
        screen_width / 2 - menu_width / 2,
        "",
        menu_width as usize,
        esc = 27 as char
    );

    // print instructions
    y += gap;
    let mut chopped_message = vec![];
    for line in message {
        let mut chopped_line = line
            .chars()
            .collect::<Vec<char>>()
            .chunks(menu_width as usize / 2 - gap as usize)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<String>>();

        chopped_message.append(&mut chopped_line);
    }

    for (n, line) in chopped_message.iter().enumerate() {
        println!(
            "{esc}[{};{}H{}",
            y + n as u16,
            screen_width / 2 + gap,
            line,
            esc = 27 as char
        );
    }

    let box_width: u16 = menu_width / 2 - 2;
}

pub fn menu(levels: Vec<PathBuf>, input: &Input) -> usize {
    let mut chosen_level = 0;
    let level_names: Vec<&OsStr> = levels
        .iter()
        .map(|path| path.file_stem().unwrap())
        .collect();
    let (screen_width, screen_height) = renderer::get_terminal_size();
    let screen_width = screen_width as u16;
    let screen_height = screen_height as u16;
    let mut box_height = 7;
    if levels.len() < 7 {
        box_height = level_names.len() as u16;
    }
    let message = vec![
        "Use |↑| and |↓| to navigate menu.",
        "Press enter to play.\n",
        " ",
        "How to win:",
        "Find the end.",
        " ",
        "|L| - your best times",
    ];

    // print background image
    print!("{esc}[H{esc}[48;2;0;0;0m", esc = 27 as char);
    for _row in 0..=screen_height {
        println!("{}\r", " ".repeat(screen_width as usize),)
    }

    // print title

    let (title, gap) = match screen_width {
        125..=u16::MAX => (TITLE_L, 2),
        95..125 => (TITLE_M, 2),
        59..95 => (TITLE_S, 1),
        45..59 => (TITLE_XS, 1),
        0..45 => ("", 0),
    };

    if title.is_empty() {
        panic!("please expand your terminal and try again.");
    };

    let lines: Vec<&str> = title.split("\n").collect();
    let menu_width = lines[1].len() as u16;
    let mut y: u16 = 0;
    let x = screen_width / 2 - menu_width / 2;
    y += gap;
    for line in &lines {
        println!(
            "{esc}[{};{}H{}",
            y,
            screen_width / 2 - (line.len() as u16) / 2,
            line,
            esc = 27 as char
        );
        y += 1;
    }

    // print controls

    y += gap;
    println!(
        "{esc}[{};{}H{:-^3$}",
        y,
        screen_width / 2 - menu_width / 2,
        "",
        menu_width as usize,
        esc = 27 as char
    );
    y += 1;

    if title == TITLE_L {
        println!(
        "{esc}[{};{}H{:^3$}",
        y,
        screen_width / 2 - menu_width / 2,
        "|W| |A| |S| |D| - move   |←| |↑| |↓| |→| - rotate camera   | [SPACEBAR] | - jump   |M| - view map   |E| - exit",
        menu_width as usize,
        esc = 27 as char
    );
    } else if title == TITLE_M {
        println!(
        "{esc}[{};{}H{:^3$}",
        y,
        screen_width / 2 - menu_width / 2,
        "|W| |A| |S| |D| - move   |←| |↑| |↓| |→| - rotate camera   | [SPACEBAR] | - jump",
        menu_width as usize,
        esc = 27 as char
    );
        y += 1;
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "|M| - view map   |E| - exit",
            menu_width as usize,
            esc = 27 as char
        );
    } else if title == TITLE_S {
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "WASD - move | ← ↑ ↓ →  - rotate camera | [SPACE] - jump",
            menu_width as usize,
            esc = 27 as char
        );
        y += 1;
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "M - map | E - exit",
            menu_width as usize,
            esc = 27 as char
        );
    } else if title == TITLE_XS {
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "WASD - move | ← ↑ ↓ →  - rotate camera",
            menu_width as usize,
            esc = 27 as char
        );
        y += 1;
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            screen_width / 2 - menu_width / 2,
            "[SPACE] - jump | M - map | E - exit",
            menu_width as usize,
            esc = 27 as char
        );
    }

    y += 1;
    println!(
        "{esc}[{};{}H{:-^3$}",
        y,
        screen_width / 2 - menu_width / 2,
        "",
        menu_width as usize,
        esc = 27 as char
    );

    // print instructions
    y += gap;
    let mut chopped_message = vec![];
    for line in message {
        let mut chopped_line = line
            .chars()
            .collect::<Vec<char>>()
            .chunks(menu_width as usize / 2 - gap as usize)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<String>>();

        chopped_message.append(&mut chopped_line);
    }

    for (n, line) in chopped_message.iter().enumerate() {
        println!(
            "{esc}[{};{}H{}",
            y + n as u16,
            screen_width / 2 + gap,
            line,
            esc = 27 as char
        );
    }

    let box_width: u16 = menu_width / 2 - 2;

    loop {
        // PRINT BOX
        // box upper line
        println!(
            "{esc}[{};{}H*{:-^3$}*",
            y,
            x,
            "",
            (box_width - 2) as usize,
            esc = 27 as char
        );

        // box content
        let mut lowest = 0;
        let mut highest = 0;
        if chosen_level < box_height.div_ceil(2) {
            highest = box_height;
        } else if level_names.len() as u16 - 1 - chosen_level < box_height.div_euclid(2) {
            highest = level_names.len() as u16;
            lowest = highest - box_height;
        } else {
            highest = chosen_level + box_height.div_euclid(2) + 1;
            lowest = chosen_level - box_height.div_euclid(2);
        }

        for i in lowest..highest {
            if i == chosen_level {
                print!("{esc}[48;2;46;46;46m", esc = 27 as char);
            }

            let mut level_name = level_names[i as usize].to_str().unwrap().to_string();
            if level_name.len() > box_width as usize - 2 {
                level_name = level_name[0..(box_width as usize - 5)].to_string() + "...";
            }
            println!(
                "{esc}[{};{}H|{:^3$}|",
                y + 1 + (i - lowest),
                x,
                level_name,
                (box_width - 2) as usize,
                esc = 27 as char
            );
            print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        }

        // box lower line
        println!(
            "{esc}[{};{}H*{:-^3$}*",
            y + box_height + 1,
            x,
            "",
            (box_width - 2) as usize,
            esc = 27 as char
        );

        //chosen level id
        let level_map = fs::read(&levels[chosen_level as usize]).unwrap();

        let level_id = records::level_id(level_names[chosen_level as usize].to_str().unwrap(), &level_map);

        // println!(
        //     "{esc}[{};{}Hid: {}",
        //     y + box_height + 2,
        //     x,
        //     level_id,
        //     esc = 27 as char
        // );

        //match input
        loop {
            match input.wait_press() {
                Key::Down if chosen_level != level_names.len() as u16 - 1 => {
                    chosen_level += 1;
                    break;
                }
                Key::Up if chosen_level != 0 => {
                    chosen_level = chosen_level.saturating_sub(1);
                    break;
                }
                Key::Enter | Key::Space => {
                    return chosen_level as usize;
                }
                Key::L => {
                    log::info!("state: best times");
                    best_times(
                        level_names[chosen_level as usize].to_str().unwrap().to_string(),
                        &level_id,
                        input,
                    );
                    menu_print();
                    break;
                }
                Key::E | Key::Escape => {
                    if screens::exit(input) {
                        exit_app();
                    };
                    menu_print();
                    break;
                }
                Key::Quit => exit_app(),
                _ => (),
            }
        }
    }
}

/// Your best times on a level, kept on this computer.
pub fn best_times(level_name: String, level_id: &str, input: &Input) {
    let (screen_width, screen_height) = renderer::get_terminal_size();
    let screen_width = screen_width as u16;
    let screen_height = screen_height as u16;
    let box_width = 50;
    let x = screen_width / 2 - box_width / 2;
    let margin = 3;
    let mut scroll: usize = 0;

    let times = records::times(level_id);
    let take = times.len().min((screen_height as usize).saturating_sub(10)).max(1);

    let level_name = level_name.to_uppercase();

    loop {
        // print background image
        print!("{esc}[H{esc}[48;2;0;0;0m", esc = 27 as char);
        for _row in 0..=screen_height {
            println!("{}\r", " ".repeat(screen_width as usize),)
        }
        let mut y = 3;
        println!(
            "{esc}[{};{}H{:^3$}",
            y,
            x,
            format!("* YOUR BEST TIMES: {level_name} *"),
            (box_width) as usize,
            esc = 27 as char
        );
        y += 1;
        println!("{esc}[{};{}H{:-^3$}", y, x, "", (box_width) as usize, esc = 27 as char);
        y += 1;

        if times.is_empty() {
            println!(
                "{esc}[{};{}H{}",
                y,
                x + margin,
                "No finished runs yet. Find the end!",
                esc = 27 as char
            );
        }
        for (i, time) in times.iter().skip(scroll).take(take).enumerate() {
            println!(
                "{esc}[{};{}H{:<3$}",
                y + i as u16,
                x + margin,
                format!("{}. {:.2}s", i + 1 + scroll, time),
                (box_width - 4) as usize,
                esc = 27 as char
            );
        }

        println!("{esc}[{};{}H{:-^3$}", y + take as u16, x, "", (box_width) as usize, esc = 27 as char);
        println!(
            "{esc}[{};{}H{}",
            y + take as u16 + 1,
            x,
            "Use |↑| and |↓| to scroll",
            esc = 27 as char
        );
        println!(
            "{esc}[{};{}H{}",
            y + take as u16 + 2,
            x,
            "Press |E| to go back",
            esc = 27 as char
        );

        loop {
            match input.wait_press() {
                Key::Up => {
                    scroll = scroll.saturating_sub(1);
                    break;
                }
                Key::Down => {
                    if scroll + take < times.len() {
                        scroll += 1;
                    }
                    break;
                }
                Key::E | Key::Escape | Key::Enter | Key::L => return,
                Key::Quit => exit_app(),
                _ => (),
            }
        }
    }
}

pub fn game_over(arg: &str, input: &Input) -> bool {
    let (screen_width, screen_height) = renderer::get_terminal_size();
    let screen_width = screen_width as u16;
    let screen_height = screen_height as u16;
    let box_width: u16 = 30.max(arg.len() as u16 + 2);
    let box_height = 5;

    let start_x = screen_width / 2 - box_width / 2;
    let start_y = screen_height / 2 - box_height / 2;
    let mut try_again = true;

    loop {
        // print background image
        print!("{esc}[H{esc}[48;2;0;0;0m", esc = 27 as char);
        for _row in 0..=screen_height {
            println!("{}\r", " ".repeat(screen_width as usize),)
        }

        // print menu
        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        println!(
            "{esc}[{};{}H*{:-^3$}*",
            start_y,
            start_x,
            "",
            (box_width - 2) as usize,
            esc = 27 as char
        );
        println!(
            "{esc}[{};{}H|{: ^3$}|",
            start_y + 1,
            start_x,
            arg,
            (box_width - 2) as usize,
            esc = 27 as char
        );
        println!(
            "{esc}[{};{}H|{: ^3$}|",
            start_y + 2,
            start_x,
            "try again?",
            (box_width - 2) as usize,
            esc = 27 as char
        );
        if try_again {
            print!("{esc}[48;2;46;46;46m", esc = 27 as char);
        }
        println!(
            "{esc}[{};{}H|{: ^3$}|",
            start_y + 3,
            start_x,
            "YES",
            (box_width - 2) as usize,
            esc = 27 as char
        );
        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        if !try_again {
            print!("{esc}[48;2;46;46;46m", esc = 27 as char);
        }
        println!(
            "{esc}[{};{}H|{: ^3$}|",
            start_y + 4,
            start_x,
            "NO",
            (box_width - 2) as usize,
            esc = 27 as char
        );

        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        println!(
            "{esc}[{};{}H*{:-^3$}*",
            start_y + 5,
            start_x,
            "",
            (box_width - 2) as usize,
            esc = 27 as char
        );

        //match input
        loop {
            match input.wait_press() {
                Key::Down | Key::Up => {
                    try_again = !try_again;
                    break;
                }
                Key::Enter | Key::Space => return try_again,
                Key::R => return true,
                Key::E | Key::Escape => {
                    if exit(input) {
                        exit_app();
                    }
                    break;
                }
                Key::Quit => exit_app(),
                _ => (),
            }
        }
    }
}

/// The end reached: the time, kept with your best times, and try again?
/// Returns 1 to try again, 2 for the menu.
pub fn finish(time: f64, level_name: &str, level_map: &str, input: &Input) -> u8 {
    let (screen_width, screen_height) = renderer::get_terminal_size();
    let screen_width = screen_width as u16;
    let screen_height = screen_height as u16;

    let id = records::level_id(level_name, level_map.as_bytes());
    let before = records::times(&id);
    records::add_time(&id, time);
    let best = records::times(&id);
    let new_best = before.first().map_or(true, |b| time < *b);

    let take = best.len().min(5);

    // set the size of the textbox
    let box_height = 8 + take as u16;
    let box_width = 35;

    let start_x = screen_width / 2 - box_width / 2;
    let start_y = (screen_height / 2).saturating_sub(box_height / 2).max(1);

    let mut chosen = 1;

    loop {
        // print background image
        print!("{esc}[H{esc}[48;2;0;0;0m", esc = 27 as char);
        for _row in 0..=screen_height {
            println!("{}\r", " ".repeat(screen_width as usize),)
        }

        let line = |y: u16, text: &str| {
            println!(
                "{esc}[{};{}H|{:^3$}|",
                y,
                start_x,
                text,
                (box_width - 2) as usize,
                esc = 27 as char
            );
        };

        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        println!("{esc}[{};{}H*{:-^3$}*", start_y, start_x, "", (box_width - 2) as usize, esc = 27 as char);
        line(start_y + 1, "You won!");
        line(start_y + 2, &format!("Time: {:.2}s", time));
        line(start_y + 3, if new_best { "A new best time!" } else { "" });
        line(start_y + 4, "---YOUR BEST---");
        for (i, t) in best.iter().take(take).enumerate() {
            line(start_y + 5 + i as u16, &format!("{}. {:.2}s", i + 1, t));
        }
        line(start_y + 5 + take as u16, "-------------");
        line(start_y + box_height - 2, "Try again?");
        if chosen == 1 {
            print!("{esc}[48;2;46;46;46m", esc = 27 as char);
        }
        line(start_y + box_height - 1, "YES");
        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        if chosen == 2 {
            print!("{esc}[48;2;46;46;46m", esc = 27 as char);
        }
        line(start_y + box_height, "NO");
        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        println!(
            "{esc}[{};{}H*{:-^3$}*",
            start_y + box_height + 1,
            start_x,
            "",
            (box_width - 2) as usize,
            esc = 27 as char
        );

        //match input
        loop {
            match input.wait_press() {
                Key::Down if chosen != 2 => {
                    chosen += 1;
                    break;
                }
                Key::Up if chosen != 1 => {
                    chosen -= 1;
                    break;
                }
                Key::Enter | Key::Space => return chosen,
                Key::R => return 1,
                Key::E | Key::Escape => {
                    if exit(input) {
                        exit_app();
                    }
                    break;
                }
                Key::Quit => exit_app(),
                _ => (),
            }
        }
    }
}

pub fn exit(input: &Input) -> bool {
    let (screen_width, screen_height) = renderer::get_terminal_size();
    let screen_width = screen_width as u16;
    let screen_height = screen_height as u16;

    let box_width: u16 = 30;
    let box_height = 5;

    let start_x = screen_width / 2 - box_width / 2;
    let start_y = screen_height / 2 - box_height / 2;
    let mut exit = true;

    loop {
        // print background image
        print!("{esc}[H{esc}[48;2;0;0;0m", esc = 27 as char);
        for _row in 0..=screen_height {
            println!("{}\r", " ".repeat(screen_width as usize),)
        }

        // print menu
        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        println!(
            "{esc}[{};{}H*{:-^3$}*",
            start_y,
            start_x,
            "",
            (box_width - 2) as usize,
            esc = 27 as char
        );
        println!(
            "{esc}[{};{}H|{: ^3$}|",
            start_y + 1,
            start_x,
            "Are you sure you",
            (box_width - 2) as usize,
            esc = 27 as char
        );
        println!(
            "{esc}[{};{}H|{: ^3$}|",
            start_y + 2,
            start_x,
            "want to exit?",
            (box_width - 2) as usize,
            esc = 27 as char
        );
        if exit {
            print!("{esc}[48;2;46;46;46m", esc = 27 as char);
        }
        println!(
            "{esc}[{};{}H|{: ^3$}|",
            start_y + 3,
            start_x,
            "YES",
            (box_width - 2) as usize,
            esc = 27 as char
        );
        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        if !exit {
            print!("{esc}[48;2;46;46;46m", esc = 27 as char);
        }
        println!(
            "{esc}[{};{}H|{: ^3$}|",
            start_y + 4,
            start_x,
            "NO",
            (box_width - 2) as usize,
            esc = 27 as char
        );

        print!("{esc}[48;2;0;0;0m", esc = 27 as char);
        println!(
            "{esc}[{};{}H*{:-^3$}*",
            start_y + 5,
            start_x,
            "",
            (box_width - 2) as usize,
            esc = 27 as char
        );

        //match input
        loop {
            match input.wait_press() {
                Key::Down | Key::Up => {
                    exit = !exit;
                    break;
                }
                Key::Enter | Key::Space => return exit,
                Key::Escape | Key::E => return false,
                Key::Quit => exit_app(),
                _ => (),
            }
        }
    }
}

pub fn exit_app() -> ! {
    log::info!("state: quit");
    crate::restore_terminal();
    println!("Thanks for playing TermTrack!\r");
    std::process::exit(0);
}
