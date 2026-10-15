//! plasma — the idlefx plasma prototype, on the core frame loop.
//!
//! Run: `cargo run --release --example plasma [-- --frames N]`.
//!
//! A half-block canvas (2 truecolor pixels per cell) rendered at 60fps, each
//! frame wrapped in synchronized output.
//!
//! Keys: `q` / `Esc` quit, `space` pause, `f` toggle the fps overlay.
//! `--frames N` renders N frames, then prints frame stats and exits.

use std::f32::consts::TAU;
use std::fmt::Write;
use std::time::{Duration, Instant};

use idlefx_core::{Activity, App, Canvas, Event, Flow, Options, Rgb, Runner};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

struct Plasma {
    t: f32,
    paused: bool,
    show_fps: bool,
    overlay: String,
    frames: u64,
    first: Option<Instant>,
    last: Option<Instant>,
    compute: Duration,
}

impl Plasma {
    fn fps(&self) -> f64 {
        match (self.first, self.last) {
            (Some(a), Some(b)) if self.frames > 1 => {
                (self.frames - 1) as f64 / (b - a).as_secs_f64()
            }
            _ => 0.0,
        }
    }
}

impl App for Plasma {
    type Event = ();

    fn update(&mut self, dt: f32) {
        if !self.paused {
            self.t += dt;
        }
    }

    fn draw(&mut self, canvas: &mut Canvas, _alpha: f32) {
        let start = Instant::now();
        plasma(canvas, self.t);
        self.compute += start.elapsed();
    }

    fn overlay(&mut self, buf: &mut Buffer, area: Rect) {
        if self.show_fps {
            self.overlay.clear();
            let _ = write!(self.overlay, " {:.1} fps ", self.fps());
            let style = Style::new().fg(Color::White).bg(Color::Black);
            buf.set_string(area.x, area.y, &self.overlay, style);
        }
    }

    fn activity(&self) -> Activity {
        if self.paused {
            Activity::Asleep(None)
        } else {
            Activity::Active
        }
    }

    fn event(&mut self, event: Event<()>, _at: Instant) -> Flow {
        let Event::Key(key) = event else {
            return Flow::Continue;
        };
        if key.kind != KeyEventKind::Press {
            return Flow::Continue;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Flow::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Quit;
            }
            KeyCode::Char(' ') => self.paused = !self.paused,
            KeyCode::Char('f') => self.show_fps = !self.show_fps,
            _ => return Flow::Continue,
        }
        Flow::Redraw
    }

    fn frame_done(&mut self, at: Instant) {
        self.frames += 1;
        self.first.get_or_insert(at);
        self.last = Some(at);
    }
}

/// Classic sum-of-sines plasma, colored with a cosine palette.
fn plasma(canvas: &mut Canvas, t: f32) {
    let (width, height) = (canvas.width(), canvas.height());
    // Normalize by height so pixels stay square whatever the aspect ratio.
    let scale = 1.0 / height.max(1) as f32;
    let (cx, cy) = (0.5 * (t * 0.31).sin(), 0.5 * (t * 0.23).cos());
    for y in 0..height {
        let v = y as f32 * scale * 4.0;
        for x in 0..width {
            let u = x as f32 * scale * 4.0;
            let (dx, dy) = (
                u - 2.0 * width as f32 * scale - cx * 3.0,
                v - 2.0 - cy * 3.0,
            );
            let s = (u + t).sin()
                + ((v + t * 0.7) * 0.8).sin()
                + ((u + v + t * 0.9) * 0.6).sin()
                + ((dx * dx + dy * dy).sqrt() * 1.6 - t * 1.3).sin();
            canvas.set(x, y, Some(palette(s * 0.125 + 0.5 + t * 0.03)));
        }
    }
}

/// Cosine palette: a + b * cos(2π (c * x + d)).
fn palette(x: f32) -> Rgb {
    const A: [f32; 3] = [0.5, 0.5, 0.5];
    const B: [f32; 3] = [0.5, 0.5, 0.5];
    const C: [f32; 3] = [1.0, 1.0, 1.0];
    const D: [f32; 3] = [0.0, 0.10, 0.20];
    let channel = |i: usize| {
        let v = A[i] + B[i] * (TAU * (C[i] * x + D[i])).cos();
        (v.clamp(0.0, 1.0) * 255.0) as u8
    };
    Rgb(channel(0), channel(1), channel(2))
}

fn parse_frames() -> Result<Option<u64>, String> {
    let mut args = std::env::args().skip(1);
    let mut frames = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--frames" => {
                let n = args.next().ok_or("--frames needs a number")?;
                frames = Some(n.parse().map_err(|_| format!("bad frame count: {n}"))?);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(frames)
}

fn main() -> std::io::Result<()> {
    let frames = match parse_frames() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("plasma: {e}\nusage: plasma [--frames N]");
            std::process::exit(2);
        }
    };
    let mut app = Plasma {
        t: 0.0,
        paused: false,
        show_fps: false,
        overlay: String::with_capacity(32),
        frames: 0,
        first: None,
        last: None,
        compute: Duration::ZERO,
    };
    let options = Options {
        frames,
        ..Options::default()
    };
    let stats = Runner::new().run(&mut app, &options)?;
    if frames.is_some() {
        println!(
            "{} avg_compute_us={}",
            stats.line(),
            app.compute.as_micros() / u128::from(app.frames.max(1))
        );
    }
    Ok(())
}
