//! The procedural dog: parts on a light skeleton, moved by springs.
//!
//! A [`Pose`] is a set of targets (body height and tilt, head position, ear
//! lift, tail angle and wag, leg folding). Critically damped springs pull
//! every value toward the current pose's targets, so a change of pose blends
//! instead of jumping. On top: a walk cycle whose leg phase follows the
//! ground actually covered (feet do not slide), tail wag, a hop, breathing
//! and blinking. Lengths are in pixels; `u` points forward (the way the dog
//! faces), `v` up from the ground.

use std::f32::consts::{PI, TAU};

use idlefx_core::{Canvas, Rgb};

const COAT: Rgb = Rgb(214, 160, 98);
const COAT_FAR: Rgb = Rgb(178, 126, 72);
const DARK: Rgb = Rgb(118, 76, 44);
const CREAM: Rgb = Rgb(246, 232, 208);
const INK: Rgb = Rgb(30, 22, 20);
const HEART: Rgb = Rgb(232, 72, 96);
const ZZZ: Rgb = Rgb(200, 215, 240);
const BANG: Rgb = Rgb(250, 210, 80);

/// Distance covered by one full walk cycle.
const STRIDE: f32 = 7.0;
/// Walking speed, pixels per second.
pub const WALK_SPEED: f32 = 18.0;
/// Half the dog's length: how close its center may come to an edge.
pub const HALF_LENGTH: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose {
    Stand,
    Walk,
    Sit,
    Lie,
    Sleep,
    Happy,
    Sad,
    Curious,
    /// Sitting, head tilted, ears up.
    Think,
    /// Play bow: front down, rear up.
    Stretch,
}

/// The eight poses reviewed with the snapshot option.
#[cfg(test)]
pub const POSES: [Pose; 8] = [
    Pose::Stand,
    Pose::Walk,
    Pose::Sit,
    Pose::Lie,
    Pose::Sleep,
    Pose::Happy,
    Pose::Sad,
    Pose::Curious,
];

impl Pose {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "stand" => Pose::Stand,
            "walk" => Pose::Walk,
            "sit" => Pose::Sit,
            "lie" => Pose::Lie,
            "sleep" => Pose::Sleep,
            "happy" => Pose::Happy,
            "sad" => Pose::Sad,
            "curious" => Pose::Curious,
            "think" => Pose::Think,
            "stretch" => Pose::Stretch,
            _ => return None,
        })
    }
}

/// Spring targets of a pose.
#[derive(Clone, Copy)]
struct Targets {
    body_h: f32,
    body_ry: f32,
    tilt: f32,
    head_u: f32,
    head_v: f32,
    head_tilt: f32,
    ear: f32,
    tail: f32,
    wag_amp: f32,
    wag_freq: f32,
    fold_front: f32,
    fold_rear: f32,
    hop: f32,
    eyes: f32,
}

const STAND: Targets = Targets {
    body_h: 10.0,
    body_ry: 4.5,
    tilt: 0.0,
    head_u: 10.0,
    head_v: 15.5,
    head_tilt: 0.0,
    ear: 0.4,
    tail: 0.6,
    wag_amp: 0.1,
    wag_freq: 1.2,
    fold_front: 0.0,
    fold_rear: 0.0,
    hop: 0.0,
    eyes: 1.0,
};

