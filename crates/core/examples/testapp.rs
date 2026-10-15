//! testapp — a small app that exercises the core frame loop, for tests.
//!
//! `--mode active|idle|asleep` picks its activity; the other options bend the
//! run (`--burn-ms` makes each frame expensive, `--slow-write-ms` a slow
//! terminal, `--wake-after-ms` sends a `Waker` event, `--panic-after` panics).
//! On exit it prints the run's stats, plus `wake_latency_us` after a wake-up.

use std::time::{Duration, Instant};

use idlefx_core::{Activity, App, Canvas, Event, Flow, Options, Rgb, Runner};
use ratatui::crossterm::event::{KeyCode, KeyEventKind};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Active,
    Idle(u32),
    Asleep,
}

struct TestApp {
    mode: Mode,
    t: f32,
    frames: u64,
    burn: Duration,
    panic_after: Option<u64>,
    woken: Option<Instant>,
    wake_latency: Option<Duration>,
}

impl App for TestApp {
    type Event = ();

    fn update(&mut self, dt: f32) {
        self.t += dt;
    }

    fn draw(&mut self, canvas: &mut Canvas, _alpha: f32) {
        if self.panic_after.is_some_and(|n| self.frames >= n) {
            panic!("testapp: forced panic after {} frames", self.frames);
        }
        let (w, h) = (canvas.width() as f32, canvas.height() as f32);
        let x = (self.t * 20.0) % w.max(1.0);
        canvas.ellipse(x, h / 2.0, 3.0, 3.0, 0.0, Rgb(240, 160, 60));
        let start = Instant::now();
        while start.elapsed() < self.burn {
            std::hint::spin_loop();
        }
    }

    fn activity(&self) -> Activity {
        match self.mode {
            Mode::Active => Activity::Active,
            Mode::Idle(n) => Activity::Idle(n),
            Mode::Asleep => Activity::Asleep(None),
        }
    }

    fn event(&mut self, event: Event<()>, at: Instant) -> Flow {
        match event {
            Event::Key(k) if k.kind == KeyEventKind::Press && k.code == KeyCode::Char('q') => {
                Flow::Quit
            }
            Event::App(()) => {
                self.woken = Some(at);
                Flow::Redraw
            }
            _ => Flow::Continue,
        }
    }

    fn frame_done(&mut self, at: Instant) {
        self.frames += 1;
        if let Some(sent) = self.woken.take() {
            self.wake_latency = Some(at - sent);
        }
    }
}

fn arg<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    let i = args.iter().position(|a| a == name)?;
    args.get(i + 1)?.parse().ok()
}

fn secs(args: &[String], name: &str) -> Option<Duration> {
    arg::<f64>(args, name).map(Duration::from_secs_f64)
}

fn millis(args: &[String], name: &str) -> Option<Duration> {
    arg::<u64>(args, name).map(Duration::from_millis)
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = match arg::<String>(&args, "--mode").as_deref() {
        Some("idle") => Mode::Idle(arg(&args, "--idle-fps").unwrap_or(4)),
        Some("asleep") => Mode::Asleep,
        _ => Mode::Active,
    };
    let mut options = Options {
        budget: arg(&args, "--budget"),
        frames: arg(&args, "--frames"),
        duration: secs(&args, "--duration"),
        measure_after: secs(&args, "--measure-after").unwrap_or_default(),
        slow_write: millis(&args, "--slow-write-ms"),
        ..Options::default()
    };
    options.session.mouse = args.iter().any(|a| a == "--mouse");

    let mut app = TestApp {
        mode,
        t: 0.0,
        frames: 0,
        burn: millis(&args, "--burn-ms").unwrap_or_default(),
        panic_after: arg(&args, "--panic-after"),
        woken: None,
        wake_latency: None,
    };
    let runner = Runner::new();
    if let Some(delay) = millis(&args, "--wake-after-ms") {
        let waker = runner.waker();
        std::thread::spawn(move || {
            std::thread::sleep(delay);
            waker.wake(());
        });
    }
    let stats = runner.run(&mut app, &options)?;
    let latency = app
        .wake_latency
        .map(|d| format!(" wake_latency_us={}", d.as_micros()))
        .unwrap_or_default();
    println!("{}{latency}", stats.line());
    Ok(())
}
