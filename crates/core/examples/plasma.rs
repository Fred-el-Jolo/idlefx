//! plasma — the idlefx plasma prototype.
//!
//! Run: `cargo run --release --example plasma [-- --frames N]`.
//!
//! A half-block framebuffer (2 truecolor pixels per cell) rendered through
//! ratatui at 60fps, each frame wrapped in synchronized output.
//!
//! Keys: `q` / `Esc` quit, `space` pause, `f` toggle the fps overlay.
//! `--frames N` renders N frames, then prints frame stats and exits.

use std::f32::consts::TAU;
use std::io;
use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::crossterm::queue;
use ratatui::crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use ratatui::crossterm::{cursor, execute};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

const FPS: f64 = 60.0;

/// RGB pixels, two per terminal cell (top and bottom half).
struct Framebuffer {
    width: usize,
    height: usize,
    pixels: Vec<[u8; 3]>,
}

impl Framebuffer {
    fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        }
    }

    /// Match the terminal area; reallocates only when the size changes.
    fn fit(&mut self, area: Rect) {
        let (w, h) = (area.width as usize, area.height as usize * 2);
        if (w, h) != (self.width, self.height) {
            self.width = w;
            self.height = h;
            self.pixels.resize(w * h, [0; 3]);
        }
    }

    /// Write the pixels into a ratatui buffer as `▀` cells: fg = top, bg = bottom.
    fn blit(&self, buf: &mut Buffer, area: Rect) {
        for cy in 0..area.height as usize {
            for x in 0..self.width {
                let top = self.pixels[(cy * 2) * self.width + x];
                let bottom = self.pixels[(cy * 2 + 1) * self.width + x];
                buf[(area.x + x as u16, area.y + cy as u16)]
                    .set_symbol("▀")
                    .set_fg(Color::Rgb(top[0], top[1], top[2]))
                    .set_bg(Color::Rgb(bottom[0], bottom[1], bottom[2]));
            }
        }
    }
}

/// Classic sum-of-sines plasma, colored with a cosine palette.
fn plasma(fb: &mut Framebuffer, t: f32) {
    // Normalize by height so pixels stay square whatever the aspect ratio.
    let scale = 1.0 / fb.height.max(1) as f32;
    let (cx, cy) = (0.5 * (t * 0.31).sin(), 0.5 * (t * 0.23).cos());
    for y in 0..fb.height {
        let v = y as f32 * scale * 4.0;
        for x in 0..fb.width {
            let u = x as f32 * scale * 4.0;
            let (dx, dy) = (
                u - 2.0 * fb.width as f32 * scale - cx * 3.0,
                v - 2.0 - cy * 3.0,
            );
            let s = (u + t).sin()
                + ((v + t * 0.7) * 0.8).sin()
                + ((u + v + t * 0.9) * 0.6).sin()
                + ((dx * dx + dy * dy).sqrt() * 1.6 - t * 1.3).sin();
            fb.pixels[y * fb.width + x] = palette(s * 0.125 + 0.5 + t * 0.03);
        }
    }
}

/// Cosine palette: a + b * cos(2π (c * x + d)).
fn palette(x: f32) -> [u8; 3] {
    const A: [f32; 3] = [0.5, 0.5, 0.5];
    const B: [f32; 3] = [0.5, 0.5, 0.5];
    const C: [f32; 3] = [1.0, 1.0, 1.0];
    const D: [f32; 3] = [0.0, 0.10, 0.20];
    let channel = |i: usize| {
        let v = A[i] + B[i] * (TAU * (C[i] * x + D[i])).cos();
        (v.clamp(0.0, 1.0) * 255.0) as u8
    };
    [channel(0), channel(1), channel(2)]
}

struct Stats {
    frames: u64,
    work: Duration,
    compute: Duration,
    first: Option<Instant>,
    last: Option<Instant>,
}