fn targets(pose: Pose) -> Targets {
    match pose {
        Pose::Stand => STAND,
        Pose::Walk => Targets {
            head_u: 10.5,
            head_v: 15.0,
            ear: 0.5,
            tail: 0.7,
            wag_amp: 0.2,
            wag_freq: 1.6,
            ..STAND
        },
        // Upright on its haunches: the rump rests on the ground, the hind
        // paws tucked forward under it, the tail lying on the floor.
        Pose::Sit => Targets {
            body_h: 7.0,
            tilt: 0.75,
            head_u: 7.2,
            head_v: 16.8,
            tail: -0.12,
            wag_amp: 0.05,
            fold_rear: 1.0,
            ..STAND
        },
        Pose::Lie => Targets {
            body_h: 4.6,
            body_ry: 3.9,
            head_u: 10.5,
            head_v: 9.0,
            ear: 0.25,
            tail: 0.0,
            wag_amp: 0.04,
            fold_front: 1.0,
            fold_rear: 1.0,
            ..STAND
        },
        Pose::Sleep => Targets {
            body_h: 4.2,
            body_ry: 3.8,
            head_u: 8.0,
            head_v: 5.6,
            ear: 0.0,
            tail: -2.6,
            wag_amp: 0.0,
            fold_front: 1.0,
            fold_rear: 1.0,
            eyes: 0.0,
            ..STAND
        },
        Pose::Happy => Targets {
            head_u: 10.5,
            head_v: 16.0,
            ear: 0.9,
            tail: 1.0,
            wag_amp: 0.45,
            wag_freq: 3.0,
            hop: 2.0,
            ..STAND
        },
        Pose::Sad => Targets {
            body_h: 9.4,
            head_u: 10.5,
            head_v: 12.5,
            ear: 0.0,
            tail: -0.6,
            wag_amp: 0.0,
            ..STAND
        },
        Pose::Curious => Targets {
            head_v: 16.0,
            head_tilt: 0.35,
            ear: 1.0,
            tail: 0.8,
            wag_amp: 0.08,
            ..STAND
        },
        Pose::Think => Targets {
            head_tilt: 0.3,
            ear: 1.0,
            wag_amp: 0.06,
            ..targets(Pose::Sit)
        },
        Pose::Stretch => Targets {
            body_h: 8.5,
            tilt: -0.35,
            head_u: 12.0,
            head_v: 9.5,
            ear: 0.6,
            tail: 1.1,
            wag_amp: 0.15,
            fold_front: 0.7,
            ..STAND
        },
    }
}

/// Everything that moves, as plain numbers, so two states can be blended.
#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub x: f32,
    pub face: f32,
    pub speed: f32,
    phase: f32,
    walk_w: f32,
    body_h: f32,
    body_ry: f32,
    tilt: f32,
    head_u: f32,
    head_v: f32,
    head_tilt: f32,
    ear: f32,
    tail: f32,
    wag_amp: f32,
    wag_freq: f32,
    wag_phase: f32,
    fold_front: f32,
    fold_rear: f32,
    hop: f32,
    hop_phase: f32,
    eyes: f32,
    breath: f32,
}

impl State {
    /// Linear blend from `a` to `b`.
    pub fn lerp(a: &State, b: &State, t: f32) -> State {
        let m = |p: f32, q: f32| p + (q - p) * t;
        State {
            x: m(a.x, b.x),
            face: m(a.face, b.face),
            speed: m(a.speed, b.speed),
            phase: if (b.phase - a.phase).abs() < 0.5 { m(a.phase, b.phase) } else { b.phase },
            walk_w: m(a.walk_w, b.walk_w),
            body_h: m(a.body_h, b.body_h),
            body_ry: m(a.body_ry, b.body_ry),
            tilt: m(a.tilt, b.tilt),
            head_u: m(a.head_u, b.head_u),
            head_v: m(a.head_v, b.head_v),
            head_tilt: m(a.head_tilt, b.head_tilt),
            ear: m(a.ear, b.ear),
            tail: m(a.tail, b.tail),
            wag_amp: m(a.wag_amp, b.wag_amp),
            wag_freq: m(a.wag_freq, b.wag_freq),
            wag_phase: m(a.wag_phase, b.wag_phase),
            fold_front: m(a.fold_front, b.fold_front),
            fold_rear: m(a.fold_rear, b.fold_rear),
            hop: m(a.hop, b.hop),
            hop_phase: m(a.hop_phase, b.hop_phase),
            eyes: m(a.eyes, b.eyes),
            breath: m(a.breath, b.breath),
        }
    }
}

/// Spring velocities, one per sprung value of [`State`].
#[derive(Clone, Copy, Debug, Default)]
struct Velocity {
    face: f32,
    speed: f32,
    walk_w: f32,
    body_h: f32,
    body_ry: f32,
    tilt: f32,
    head_u: f32,
    head_v: f32,
    head_tilt: f32,
    ear: f32,
    tail: f32,
    wag_amp: f32,
    wag_freq: f32,
    fold_front: f32,
    fold_rear: f32,
    hop: f32,
    eyes: f32,
}

