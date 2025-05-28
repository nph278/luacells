#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::needless_range_loop
)]

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton,
    MouseEvent, MouseEventKind, read,
};
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode};

use grid_area::{Neighborhood, Topology, neighborhood};
use mlua::prelude::*;
use rand::prelude::*;
use std::sync::mpsc;

const CONTROLS: &str =
    "[q] Exit [hjkl/wasd/arrows] Move [+/-] Change speed [r] Randomize [c] Clear [s] Save pattern";
const CONTROLS2: &str =
    "[space] Play/Pause [tab] Step [leftclick] Draw [rightclick] Erase [scroll/1-9] Change state";
const USAGE: &str = "
USAGE: luacells [rule.lua]
  --pattern -p        Load pattern file
  --delay -d          Delay in millis
  --size -s           Size of grid
  --save -sp          Pattern file to save to after exit
  --help -h           Display this message
";

// Clean up and return an error
macro_rules! die {
    ($($s:expr), +) => {{
        execute!(std::io::stdout(), Show).unwrap();
        execute!(std::io::stdout(), DisableMouseCapture).unwrap();
        execute!(std::io::stdout(), Clear(ClearType::All)).unwrap();
        execute!(std::io::stdout(), MoveTo(0, 0)).unwrap();
        disable_raw_mode().unwrap();
        eprintln!($($s), +);
        std::process::exit(1);
    }};
}

// Make a Display() output exactly 2 chars long
fn normalize_cell(s: &str) -> String {
    if s.len() > 2 {
        return s[0..3].to_string();
    }
    let mut s = s.to_string();
    while s.len() < 2 {
        s.push(' ');
    }
    s
}

// Input message
#[derive(Debug, Clone, Copy)]
enum Message {
    ShiftRow(i16),
    ShiftCol(i16),
    ShiftDelay(i16),
    CycleState(i16),
    SetState(u16),
    /// Col, row
    Draw(u16, u16),
    /// Col, row
    Erase(u16, u16),
    Step,
    Render,
    ScreenClear,
    GridClear,
    PlayPause,
    Randomize,
    Exit,
}

fn serialize_pattern(v: &[Vec<u16>]) -> String {
    v.iter()
        .map(|x| {
            x.iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<String>>()
                .join(",")
        })
        .collect::<Vec<String>>()
        .join(";")
}

fn deserialize_pattern(s: &str) -> Vec<Vec<u16>> {
    s.split(';')
        .map(|x| {
            x.split(',')
                .map(|x| {
                    println!("- {}", x);
                    x.trim()
                        .parse()
                        .unwrap_or_else(|_| die!("Malformed pattern"))
                })
                .collect()
        })
        .collect()
}

// SIGTERM handler
fn term() {
    execute!(std::io::stdout(), Show).unwrap();
    execute!(std::io::stdout(), Show).unwrap();
    execute!(std::io::stdout(), DisableMouseCapture).unwrap();
    disable_raw_mode().unwrap();
    println!();
    std::process::exit(130);
}

