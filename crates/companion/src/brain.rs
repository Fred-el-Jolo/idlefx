//! Behaviour: what the dog does, when, and why.
//!
//! A state machine over the dog's poses. Calm states (idle, sit, lie, nap)
//! pick the next one by weighted randomness after a random duration; the time
//! of day biases it (asleep from 22:00 to 07:00); clicks and pokes start
//! reactions. Every change of state is recorded as a [`Transition`] with its
//! cause, for the trace file. Time is simulated time, so the same code runs
//! live and headless.

use crate::dog::{Dog, HALF_LENGTH, Mark, Pose};

const DAY: f64 = 86_400.0;
const NIGHT_START: f64 = 22.0 * 3600.0;
const NIGHT_END: f64 = 7.0 * 3600.0;
/// After a change of state, keep full frame rate this long for the blend.
const BLEND: f64 = 1.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Wander,
    Walk,
    Sit,
    Lie,
    Sleep,
    Happy,
    Sad,
    Curious,
    Thinking,
    Stretch,
}

impl State {
    pub fn name(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Wander => "wander",
            State::Walk => "walk",
            State::Sit => "sit",
            State::Lie => "lie",
            State::Sleep => "sleep",
            State::Happy => "happy",
            State::Sad => "sad",
            State::Curious => "curious",
            State::Thinking => "thinking",
            State::Stretch => "stretch",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        [
            State::Idle,
            State::Wander,
            State::Walk,
            State::Sit,
            State::Lie,
            State::Sleep,
            State::Happy,
            State::Sad,
            State::Curious,
            State::Thinking,
            State::Stretch,
        ]
        .into_iter()
        .find(|s| s.name() == name)
    }

    fn pose(self) -> Pose {
        match self {
            State::Idle => Pose::Stand,
            State::Wander | State::Walk => Pose::Walk,
            State::Sit => Pose::Sit,
            State::Lie => Pose::Lie,
            State::Sleep => Pose::Sleep,
            State::Happy => Pose::Happy,
            State::Sad => Pose::Sad,
            State::Curious => Pose::Curious,
            State::Thinking => Pose::Think,
            State::Stretch => Pose::Stretch,
        }
    }
}

/// Events sent with `idlefx-companion poke <event>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poke {
    Hello,
    Thinking,
    Success,
    Failure,
    Commit,
}

impl Poke {
    pub const ALL: [Poke; 5] = [Poke::Hello, Poke::Thinking, Poke::Success, Poke::Failure, Poke::Commit];

    pub fn name(self) -> &'static str {
        match self {
            Poke::Hello => "hello",
            Poke::Thinking => "thinking",
            Poke::Success => "success",
            Poke::Failure => "failure",
            Poke::Commit => "commit",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// Why the state changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Start,
    Timer,
    Night,
    Morning,
    Done,
    Arrived,
    Pet,
    Click,
    Poke(Poke),
}

impl Cause {
    pub fn write(self, out: &mut String) {
        match self {
            Cause::Start => out.push_str("start"),
            Cause::Timer => out.push_str("timer"),
            Cause::Night => out.push_str("night"),
            Cause::Morning => out.push_str("morning"),
            Cause::Done => out.push_str("done"),
            Cause::Arrived => out.push_str("arrived"),
            Cause::Pet => out.push_str("pet"),
            Cause::Click => out.push_str("click"),
            Cause::Poke(p) => {
                out.push_str("poke:");
                out.push_str(p.name());
            }
        }
    }
}

/// What the user or the outside world did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    /// A click on the dog.
    Pet,
    /// A click elsewhere, at this x.
    Click(f32),
    Poke(Poke),
}

/// One change of state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transition {
    /// Local time of day, in seconds.
    pub tod: f64,
    pub from: Option<State>,
    pub to: State,
    pub cause: Cause,
    /// The dog's x, when it matters (arrival).
    pub x: Option<f32>,
}

impl Transition {
    /// `HH:MM:SS.mmm from to cause [x=…]`
    pub fn write(&self, out: &mut String) {
        use std::fmt::Write;
        write_tod(out, self.tod);
        let _ = write!(
            out,
            " {} {} ",
            self.from.map_or("-", State::name),
            self.to.name()
        );
        self.cause.write(out);
        if let Some(x) = self.x {
            let _ = write!(out, " x={x:.1}");
        }
    }
}

