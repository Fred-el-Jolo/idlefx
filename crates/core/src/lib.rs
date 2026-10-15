//! idlefx-core — the rendering core shared by the companion and the ship game.
//!
//! Three pieces:
//! - [`session`]: the terminal set up and always restored, frames wrapped in
//!   synchronized output;
//! - [`canvas`]: a transparent half-block pixel canvas;
//! - [`Runner`]: the frame loop. It drives an [`App`] on a fixed simulation
//!   step, draws at the rate the app's [`Activity`] asks for (up to 60fps,
//!   a few fps, or none at all), and can keep CPU use under a budget.

pub mod budget;
pub mod canvas;
pub mod session;

use std::io;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, KeyEvent, MouseEvent};
use ratatui::layout::Rect;

pub use budget::{CpuMeter, CpuSource, Governor};
pub use canvas::{Canvas, Pixel, Rgb};
pub use session::{Session, SessionOptions};

/// Simulation step: 120 updates per second.
pub const STEP: Duration = Duration::from_nanos(1_000_000_000 / 120);

/// How often the app needs frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    /// Something moves fast: up to 60fps (less under a tight CPU budget).
    Active,
    /// Small motion: `n` frames per second, at most 6.
    Idle(u32),
    /// Nothing moves: no frame until input, a resize, a wake-up, or the
    /// given delay (the app's next timer).
    Asleep(Option<Duration>),
}

/// What reaches [`App::event`].
#[derive(Clone, Debug)]
pub enum Event<E> {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    /// An event sent through a [`Waker`].
    App(E),
}

/// What the loop does after an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// Nothing visible changed.
    Continue,
    /// Draw a frame now.
    Redraw,
    Quit,
}

/// An application driven by the frame loop.
pub trait App {
    /// Events sent from other threads through a [`Waker`].
    type Event: Send + 'static;

    /// Advance the simulation by one fixed step of `dt` seconds.
    fn update(&mut self, dt: f32);

    /// Draw into the (cleared) canvas; `alpha` is how far the real time is
    /// between the last step and the next, for interpolation.
    fn draw(&mut self, canvas: &mut Canvas, alpha: f32);

    /// Draw text or cells over the canvas, after it is blitted.
    fn overlay(&mut self, _buf: &mut Buffer, _area: Rect) {}

    /// The frame rate needed now; asked after each step and event.
    fn activity(&self) -> Activity;

    /// Input, resize or outside event, received at `at`.
    fn event(&mut self, event: Event<Self::Event>, at: Instant) -> Flow;

    /// Called right after each frame reaches the terminal.
    fn frame_done(&mut self, _at: Instant) {}
}

/// Loop options.
#[derive(Clone, Debug)]
pub struct Options {
    pub session: SessionOptions,
    /// CPU budget as a share of one core (0.10 = 10%), or none.
    pub budget: Option<f64>,
    /// Stop after this many frames.
    pub frames: Option<u64>,
    /// Stop after this long.
    pub duration: Option<Duration>,
    /// Start the stats this long after the start.
    pub measure_after: Duration,
    /// Test only: pretend each frame takes this long to reach the terminal.
    pub slow_write: Option<Duration>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            session: SessionOptions::default(),
            budget: None,
            frames: None,
            duration: None,
            measure_after: Duration::ZERO,
            slow_write: None,
        }
    }
}

/// What a run measured, from `measure_after` to the end.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    /// Frames drawn in the measured window.
    pub frames: u64,
    /// Wall time of the measured window.
    pub wall: Duration,
    /// Process CPU time in the measured window.
    pub cpu: Duration,
    /// Simulated time over the whole run.
    pub sim: Duration,
    /// Wall time of the whole run.
    pub total: Duration,
}

impl Stats {
    pub fn fps(&self) -> f64 {
        ratio(self.frames as f64, self.wall)
    }

    pub fn cpu_pct(&self) -> f64 {
        100.0 * ratio(self.cpu.as_secs_f64(), self.wall)
    }

    /// Simulated time over wall time: 1.0 when the simulation keeps real time.
    pub fn sim_wall(&self) -> f64 {
        ratio(self.sim.as_secs_f64(), self.total)
    }

    /// `frames=… fps=… cpu_ms=… cpu_pct=… sim_wall=…`
    pub fn line(&self) -> String {
        format!(
            "frames={} fps={:.1} cpu_ms={} cpu_pct={:.2} sim_wall={:.3}",
            self.frames,
            self.fps(),
            self.cpu.as_millis(),
            self.cpu_pct(),
            self.sim_wall()
        )
    }
}

fn ratio(x: f64, over: Duration) -> f64 {
    let secs = over.as_secs_f64();
    if secs > 0.0 { x / secs } else { 0.0 }
}

enum Msg<E> {
    Input(event::Event, Instant),
    App(E, Instant),
}

/// Sends app events into the loop from any thread; wakes it if it sleeps.
pub struct Waker<E> {
    tx: Sender<Msg<E>>,
}

impl<E> Clone for Waker<E> {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

impl<E: Send + 'static> Waker<E> {
    /// Deliver `event`; false once the loop has ended.
    pub fn wake(&self, event: E) -> bool {
        self.tx.send(Msg::App(event, Instant::now())).is_ok()
    }
}