// Sends Message to main thread for inputs.
// Handles sending Render after things that cannot be incrementally rendered
fn handle_input(send: &mpsc::Sender<Message>) {
    match read().unwrap() {
        Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            term();
        }
        Event::Key(KeyEvent { code, .. }) => match code {
            KeyCode::Char('q') => send.send(Message::Exit).unwrap(),
            KeyCode::Char('a' | 'h') | KeyCode::Left => {
                send.send(Message::ShiftCol(-3)).unwrap();
                send.send(Message::Render).unwrap();
            }
            KeyCode::Char('d' | 'l') | KeyCode::Right => {
                send.send(Message::ShiftCol(3)).unwrap();
                send.send(Message::Render).unwrap();
            }
            KeyCode::Char('w' | 'k') | KeyCode::Up => {
                send.send(Message::ShiftRow(-3)).unwrap();
                send.send(Message::Render).unwrap();
            }
            KeyCode::Char('s' | 'j') | KeyCode::Down => {
                send.send(Message::ShiftRow(3)).unwrap();
                send.send(Message::Render).unwrap();
            }
            KeyCode::Char(' ') => {
                send.send(Message::PlayPause).unwrap();
            }
            KeyCode::Tab => {
                send.send(Message::Step).unwrap();
            }
            KeyCode::Char('+') => {
                send.send(Message::ShiftDelay(-20)).unwrap();
            }
            KeyCode::Char('-') => {
                send.send(Message::ShiftDelay(20)).unwrap();
            }
            KeyCode::Char('r') => {
                send.send(Message::Randomize).unwrap();
                send.send(Message::Render).unwrap();
            }
            KeyCode::Char('c') => {
                send.send(Message::GridClear).unwrap();
                send.send(Message::Render).unwrap();
            }
            KeyCode::Char(x @ '1'..='9') => {
                send.send(Message::SetState(x.to_digit(10).unwrap() as u16))
                    .unwrap();
            }
            _ => {}
        },
        Event::Resize(_, _) => {
            send.send(Message::ScreenClear).unwrap();
            send.send(Message::Render).unwrap();
        }
        Event::Mouse(MouseEvent {
            kind, column, row, ..
        }) => match kind {
            MouseEventKind::Down(b) | MouseEventKind::Drag(b) => match b {
                MouseButton::Left => send.send(Message::Draw(column, row)).unwrap(),
                MouseButton::Right => send.send(Message::Erase(column, row)).unwrap(),
                MouseButton::Middle => {}
            },
            MouseEventKind::ScrollUp => send.send(Message::CycleState(1)).unwrap(),
            MouseEventKind::ScrollDown => send.send(Message::CycleState(-1)).unwrap(),
            _ => {}
        },
        _ => {}
    };
}