pub fn write_tod(out: &mut String, tod: f64) {
    use std::fmt::Write;
    let ms = (tod.rem_euclid(DAY) * 1000.0) as u64;
    let _ = write!(
        out,
        "{:02}:{:02}:{:02}.{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    );
}

/// Parse `HH:MM` (or `HH:MM:SS`) into seconds of the day.
pub fn parse_tod(text: &str) -> Option<f64> {
    let mut parts = text.split(':').map(|p| p.parse::<u32>().ok());
    let h = parts.next()??;
    let m = parts.next()??;
    let s = parts.next().unwrap_or(Some(0))?;
    (h < 24 && m < 60 && s < 60).then(|| f64::from(h * 3600 + m * 60 + s))
}

/// Small deterministic generator (xorshift64*).
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

/// The dog and its behaviour, in simulated time.
pub struct Companion {
    pub dog: Dog,
    state: State,
    /// Simulated seconds since start.
    t: f64,
    /// Time of day at start, in seconds.
    base_tod: f64,
    until: f64,
    changed_at: f64,
    width: f32,
    rng: Rng,
    /// Forced start state: kept until the first input.
    hold: bool,
    night_sleep: bool,
    mark: Option<Mark>,
    /// The current happy reaction came from petting (heart) or a commit (!).
    heart: bool,
    bang: bool,
    transitions: Vec<Transition>,
}

impl Companion {
    /// A dog at `x` in a canvas `width` pixels wide, starting at `tod`.
    /// With `start`, it begins in that state and holds it until an input.
    pub fn new(width: f32, x: f32, tod: f64, seed: u64, start: Option<State>) -> Self {
        let state = start.unwrap_or(State::Idle);
        let mut c = Companion {
            dog: Dog::new(x, state.pose()),
            state,
            t: 0.0,
            base_tod: tod,
            until: 0.0,
            changed_at: f64::NEG_INFINITY,
            width,
            rng: Rng::new(seed),
            hold: start.is_some(),
            night_sleep: false,
            mark: None,
            heart: false,
            bang: false,
            transitions: Vec::with_capacity(8),
        };
        c.clamp_x();
        c.until = c.t + c.duration(state);
        c.night_sleep = state == State::Sleep && c.night();
        if state == State::Walk {
            c.pace();
        }
        c.transitions.push(Transition {
            tod: c.tod(),
            from: None,
            to: state,
            cause: Cause::Start,
            x: None,
        });
        c
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn tod(&self) -> f64 {
        (self.base_tod + self.t).rem_euclid(DAY)
    }

    pub fn night(&self) -> bool {
        let tod = self.tod();
        !(NIGHT_END..NIGHT_START).contains(&tod)
    }

    /// Transitions since the last call.
    pub fn drain(&mut self) -> std::vec::Drain<'_, Transition> {
        self.transitions.drain(..)
    }

    pub fn mark(&self) -> Option<Mark> {
        self.mark
    }

    pub fn set_width(&mut self, width: f32) {
        self.width = width;
        self.clamp_x();
    }

    fn clamp_x(&mut self) {
        let (lo, hi) = self.x_range();
        let x = self.dog.x().clamp(lo, hi);
        self.dog.cur.x = x;
        self.dog.prev.x = x;
    }

    fn x_range(&self) -> (f32, f32) {
        let lo = HALF_LENGTH;
        (lo, (self.width - HALF_LENGTH).max(lo))
    }

    /// Whether the dog needs full frame rate now.
    pub fn busy(&self) -> bool {
        self.t - self.changed_at < BLEND
            || self.dog.moving()
            || matches!(
                self.state,
                State::Walk | State::Wander | State::Happy | State::Stretch
            )
    }

    /// Advance by one step of `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        self.t += f64::from(dt);
        let arrived = self.dog.update(dt);
        if arrived {
            match self.state {
                State::Wander | State::Walk if self.hold && self.state == State::Walk => self.pace(),
                State::Wander | State::Walk => {
                    let x = Some(self.dog.x());
                    self.go(State::Idle, Cause::Arrived, x);
                }
                _ => {}
            }
        }
        self.update_mark();
        if self.hold {
            return;
        }
        let night = self.night();
        if self.state == State::Sleep && self.night_sleep && !night {
            self.go(State::Stretch, Cause::Morning, None);
            return;
        }
        // Night shortens the calm states, so the dog settles down soon.
        if night
            && matches!(self.state, State::Idle | State::Sit | State::Lie)
            && self.until - self.t > 6.0
        {
            self.until = self.t + self.rng.range(2.0, 4.0);
        }
        if self.t >= self.until {
            self.next(night);
        }
    }

    /// React to an input.
    pub fn input(&mut self, input: Input) {
        self.hold = false;
        self.dog.stop();
        match input {
            Input::Pet => self.go(State::Happy, Cause::Pet, None),
            Input::Click(x) => {
                let (lo, hi) = self.x_range();
                self.dog.walk_to(x.clamp(lo, hi));
                self.go(State::Walk, Cause::Click, None);
            }
            Input::Poke(p) => {
                let to = match p {
                    Poke::Hello | Poke::Success | Poke::Commit => State::Happy,
                    Poke::Failure => State::Sad,
                    Poke::Thinking => State::Thinking,
                };
                self.go(to, Cause::Poke(p), None);
            }
        }
    }

    fn go(&mut self, to: State, cause: Cause, x: Option<f32>) {
        self.transitions.push(Transition {
            tod: self.tod(),
            from: Some(self.state),
            to,
            cause,
            x,
        });
        self.state = to;
        self.changed_at = self.t;
        self.until = self.t + self.duration(to);
        self.night_sleep = to == State::Sleep && self.night();
        self.dog.set_pose(to.pose());
        if !matches!(to, State::Wander | State::Walk) {
            self.dog.stop();
        }
        self.mark = None;
        self.bang = matches!(cause, Cause::Poke(Poke::Commit));
        self.heart = matches!(cause, Cause::Pet);
        if to == State::Wander {
            let (lo, hi) = self.x_range();
            let here = self.dog.x();
            let mut target = self.rng.range(f64::from(lo), f64::from(hi)) as f32;
            if (target - here).abs() < 8.0 {
                target = if here - lo > hi - here { lo } else { hi };
            }
            self.dog.walk_to(target);
        }
    }

    /// Pace back and forth (the forced `walk` state).
    fn pace(&mut self) {
        let (lo, hi) = self.x_range();
        let x = self.dog.x();
        self.dog.walk_to(if x - lo > hi - x { lo } else { hi });
    }

    fn duration(&mut self, state: State) -> f64 {
        let night = self.night();
        let r = &mut self.rng;
        match state {
            State::Idle if night => r.range(2.0, 4.0),
            State::Idle => r.range(3.0, 9.0),
            State::Sit if night => r.range(2.0, 4.0),
            State::Sit => r.range(6.0, 16.0),
            State::Lie if night => r.range(3.0, 6.0),
            State::Lie => r.range(8.0, 20.0),
            State::Sleep if night => f64::INFINITY,
            State::Sleep => r.range(20.0, 60.0),
            State::Happy | State::Sad | State::Stretch => 3.0,
            State::Curious => 1.5,
            State::Thinking => 60.0,
            // Walking states end on arrival.
            State::Wander | State::Walk => f64::INFINITY,
        }
    }

    fn next(&mut self, night: bool) {
        let r = self.rng.unit();
        let (to, cause) = match (self.state, night) {
            (State::Idle | State::Wander | State::Sit | State::Curious, true) => {
                (State::Lie, Cause::Night)
            }
            (State::Lie, true) => (State::Sleep, Cause::Night),
            (State::Happy | State::Sad | State::Thinking | State::Stretch, _) => {
                (State::Idle, Cause::Done)
            }
            (State::Curious, false) => (State::Idle, Cause::Done),
            (State::Idle, false) => match r {
                r if r < 0.40 => (State::Wander, Cause::Timer),
                r if r < 0.65 => (State::Sit, Cause::Timer),
                r if r < 0.80 => (State::Lie, Cause::Timer),
                _ => {
                    // Stay idle and look the other way.
                    let face = -self.dog.cur.face.signum();
                    self.dog.look(face);
                    self.until = self.t + self.duration(State::Idle);
                    return;
                }
            },
            (State::Sit, false) => {
                if r < 0.5 { (State::Idle, Cause::Timer) } else { (State::Lie, Cause::Timer) }
            }
            (State::Lie, false) => match r {
                r if r < 0.45 => (State::Sleep, Cause::Timer),
                r if r < 0.65 => (State::Sit, Cause::Timer),
                _ => (State::Idle, Cause::Timer),
            },
            (State::Sleep, false) => (State::Idle, Cause::Timer),
            (State::Sleep, true) | (State::Walk | State::Wander, _) => {
                self.until = f64::INFINITY;
                return;
            }
        };
        self.go(to, cause, None);
    }

    fn update_mark(&mut self) {
        let since = (self.t - self.changed_at) as f32;
        self.mark = match self.state {
            State::Happy if self.heart => Some(Mark::Heart((since / 2.5).min(1.0))),
            State::Happy if self.bang && since < 1.5 => Some(Mark::Bang),
            State::Sleep => {
                let cycle = (self.t % 4.0) as f32;
                (cycle < 3.0).then(|| Mark::Zzz(cycle / 3.0))
            }
            _ => None,
        };
    }
}
