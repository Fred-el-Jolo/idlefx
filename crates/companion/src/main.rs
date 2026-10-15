//! idlefx-companion — a small procedural dog living in its own terminal window.
//!
//! See AGENTS.md next to this crate for the states, events and desktop setup.

mod app;
mod brain;
mod control;
mod dog;
mod hooks;
mod launch;
mod socket;

use std::fs::File;
use std::process::ExitCode;
use std::time::Duration;

use idlefx_core::{Canvas, Options, Runner};

use brain::{Companion, State};
use dog::{Dog, Pose};

const USAGE: &str = "\
idlefx-companion — a small dog living in a terminal window

usage:
  idlefx-companion [options]          run the dog in this terminal
  idlefx-companion launch [--window COLSxROWS] [--dry-run]
                                      open its own floating kitty window (64x16)
  idlefx-companion poke <event>       send an event to the running dog (hooks call this)
  idlefx-companion poke --list        list the events and the dog's reactions
  idlefx-companion hooks install [claude] [git] [hypridle] [gnome] [--template]
                                      set up the hooks that poke the dog (no target: all
                                      that apply here; --template: git hooks for new repos too)
  idlefx-companion hooks status [targets]     which hooks are installed
  idlefx-companion hooks uninstall [targets] [--template]
                                      remove exactly what install added
  idlefx-companion status             is the dog running, and where
  idlefx-companion stop               close the launched window
  idlefx-companion stats              CPU and memory of the dog (and its window) every 2 s
  idlefx-companion snapshot --pose <pose>
                                      print one pose: stand walk sit lie sleep happy sad curious
                                      think stretch dig alert
  idlefx-companion simulate --start HH:MM --duration SECS [--seed N] [--trace FILE]
                   [--break-every MIN]
                                      run the behaviour headless on a fake clock

options:
  --trace FILE     write one line per state change (time from to cause)
  --stats          print frame, CPU and memory stats on exit
  --duration SECS  quit after this long
  --state STATE    start in a state and hold it until an input
                   (idle wander walk sit lie sleep happy sad curious thinking stretch
                   working alert run)
  --clock HH:MM    pretend the local time is this at start
  --at X           start at this x, in pixels
  --seed N         seed the behaviour's randomness
  --break-every MIN  remind you to take a break after MIN minutes of work, then
                   every 5 min until you leave (default 25; 0 turns it off)

keys: q, Esc or Ctrl-C quit. Click the dog to pet it, elsewhere to call it.";

fn arg<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1).map(String::as_str)
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn fail(message: &str) -> ExitCode {
    eprintln!("idlefx-companion: {message}");
    ExitCode::from(2)
}

/// Local time of day, in seconds.
fn local_tod() -> f64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as libc::time_t;
    // SAFETY: localtime_r writes only into `tm`, which we own.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&secs, &mut tm) }.is_null() {
        return 12.0 * 3600.0;
    }
    f64::from(tm.tm_hour * 3600 + tm.tm_min * 60 + tm.tm_sec) + f64::from(now.subsec_millis()) / 1000.0
}

fn seed(args: &[String]) -> Result<u64, String> {
    match arg(args, "--seed") {
        Some(s) => s.parse().map_err(|_| format!("bad seed: {s}")),
        None => Ok(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64)),
    }
}

fn tod(args: &[String], name: &str) -> Result<Option<f64>, String> {
    arg(args, name)
        .map(|s| brain::parse_tod(s).ok_or(format!("bad time (HH:MM): {s}")))
        .transpose()
}

fn secs(args: &[String], name: &str) -> Result<Option<Duration>, String> {
    arg(args, name)
        .map(|s| {
            s.parse::<f64>()
                .ok()
                .filter(|v| *v >= 0.0)
                .map(Duration::from_secs_f64)
                .ok_or(format!("bad duration: {s}"))
        })
        .transpose()
}

/// `--break-every MIN`, in seconds.
fn break_every(args: &[String]) -> Result<f64, String> {
    match arg(args, "--break-every") {
        Some(s) => s
            .parse::<f64>()
            .ok()
            .filter(|v| *v >= 0.0)
            .map(|v| v * 60.0)
            .ok_or(format!("bad break interval (minutes): {s}")),
        None => Ok(brain::BREAK_EVERY),
    }
}