fn main() {
    ctrlc::set_handler(term).ok();

    let mut args = std::env::args().skip(1);

    // Path to rule
    let path = args
        .next()
        .unwrap_or_else(|| die!("Please provide path to rule file"));
    if path == "--help" || path == "-h" {
        println!("{}", USAGE);
        std::process::exit(0);
    }
    let rule =
        std::fs::read_to_string(path).unwrap_or_else(|e| die!("Could not read rule file: {}", e));

    // Pattern path
    let mut pattern = None;
    let mut save_path = None;
    let mut delay: u64 = 100;

    let (term_cols, term_rows) = crossterm::terminal::size().unwrap();
    // Trim bottom of terminal
    let term_rows = term_rows - 3;
    // Because cells are 2 chars wide
    let mut cols = term_cols as usize / 2;
    let mut rows = term_rows as usize;

    // Handle cli options
    while let Some(s) = args.next() {
        if s == "--pattern" || s == "-p" {
            let path = args
                .next()
                .unwrap_or_else(|| die!("--pattern requires argument"));
            pattern = Some(
                std::fs::read_to_string(path)
                    .unwrap_or_else(|e| die!("Could not read pattern: {}", e)),
            );
        }
        if s == "--delay" || s == "-d" {
            delay = args
                .next()
                .unwrap_or_else(|| die!("--delay requires argument"))
                .parse()
                .unwrap_or_else(|_| die!("invalid delay"));
        }
        if s == "--size" || s == "-s" {
            cols = args
                .next()
                .unwrap_or_else(|| die!("--size requires two arguments [rows, cols]"))
                .parse()
                .unwrap_or_else(|_| die!("invalid size"));
            rows = args
                .next()
                .unwrap_or_else(|| die!("--size requires two arguments [rows, cols]"))
                .parse()
                .unwrap_or_else(|_| die!("invalid size"));
        }
        if s == "--save" || s == "-sp" {
            save_path = Some(
                args.next()
                    .unwrap_or_else(|| die!("--save requires argument")),
            );
        }
        if s == "--help" || s == "-h" {
            println!("{}", USAGE);
            std::process::exit(0);
        }
    }

    // Read pattern
    let pattern: Option<Vec<Vec<u16>>> = pattern.map(|x| deserialize_pattern(&x));
    let mut rng = thread_rng();

    // Start lua
    let lua = Lua::new();

    // Load rule
    lua.load(&rule)
        .exec()
        .unwrap_or_else(|e| eprintln!("{}", e));

    let update: LuaFunction = lua
        .globals()
        .get("Update")
        .unwrap_or_else(|_| die!("No Update global"));

    let display: LuaFunction = lua
        .globals()
        .get("Display")
        .unwrap_or_else(|_| die!("No Display global"));

    let states: u16 = lua
        .globals()
        .get("States")
        .unwrap_or_else(|_| die!("No States global"));

    let randomize_start: bool = lua.globals().get("Randomize").unwrap_or(false);

    let mut grid = if let Some(pattern) = pattern {
        // Read pattern
        let mut pattern: Vec<Vec<u16>> = pattern
            .into_iter()
            .map(|x| {
                let mut x = x;
                while x.len() < cols {
                    x.push(0);
                }
                x
            })
            .collect();
        while pattern.len() < rows {
            pattern.push(vec![0; cols]);
        }
        pattern
    } else {
        // Empty pattern
        let mut p = vec![vec![0; cols]; rows];
        if randomize_start {
            for cell in p.iter_mut().flatten() {
                *cell = rng.gen_range(0..states);
            }
        }
        p
    };

    let (send, recv) = mpsc::channel::<Message>();

    // Startup messages
    send.send(Message::ScreenClear).unwrap(); // Clear at start
    send.send(Message::Render).unwrap(); // Draw at start

    // Input loop
    {
        let send = send.clone();
        std::thread::spawn(move || {
            loop {
                handle_input(&send);
            }
        });
    }

    let mut row_offset = 0;
    let mut col_offset = 0;
    let mut playing = false;
    // What state is being drawm
    let mut draw_state: u16 = 1;

    execute!(std::io::stdout(), Hide).unwrap();
    execute!(std::io::stdout(), EnableMouseCapture).unwrap();
    enable_raw_mode().unwrap();

    // For use in drawing
    // Takes in pixel coords, not grid coords.
    let render_pixel = {
        let display = &display;
        move |i, j, n, term_rows, term_cols| {
            let i = (i as i16 - row_offset).rem_euclid(rows as i16) as usize;
            let j = (j as i16 - col_offset).rem_euclid(cols as i16) as usize;
            let row_repeats = term_rows as usize / rows + 2;
            let col_repeats = term_cols as usize / (cols * 2) + 2;
            for q in 0..row_repeats {
                for w in 0..col_repeats {
                    let i = ((i + q * rows) as i16 + row_offset) as u16;
                    let j = ((j + w * cols) as i16 + col_offset) as u16 * 2;
                    if i < term_rows && j < term_cols {
                        execute!(std::io::stdout(), MoveTo(j, i)).unwrap();
                        println!(
                            "{}",
                            normalize_cell(
                                &display
                                    .call::<u16, String>(n)
                                    .unwrap_or_else(|e| die!("Error in Display function:\n{}", e))
                            )
                        );
                    }
                }
            }
        }
    };

    // Main loop
    for message in recv.iter() {
        let (term_cols, term_rows) = crossterm::terminal::size().unwrap();
        let term_rows = term_rows - 3;

        match message {
            Message::Exit => break,
            Message::ScreenClear => execute!(std::io::stdout(), Clear(ClearType::All)).unwrap(),
            Message::ShiftRow(n) => {
                row_offset += n;
                row_offset = row_offset.rem_euclid(rows as i16);
            }
            Message::ShiftCol(n) => {
                col_offset += n;
                col_offset = col_offset.rem_euclid(cols as i16);
            }
            Message::ShiftDelay(n) => delay = (delay as i16 + n).clamp(0, 1000) as u64,
            Message::Render => {
                for i in 0..term_rows {
                    let di = (i as i16 + row_offset).rem_euclid(rows as i16) as usize;
                    for j in 0..term_cols {
                        let dj = (j as i16 + col_offset).rem_euclid(cols as i16) as usize;
                        execute!(std::io::stdout(), MoveTo(j as u16 * 2, i as u16)).unwrap();
                        print!(
                            "{}",
                            normalize_cell(
                                &display
                                    .call::<u16, String>(grid[di][dj])
                                    .unwrap_or_else(|_| die!("Invalid Display function"))
                            )
                        );
                    }
                }

                execute!(std::io::stdout(), MoveTo(0, term_rows)).unwrap();
                if term_cols > CONTROLS.len() as u16 + 1 {
                    println!(" {}", CONTROLS);
                }
                execute!(std::io::stdout(), MoveTo(0, term_rows + 1)).unwrap();
                if term_cols > CONTROLS2.len() as u16 + 7 {
                    println!(" {} - {}", CONTROLS2, draw_state);
                }
            }
            Message::Step => {
                // If we are playing, wait the delay and take another step
                if playing {
                    let send = send.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                        send.send(Message::Step).unwrap();
                    });
                }

                // This stores pairs of grid coords + their new values
                let mut diff = vec![];
                for i in 0..rows {
                    for j in 0..cols {
                        let new = update
                            .call((
                                grid[i][j],
                                neighborhood(
                                    Topology::Torus,
                                    cols,
                                    rows,
                                    j,
                                    i,
                                    Neighborhood::Square,
                                )
                                .map(|(x, y)| grid[y][x])
                                .collect::<Vec<u16>>(),
                            ))
                            .unwrap_or_else(|e| die!("Error in Update function:\n{}", e));
                        if new != grid[i][j] {
                            diff.push((i, j, new));
                        }
                    }
                }
                let row_repeats = term_rows as usize / rows + 2;
                let col_repeats = term_cols as usize / (cols * 2) + 2;
                let pl = diff.len() > 0;
                for (i, j, n) in diff {
                    grid[i][j] = n;
                    // Incremental draw
                    for q in 0..row_repeats {
                        for w in 0..col_repeats {
                            let i = ((i + q * rows) as i16 - row_offset) as u16;
                            let j = ((j + w * cols) as i16 - col_offset) as u16 * 2;
                            if i < term_rows && j < term_cols {
                                execute!(std::io::stdout(), MoveTo(j, i)).unwrap();
                                print!(
                                    "{}",
                                    normalize_cell(&display.call::<u16, String>(n).unwrap_or_else(
                                        |e| die!("Error in Display function:\n{}", e)
                                    ))
                                );
                            }
                        }
                    }
                }
                if pl {
                    println!();
                }
            }
            Message::PlayPause => {
                playing = !playing;
                if playing {
                    send.send(Message::Step).unwrap();
                }
            }
            Message::CycleState(n) => {
                draw_state = (draw_state as i16 - n).clamp(1, states as i16 - 1) as u16;

                execute!(std::io::stdout(), MoveTo(0, term_rows)).unwrap();
                if term_cols > CONTROLS.len() as u16 + 1 {
                    println!(" {}", CONTROLS);
                }
                execute!(std::io::stdout(), MoveTo(0, term_rows + 1)).unwrap();
                if term_cols > CONTROLS2.len() as u16 + 7 {
                    println!(" {} - {}", CONTROLS2, draw_state);
                }
            }
            Message::SetState(n) => {
                draw_state = n.clamp(1, states - 1);

                execute!(std::io::stdout(), MoveTo(0, term_rows)).unwrap();
                if term_cols > CONTROLS.len() as u16 + 1 {
                    println!(" {}", CONTROLS);
                }
                execute!(std::io::stdout(), MoveTo(0, term_rows + 1)).unwrap();
                if term_cols > CONTROLS2.len() as u16 + 7 {
                    println!(" {} - {}", CONTROLS2, draw_state);
                }
            }
            Message::Draw(j, i) => {
                let j = j / 2;
                let di = (i as i16 + row_offset).rem_euclid(rows as i16) as usize;
                let dj = (j as i16 + col_offset).rem_euclid(cols as i16) as usize;

                grid[di][dj] = draw_state;

                render_pixel(i as usize, j as usize, draw_state, term_rows, term_cols);
            }
            Message::Erase(j, i) => {
                let j = j / 2;

                let di = (i as i16 + row_offset).rem_euclid(rows as i16) as usize;
                let dj = (j as i16 + col_offset).rem_euclid(cols as i16) as usize;
                grid[di][dj] = 0;

                render_pixel(i as usize, j as usize, 0, term_rows, term_cols);
            }
            Message::Randomize => {
                for cell in grid.iter_mut().flatten() {
                    *cell = rng.gen_range(0..states);
                }
            }
            Message::GridClear => {
                for cell in grid.iter_mut().flatten() {
                    *cell = 0;
                }
            }
        }
    }

    execute!(std::io::stdout(), Show).unwrap();
    execute!(std::io::stdout(), Show).unwrap();
    execute!(std::io::stdout(), DisableMouseCapture).unwrap();
    execute!(std::io::stdout(), Clear(ClearType::All)).unwrap();
    execute!(std::io::stdout(), MoveTo(0, 0)).unwrap();
    disable_raw_mode().unwrap();
    if let Some(p) = save_path {
        let serialized = serialize_pattern(&grid);
        if std::fs::write(p, &serialized).is_err() {
            eprintln!("Could not write to file, printing to stdout:");
            println!("{}", serialized);
        };
    }
    std::process::exit(0);
}