/// Critically damped spring step (semi-implicit Euler).
fn spring(x: &mut f32, v: &mut f32, target: f32, omega: f32, dt: f32) {
    *v += (omega * omega * (target - *x) - 2.0 * omega * *v) * dt;
    *x += *v * dt;
}

/// The dog: its current and previous state, and where it is heading.
#[derive(Clone, Debug)]
pub struct Dog {
    pub cur: State,
    pub prev: State,
    vel: Velocity,
    pose: Pose,
    goal: Option<f32>,
    face_goal: f32,
    blink: f32,
    time: f32,
}

/// Pose spring stiffness (rad/s): slow enough to read as weight.
const OMEGA: f32 = 7.0;

impl Dog {
    /// A dog standing still at `x`, facing right, settled in `pose`.
    pub fn new(x: f32, pose: Pose) -> Self {
        let mut dog = Dog {
            cur: State {
                x,
                face: 1.0,
                ..State::default()
            },
            prev: State::default(),
            vel: Velocity::default(),
            pose,
            goal: None,
            face_goal: 1.0,
            blink: 0.0,
            time: 0.0,
        };
        dog.settle();
        dog
    }

    /// Jump every sprung value to the current pose's targets.
    pub fn settle(&mut self) {
        let t = targets(self.pose);
        let s = &mut self.cur;
        s.body_h = t.body_h;
        s.body_ry = t.body_ry;
        s.tilt = t.tilt;
        s.head_u = t.head_u;
        s.head_v = t.head_v;
        s.head_tilt = t.head_tilt;
        s.ear = t.ear;
        s.tail = t.tail;
        s.wag_amp = t.wag_amp;
        s.wag_freq = t.wag_freq;
        s.fold_front = t.fold_front;
        s.fold_rear = t.fold_rear;
        s.hop = t.hop;
        s.eyes = t.eyes;
        self.vel = Velocity::default();
        self.prev = self.cur;
    }

    pub fn set_pose(&mut self, pose: Pose) {
        self.pose = pose;
    }

    pub fn x(&self) -> f32 {
        self.cur.x
    }

    /// Walk to `x`, easing in and out.
    pub fn walk_to(&mut self, x: f32) {
        self.goal = Some(x);
    }

    pub fn stop(&mut self) {
        self.goal = None;
    }

    pub fn walking(&self) -> bool {
        self.goal.is_some() || self.cur.speed.abs() > 0.5
    }

    /// Face a direction (`1` right, `-1` left) without walking.
    pub fn look(&mut self, dir: f32) {
        self.face_goal = dir.signum();
    }

    /// True while any spring is still visibly moving.
    pub fn moving(&self) -> bool {
        let v = &self.vel;
        let fast = |x: f32| x.abs() > 0.4;
        self.walking()
            || [
                v.face, v.body_h, v.tilt, v.head_u, v.head_v, v.ear, v.tail, v.fold_front,
                v.fold_rear, v.hop, v.eyes,
            ]
            .into_iter()
            .any(fast)
            || self.cur.hop > 0.1
            || self.cur.wag_amp > 0.3
    }