/// The frame loop. Create it, hand out [`Waker`]s, then [`Runner::run`].
pub struct Runner<E> {
    tx: Sender<Msg<E>>,
    rx: Receiver<Msg<E>>,
}

impl<E: Send + 'static> Default for Runner<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Send + 'static> Runner<E> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self { tx, rx }
    }

    pub fn waker(&self) -> Waker<E> {
        Waker {
            tx: self.tx.clone(),
        }
    }

    /// Run `app` until it quits, or until the frame or time limit.
    pub fn run<A: App<Event = E>>(self, app: &mut A, options: &Options) -> io::Result<Stats> {
        let mut session = Session::enter(options.session)?;
        spawn_input(self.tx.clone());
        let stats = drive(app, options, &mut session, &self.rx);
        drop(session);
        stats
    }
}

/// Read terminal input on its own thread, blocking, and forward it.
fn spawn_input<E: Send + 'static>(tx: Sender<Msg<E>>) {
    let _ = std::thread::Builder::new()
        .name("idlefx-input".into())
        .spawn(move || {
            while let Ok(ev) = event::read() {
                if tx.send(Msg::Input(ev, Instant::now())).is_err() {
                    break;
                }
            }
        });
}

fn drive<A: App>(
    app: &mut A,
    options: &Options,
    session: &mut Session,
    rx: &Receiver<Msg<A::Event>>,
) -> io::Result<Stats> {
    let start = Instant::now();
    let end = options.duration.map(|d| start + d);
    let mut governor = options
        .budget
        .map(|limit| Governor::new(limit, CpuMeter::new(CpuSource::detect())));
    let mut cpu = CpuMeter::process();
    let mut canvas = Canvas::default();

    let mut measure: Option<(Instant, Duration)> = None;
    let mut frames_total: u64 = 0;
    let mut frames_measured: u64 = 0;
    let mut prev = start;
    let mut acc = Duration::ZERO;
    let mut sim = Duration::ZERO;
    let mut last_frame: Option<Instant> = None;
    let mut dirty = true;

    loop {
        let now = Instant::now();
        if measure.is_none() && now - start >= options.measure_after {
            measure = Some((now, cpu.read()));
        }
        if end.is_some_and(|e| now >= e) || options.frames.is_some_and(|n| frames_total >= n) {
            break;
        }

        // Fixed-step simulation, catching up with real time.
        acc += (now - prev).min(Duration::from_secs(60));
        prev = now;
        while acc >= STEP {
            app.update(STEP.as_secs_f32());
            acc -= STEP;
            sim += STEP;
        }

        let rate = governor.as_ref().map_or(60, Governor::rate);
        let interval = match app.activity() {
            Activity::Active => Some(Duration::from_secs(1) / rate),
            Activity::Idle(n) => Some(Duration::from_secs(1) / n.clamp(1, 6)),
            Activity::Asleep(_) => None,
        };
        let due = dirty
            || match (interval, last_frame) {
                (Some(i), Some(l)) => now >= l + i,
                (Some(_), None) => true,
                (None, _) => false,
            };
        if due {
            let alpha = acc.as_secs_f32() / STEP.as_secs_f32();
            session.draw(|frame| {
                let area = frame.area();
                canvas.fit(area);
                canvas.clear();
                app.draw(&mut canvas, alpha);
                canvas.blit(frame.buffer_mut(), area);
                app.overlay(frame.buffer_mut(), area);
            })?;
            if let Some(d) = options.slow_write {
                std::thread::sleep(d);
            }
            let done = Instant::now();
            app.frame_done(done);
            last_frame = Some(now);
            dirty = false;
            frames_total += 1;
            if measure.is_some() {
                frames_measured += 1;
            }
            if let Some(g) = governor.as_mut() {
                g.tick(done);
            }
        }

        // Sleep until the next frame, the app's timer or the end; wake early
        // on input or an app event.
        let next = match app.activity() {
            Activity::Asleep(timer) => timer.map(|d| Instant::now() + d),
            _ => interval.zip(last_frame).map(|(i, l)| l + i),
        };
        let next = match (next, end) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let first = match next {
            Some(t) => match rx.recv_timeout(t.saturating_duration_since(Instant::now())) {
                Ok(msg) => Some(msg),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(msg) => Some(msg),
                Err(_) => break,
            },
        };
        let mut quit = false;
        for msg in first.into_iter().chain(std::iter::from_fn(|| rx.try_recv().ok())) {
            let flow = match msg {
                Msg::Input(event::Event::Key(k), at) => app.event(Event::Key(k), at),
                Msg::Input(event::Event::Mouse(m), at) => app.event(Event::Mouse(m), at),
                Msg::Input(event::Event::Resize(w, h), at) => {
                    dirty = true;
                    app.event(Event::Resize(w, h), at)
                }
                Msg::Input(_, _) => Flow::Continue,
                Msg::App(e, at) => app.event(Event::App(e), at),
            };
            match flow {
                Flow::Continue => {}
                Flow::Redraw => dirty = true,
                Flow::Quit => {
                    quit = true;
                    break;
                }
            }
        }
        if quit {
            break;
        }
    }

    let now = Instant::now();
    let (from, cpu_from) = measure.unwrap_or((now, cpu.read()));
    Ok(Stats {
        frames: frames_measured,
        wall: now - from,
        cpu: cpu.read().saturating_sub(cpu_from),
        sim,
        total: now - start,
    })
}