impl Stats {
    fn fps(&self) -> f64 {
        match (self.first, self.last) {
            (Some(a), Some(b)) if self.frames > 1 => {
                (self.frames - 1) as f64 / (b - a).as_secs_f64()
            }
            _ => 0.0,
        }
    }

    fn avg_frame_us(&self) -> u128 {
        self.work.as_micros() / u128::from(self.frames.max(1))
    }

    fn avg_compute_us(&self) -> u128 {
        self.compute.as_micros() / u128::from(self.frames.max(1))
    }
}

fn run(terminal: &mut DefaultTerminal, max_frames: Option<u64>) -> io::Result<Stats> {
    let frame_dt = Duration::from_secs_f64(1.0 / FPS);
    let mut fb = Framebuffer::new();
    let mut stats = Stats {
        frames: 0,
        work: Duration::ZERO,
        compute: Duration::ZERO,
        first: None,
        last: None,
    };
    let mut overlay = String::with_capacity(64);
    let mut show_fps = false;
    let mut paused = false;
    let mut t = 0.0_f32;
    let mut prev = Instant::now();
    let mut next = prev;

    loop {
        if max_frames.is_some_and(|n| stats.frames >= n) {
            return Ok(stats);
        }

        // Time-based animation: advance by real elapsed time, not by frames.
        let now = Instant::now();
        if !paused {
            t += (now - prev).as_secs_f32();
        }
        prev = now;

        let start = Instant::now();
        queue!(terminal.backend_mut(), BeginSynchronizedUpdate)?;
        terminal.draw(|frame| {
            let area = frame.area();
            let compute = Instant::now();
            fb.fit(area);
            plasma(&mut fb, t);
            fb.blit(frame.buffer_mut(), area);
            stats.compute += compute.elapsed();
            if show_fps {
                use std::fmt::Write;
                overlay.clear();
                let _ = write!(
                    overlay,
                    " {:.1} fps · {} µs/frame ",
                    stats.fps(),
                    stats.avg_frame_us()
                );
                let style = Style::new().fg(Color::White).bg(Color::Black);
                frame
                    .buffer_mut()
                    .set_string(area.x, area.y, &overlay, style);
            }
        })?;
        execute!(terminal.backend_mut(), EndSynchronizedUpdate)?;
        stats.work += start.elapsed();
        stats.frames += 1;
        stats.first.get_or_insert(start);
        stats.last = Some(start);

        // Fixed schedule; resync instead of bursting if we fell behind.
        next += frame_dt;
        let now = Instant::now();
        if next < now {
            next = now;
        }

        // Wait for the next frame, handling input meanwhile. Paused: block on
        // input only, so an idle pause costs no CPU.
        loop {
            let timeout = if paused {
                Duration::from_secs(3600)
            } else {
                next.saturating_duration_since(Instant::now())
            };
            if !event::poll(timeout)? {
                if paused {
                    continue;
                }
                break;
            }
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(stats),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(stats);
                    }
                    KeyCode::Char(' ') => {
                        paused = !paused;
                        prev = Instant::now();
                        next = prev;
                        break;
                    }
                    KeyCode::Char('f') => {
                        show_fps = !show_fps;
                        break;
                    }
                    _ => {}
                },
                Event::Resize(..) => break,
                _ => {}
            }
        }
    }
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

fn main() -> io::Result<()> {
    let max_frames = match parse_frames() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("plasma: {e}\nusage: plasma [--frames N]");
            std::process::exit(2);
        }
    };

    // ratatui::init enters raw mode and the alternate screen, and installs a
    // panic hook that restores the terminal.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, max_frames);
    ratatui::restore();
    execute!(io::stdout(), cursor::Show)?;

    let stats = result?;
    if max_frames.is_some() {
        println!(
            "frames={} fps={:.1} avg_frame_us={} avg_compute_us={}",
            stats.frames,
            stats.fps(),
            stats.avg_frame_us(),
            stats.avg_compute_us()
        );
    }
    Ok(())
}