    /// Advance by `dt` seconds. Returns true on the step the dog arrives at
    /// its walking goal.
    pub fn update(&mut self, dt: f32) -> bool {
        self.prev = self.cur;
        self.time += dt;
        let t = targets(self.pose);
        let (s, v) = (&mut self.cur, &mut self.vel);

        // Movement: speed eases toward what the remaining distance allows.
        let mut arrived = false;
        let desired = match self.goal {
            Some(g) => {
                let d = g - s.x;
                if d.abs() > 0.5 {
                    self.face_goal = d.signum();
                }
                if d.abs() < 0.3 && s.speed.abs() < 1.0 {
                    self.goal = None;
                    arrived = true;
                    0.0
                } else {
                    (d * 2.5).clamp(-WALK_SPEED, WALK_SPEED)
                }
            }
            None => 0.0,
        };
        spring(&mut s.speed, &mut v.speed, desired, 6.0, dt);
        let dx = s.speed * dt;
        s.x += dx;
        s.phase = (s.phase + dx.abs() / STRIDE).fract();
        let walk_target = if self.goal.is_some() || s.speed.abs() > 0.5 { 1.0 } else { 0.0 };
        spring(&mut s.walk_w, &mut v.walk_w, walk_target, 8.0, dt);
        spring(&mut s.face, &mut v.face, self.face_goal, 9.0, dt);

        spring(&mut s.body_h, &mut v.body_h, t.body_h, OMEGA, dt);
        spring(&mut s.body_ry, &mut v.body_ry, t.body_ry, OMEGA, dt);
        spring(&mut s.tilt, &mut v.tilt, t.tilt, OMEGA, dt);
        spring(&mut s.head_u, &mut v.head_u, t.head_u, OMEGA, dt);
        spring(&mut s.head_v, &mut v.head_v, t.head_v, OMEGA, dt);
        spring(&mut s.head_tilt, &mut v.head_tilt, t.head_tilt, OMEGA, dt);
        // Ears are a little slower than the head, so they lag behind it.
        spring(&mut s.ear, &mut v.ear, t.ear, 5.0, dt);
        spring(&mut s.tail, &mut v.tail, t.tail, 5.0, dt);
        spring(&mut s.wag_amp, &mut v.wag_amp, t.wag_amp, 4.0, dt);
        spring(&mut s.wag_freq, &mut v.wag_freq, t.wag_freq, 4.0, dt);
        spring(&mut s.fold_front, &mut v.fold_front, t.fold_front, OMEGA, dt);
        spring(&mut s.fold_rear, &mut v.fold_rear, t.fold_rear, OMEGA, dt);
        spring(&mut s.hop, &mut v.hop, t.hop, 5.0, dt);
        spring(&mut s.eyes, &mut v.eyes, t.eyes, 6.0, dt);
        s.wag_phase = (s.wag_phase + TAU * s.wag_freq * dt) % TAU;
        s.hop_phase = (s.hop_phase + TAU * 1.5 * dt) % TAU;
        s.breath = (self.time * TAU / 4.0).sin();

        // Blink every few seconds while awake (deterministic in time).
        let cycle = self.time % 4.7;
        self.blink = if cycle < 0.2 { 1.0 } else { 0.0 };
        arrived
    }

    /// The state to draw at `alpha` between the last two steps.
    pub fn at(&self, alpha: f32) -> State {
        State::lerp(&self.prev, &self.cur, alpha.clamp(0.0, 1.0))
    }

    /// Whether the eyes are shut by a blink right now.
    pub fn blinking(&self) -> bool {
        self.blink > 0.5
    }
}

/// World positions of the dog's parts, `y` pointing down.
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub body: (f32, f32),
    body_r: (f32, f32),
    body_angle: f32,
    pub chest: (f32, f32),
    pub head: (f32, f32),
    head_angle: f32,
    pub snout: (f32, f32),
    pub nose: (f32, f32),
    pub eye: (f32, f32),
    pub ear_base: (f32, f32),
    pub ear_tip: (f32, f32),
    pub tail_base: (f32, f32),
    pub tail_tip: (f32, f32),
    tail_mid: (f32, f32),
    /// Near front, far front, near rear, far rear.
    pub hips: [(f32, f32); 4],
    pub feet: [(f32, f32); 4],
    haunch: Option<(f32, f32, f32)>,
    squash: f32,
}