fn max_rss_kb() -> i64 {
    // SAFETY: getrusage writes only into `usage`, which we own.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } == 0 {
        usage.ru_maxrss
    } else {
        0
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let start = match arg(args, "--state") {
        Some(s) => Some(State::parse(s).ok_or(format!("unknown state: {s}"))?),
        None => None,
    };
    let (cols, _) = ratatui::crossterm::terminal::size().unwrap_or((64, 16));
    let width = f32::from(cols);
    let x = match arg(args, "--at") {
        Some(s) => s.parse().map_err(|_| format!("bad x: {s}"))?,
        None => width / 2.0,
    };
    let tod = tod(args, "--clock")?.unwrap_or_else(local_tod);
    let trace = match arg(args, "--trace") {
        Some(p) => Some(File::create(p).map_err(|e| format!("cannot write {p}: {e}"))?),
        None => None,
    };
    let mut model = Companion::new(width, x, tod, seed(args)?, start);
    model.set_break_every(break_every(args)?);
    let mut app = app::CompanionApp::new(model, trace);

    let runner = Runner::new();
    let _listener = socket::listen(runner.waker());
    let mut options = Options {
        budget: Some(0.10),
        duration: secs(args, "--duration")?,
        frames: arg(args, "--frames").and_then(|s| s.parse().ok()),
        ..Options::default()
    };
    options.session.mouse = true;
    let stats = runner.run(&mut app, &options).map_err(|e| e.to_string())?;
    if flag(args, "--stats") {
        println!("{} max_rss_kb={}", stats.line(), max_rss_kb());
    }
    Ok(())
}

/// Print one pose inline, as truecolor half-blocks.
fn snapshot(args: &[String]) -> Result<(), String> {
    let name = arg(args, "--pose").ok_or("snapshot needs --pose")?;
    let pose = Pose::parse(name).ok_or(format!("unknown pose: {name}"))?;
    let mut canvas = Canvas::new(64, 32);
    let mut dog = Dog::new(32.0, pose);
    if pose == Pose::Walk {
        dog.walk_to(1000.0);
        for _ in 0..30 {
            dog.update(1.0 / 120.0);
        }
        dog.cur.x = 32.0;
        dog.prev = dog.cur;
    }
    dog::draw(&mut canvas, &dog, 1.0, None);
    let mut out = String::new();
    for cy in 0..canvas.height() / 2 {
        for x in 0..canvas.width() {
            let (top, bottom) = (canvas.get(x, cy * 2), canvas.get(x, cy * 2 + 1));
            match (top, bottom) {
                (None, None) => out.push_str("\x1b[0m "),
                (Some(t), None) => out.push_str(&format!("\x1b[0;38;2;{};{};{}m▀", t.0, t.1, t.2)),
                (None, Some(b)) => out.push_str(&format!("\x1b[0;38;2;{};{};{}m▄", b.0, b.1, b.2)),
                (Some(t), Some(b)) => out.push_str(&format!(
                    "\x1b[0;38;2;{};{};{};48;2;{};{};{}m▀",
                    t.0, t.1, t.2, b.0, b.1, b.2
                )),
            }
        }
        out.push_str("\x1b[0m\n");
    }
    print!("{out}");
    Ok(())
}

/// Run the behaviour on a fake clock, writing the trace.
fn simulate(args: &[String]) -> Result<(), String> {
    let start = tod(args, "--start")?.ok_or("simulate needs --start HH:MM")?;
    let duration = secs(args, "--duration")?.ok_or("simulate needs --duration SECS")?;
    let width = 64.0;
    let mut model = Companion::new(width, width / 2.0, start, seed(args)?, None);
    model.set_break_every(break_every(args)?);
    let mut out: Box<dyn std::io::Write> = match arg(args, "--trace") {
        Some(p) => Box::new(std::io::BufWriter::new(
            File::create(p).map_err(|e| format!("cannot write {p}: {e}"))?,
        )),
        None => Box::new(std::io::stdout()),
    };
    let dt = 1.0 / 120.0;
    let steps = (duration.as_secs_f64() / f64::from(dt)).round() as u64;
    let mut line = String::new();
    for i in 0..=steps {
        if i > 0 {
            model.step(dt);
        }
        for t in model.drain() {
            line.clear();
            t.write(&mut line);
            writeln!(out, "{line}").map_err(|e| e.to_string())?;
        }
    }
    out.flush().map_err(|e| e.to_string())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some("poke") => match args.get(1).map(String::as_str) {
            None | Some("--list" | "-l") => {
                for p in brain::Poke::ALL {
                    println!("{:<9} {}", p.name(), p.reaction());
                }
                Ok(())
            }
            Some(name) => {
                socket::poke(name);
                Ok(())
            }
        },
        Some("hooks") => return ExitCode::from(hooks::main(&args[1..]) as u8),
        Some("status") => return ExitCode::from(control::status() as u8),
        Some("stop") => return ExitCode::from(control::stop() as u8),
        Some("stats") => return ExitCode::from(control::stats() as u8),
        Some("launch") => {
            let window = arg(&args, "--window").unwrap_or(launch::DEFAULT_WINDOW);
            return ExitCode::from(launch::launch(window, flag(&args, "--dry-run")) as u8);
        }
        Some("snapshot") => snapshot(&args),
        Some("simulate") => simulate(&args),
        _ => run(&args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(&e),
    }
}