impl Geometry {
    /// Lay the dog out from `s`, standing on `ground`.
    pub fn of(s: &State, ground: f32) -> Self {
        let f = s.face;
        let squash = f.abs().clamp(0.15, 1.0);
        let hop = s.hop * s.hop_phase.sin().abs();
        let bob = 0.4 * (s.phase * 2.0 * TAU).sin() * s.walk_w;
        let lift = hop + bob;
        let world = |u: f32, v: f32| (s.x + f * u, ground - v - lift);
        // Feet hop with the dog but do not bob with the walking body.
        let world_foot = |u: f32, v: f32| (s.x + f * u, ground - v - hop);
        let body_v = s.body_h;
        // A point of the body frame, rotated by the tilt around the center.
        let (sin_t, cos_t) = s.tilt.sin_cos();
        let body_pt = |u: f32, v: f32| {
            let (ru, rv) = (u * cos_t - v * sin_t, u * sin_t + v * cos_t);
            (ru, body_v + rv)
        };

        let chest = body_pt(5.5, -1.2);
        let tail_base = body_pt(-8.0, 1.5);
        let theta = s.tail + s.wag_amp * s.wag_phase.sin();
        let tail_dir = (-theta.cos(), theta.sin());
        let tail_tip = (tail_base.0 + 6.5 * tail_dir.0, tail_base.1 + 6.5 * tail_dir.1);
        let tail_mid = (tail_base.0 + 4.3 * tail_dir.0, tail_base.1 + 4.3 * tail_dir.1);

        let head = (s.head_u, s.head_v);
        let (hs, hc) = s.head_tilt.sin_cos();
        let head_pt = |u: f32, v: f32| (head.0 + u * hc + v * hs, head.1 - u * hs + v * hc);
        let snout = head_pt(3.6, -1.2);
        let nose = head_pt(5.5, -0.7);
        let eye = head_pt(1.4, 1.0);
        let ear_base = head_pt(-1.6, 2.8);
        let droop = (-2.0, -4.6);
        let perk = (0.6, 2.6);
        let ear_tip = (
            ear_base.0 + droop.0 + (perk.0 - droop.0) * s.ear,
            ear_base.1 + droop.1 + (perk.1 - droop.1) * s.ear,
        );

        // Legs: hips in the body frame; feet from standing, folding and the
        // walk cycle. Phases pair them diagonally (trot).
        let hip_local = [(5.5, -2.0), (6.3, -2.0), (-5.5, -1.5), (-4.7, -1.5)];
        let offsets = [0.0, 0.5, 0.5, 0.0];
        let mut hips = [(0.0, 0.0); 4];
        let mut feet = [(0.0, 0.0); 4];
        for i in 0..4 {
            let hip = body_pt(hip_local[i].0, hip_local[i].1);
            let front = i < 2;
            let fold = if front { s.fold_front } else { s.fold_rear };
            let fold_reach = if front { 4.5 } else { 4.0 };
            let p = (s.phase + offsets[i]).fract();
            let (walk_u, walk_lift) = if p < 0.5 {
                (STRIDE * (0.25 - p), 0.0)
            } else {
                (STRIDE * (p - 0.75), 1.6 * ((p - 0.5) * TAU).sin())
            };
            let w = s.walk_w * (1.0 - fold);
            let foot_u = hip.0 + 0.2 + (fold_reach - 0.2) * fold + w * walk_u;
            let foot_v = 0.9 * fold + w * walk_lift;
            hips[i] = world(hip.0, hip.1);
            feet[i] = world_foot(foot_u, foot_v);
        }
        let haunch = (s.fold_rear > 0.3).then(|| {
            let h = body_pt(-4.0, -1.5);
            let (x, y) = world(h.0, h.1);
            (x, y, s.fold_rear)
        });

        let breath = 0.25 * s.breath;
        Geometry {
            body: world(0.0, body_v),
            body_r: (8.5 * squash, s.body_ry + breath),
            body_angle: -s.tilt * f.signum(),
            chest: world(chest.0, chest.1),
            head: world(head.0, head.1),
            head_angle: -s.head_tilt * f.signum(),
            snout: world(snout.0, snout.1),
            nose: world(nose.0, nose.1),
            eye: world(eye.0, eye.1),
            ear_base: world(ear_base.0, ear_base.1),
            ear_tip: world(ear_tip.0, ear_tip.1),
            tail_base: world(tail_base.0, tail_base.1),
            tail_tip: world(tail_tip.0, tail_tip.1),
            tail_mid: world(tail_mid.0, tail_mid.1),
            hips,
            feet,
            haunch,
            squash,
        }
    }

    /// The points the continuity tests follow.
    #[cfg(test)]
    pub fn key_points(&self) -> [(f32, f32); 12] {
        [
            self.body,
            self.chest,
            self.head,
            self.nose,
            self.ear_tip,
            self.tail_tip,
            self.tail_base,
            self.feet[0],
            self.feet[1],
            self.feet[2],
            self.feet[3],
            self.eye,
        ]
    }

    /// Draw the dog: far parts first, near parts last.
    pub fn draw(&self, canvas: &mut Canvas, eyes_open: bool) {
        let sq = self.squash;
        // Tail, then the far legs, behind the body.
        canvas.capsule(self.tail_base.0, self.tail_base.1, self.tail_mid.0, self.tail_mid.1, 1.1, COAT);
        canvas.capsule(self.tail_mid.0, self.tail_mid.1, self.tail_tip.0, self.tail_tip.1, 1.0, DARK);
        for i in [1, 3] {
            leg(canvas, self.hips[i], self.feet[i], COAT_FAR);
        }
        canvas.ellipse(self.body.0, self.body.1, self.body_r.0, self.body_r.1, self.body_angle, COAT);
        if let Some((x, y, k)) = self.haunch {
            canvas.ellipse(x, y, 3.6 * sq, 2.4 + 0.6 * k, 0.0, COAT);
        }
        canvas.ellipse(self.chest.0, self.chest.1, 3.0 * sq, 3.0, 0.0, CREAM);
        for i in [0, 2] {
            leg(canvas, self.hips[i], self.feet[i], COAT);
        }
        canvas.ellipse(self.head.0, self.head.1, 4.2 * sq, 3.8, self.head_angle, COAT);
        canvas.ellipse(self.snout.0, self.snout.1, 2.6 * sq, 1.8, self.head_angle, CREAM);
        canvas.pixel(self.nose.0, self.nose.1, INK);
        if eyes_open {
            canvas.pixel(self.eye.0, self.eye.1, INK);
        } else {
            canvas.pixel(self.eye.0, self.eye.1 + 1.0, DARK);
        }
        canvas.capsule(self.ear_base.0, self.ear_base.1, self.ear_tip.0, self.ear_tip.1, 1.4, DARK);
    }
}

fn leg(canvas: &mut Canvas, hip: (f32, f32), foot: (f32, f32), color: Rgb) {
    canvas.capsule(hip.0, hip.1, foot.0, foot.1 - 0.6, 1.15, color);
}

/// A tiny pixel glyph, `#` opaque, drawn with its bottom-left at `(x, y)`.
fn glyph(canvas: &mut Canvas, rows: &[&str], x: f32, y: f32, color: Rgb) {
    let h = rows.len() as f32;
    for (j, row) in rows.iter().enumerate() {
        for (i, c) in row.chars().enumerate() {
            if c == '#' {
                canvas.pixel(x + i as f32, y - h + j as f32, color);
            }
        }
    }
}

/// Small marks drawn above the head.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mark {
    /// A heart rising, `t` in 0..1.
    Heart(f32),
    /// A `z` drifting up, `t` in 0..1.
    Zzz(f32),
    /// An exclamation mark.
    Bang,
}

pub fn draw_mark(canvas: &mut Canvas, geo: &Geometry, mark: Mark) {
    let (hx, hy) = geo.head;
    match mark {
        Mark::Heart(t) => glyph(
            canvas,
            &[".#.#.", "#####", ".###.", "..#.."],
            hx - 2.0,
            hy - 5.0 - 6.0 * t,
            HEART,
        ),
        Mark::Zzz(t) => glyph(
            canvas,
            &["####", "..#.", ".#..", "####"],
            hx + 1.0 + 2.0 * (t * PI).sin(),
            hy - 4.0 - 7.0 * t,
            ZZZ,
        ),
        Mark::Bang => glyph(canvas, &["#", "#", "#", ".", "#"], hx + 1.0, hy - 5.0, BANG),
    }
}

/// Draw `dog` at `alpha` on `canvas`, standing on its bottom edge.
pub fn draw(canvas: &mut Canvas, dog: &Dog, alpha: f32, mark: Option<Mark>) {
    let s = dog.at(alpha);
    let geo = Geometry::of(&s, canvas.height() as f32);
    let eyes_open = s.eyes > 0.5 && !dog.blinking();
    geo.draw(canvas, eyes_open);
    if let Some(m) = mark {
        draw_mark(canvas, &geo, m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;
    const W: usize = 64;
    const H: usize = 32;

    fn bounds_of(dog: &Dog) -> (usize, usize, usize, usize) {
        let mut c = Canvas::new(W, H);
        draw(&mut c, dog, 1.0, None);
        c.bounds().expect("the dog draws something")
    }

    #[test]
    fn poses_fit_canvas() {
        for pose in POSES.into_iter().chain([Pose::Think, Pose::Stretch]) {
            let dog = Dog::new(32.0, pose);
            let s = dog.at(1.0);
            // Unclipped geometry stays inside the canvas, hop included.
            let mut hopping = s;
            hopping.hop_phase = PI / 2.0;
            for st in [s, hopping] {
                let geo = Geometry::of(&st, H as f32);
                for (x, y) in geo.key_points() {
                    assert!(
                        (1.0..W as f32 - 1.0).contains(&x) && (1.0..H as f32 + 0.1).contains(&y),
                        "{pose:?}: point ({x}, {y}) leaves the canvas"
                    );
                }
            }
            let (x0, y0, x1, _) = bounds_of(&dog);
            assert!(x0 > 0 && x1 < W && y0 > 0, "{pose:?} touches the canvas edge");
        }
    }

    #[test]
    fn stand_bbox_size() {
        let (x0, y0, x1, y1) = bounds_of(&Dog::new(32.0, Pose::Stand));
        let (w, h) = (x1 - x0, y1 - y0);
        assert!((26..=34).contains(&w), "width {w}");
        assert!((16..=24).contains(&h), "height {h}");
    }

    /// Run `dog` for `secs`, sampling key points every 60fps frame, and
    /// return the largest move of any point between two frames.
    fn max_step(dog: &mut Dog, secs: f32) -> f32 {
        let mut worst: f32 = 0.0;
        let mut last = Geometry::of(&dog.cur, H as f32).key_points();
        let steps = (secs / DT) as usize;
        for i in 1..=steps {
            dog.update(DT);
            if i % 2 == 0 {
                let now = Geometry::of(&dog.cur, H as f32).key_points();
                for (a, b) in last.iter().zip(now.iter()) {
                    worst = worst.max(((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt());
                }
                last = now;
            }
        }
        worst
    }

    #[test]
    fn pose_transitions_continuous() {
        let all = [
            Pose::Stand,
            Pose::Walk,
            Pose::Sit,
            Pose::Lie,
            Pose::Sleep,
            Pose::Happy,
            Pose::Sad,
            Pose::Curious,
            Pose::Think,
            Pose::Stretch,
        ];
        for a in all {
            for b in all {
                let mut dog = Dog::new(32.0, a);
                if a == Pose::Walk {
                    dog.walk_to(200.0);
                    max_step(&mut dog, 2.0);
                }
                dog.set_pose(b);
                if b == Pose::Walk {
                    dog.walk_to(dog.x() + 60.0);
                } else {
                    dog.stop();
                }
                let worst = max_step(&mut dog, 3.0);
                assert!(worst <= 2.0, "{a:?} -> {b:?}: a part moved {worst:.2} px in one frame");
            }
        }
    }

    #[test]
    fn walk_no_foot_sliding() {
        let mut dog = Dog::new(20.0, Pose::Walk);
        dog.walk_to(400.0);
        // Reach a steady walk first.
        for _ in 0..240 {
            dog.update(DT);
        }
        let ground = H as f32;
        for leg in 0..4 {
            // Track each stance: the foot's x while it is on the ground.
            let mut probe = dog.clone();
            let mut worst: f32 = 0.0;
            let mut planted: Option<f32> = None;
            for _ in 0..600 {
                probe.update(DT);
                let geo = Geometry::of(&probe.cur, ground);
                let (fx, fy) = geo.feet[leg];
                let s = &probe.cur;
                let lifted = ground - fy - s.hop * s.hop_phase.sin().abs() > 0.01;
                match (lifted, planted) {
                    (false, None) => planted = Some(fx),
                    (false, Some(x0)) => worst = worst.max((fx - x0).abs()),
                    (true, _) => planted = None,
                }
            }
            assert!(worst < 1.0, "leg {leg}: a planted foot slid {worst:.2} px");
        }
    }

    #[test]
    fn turning_squashes_instead_of_flipping() {
        let mut dog = Dog::new(32.0, Pose::Stand);
        dog.look(-1.0);
        let mut seen_thin = false;
        for _ in 0..120 {
            dog.update(DT);
            if dog.cur.face.abs() < 0.5 {
                seen_thin = true;
            }
        }
        assert!(seen_thin);
        assert!(dog.cur.face < -0.9);
    }
}
