//! The live companion: the dog and its behaviour, driven by the core loop.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

use idlefx_core::{Activity, App, Canvas, Event, Flow};
use ratatui::crossterm::event::{KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};

use crate::brain::{Companion, Input, Poke, State, Transition};
use crate::dog;

pub struct CompanionApp {
    pub model: Companion,
    trace: Option<BufWriter<File>>,
    /// Transitions not yet written, with the time of the input behind them.
    pending: Vec<(Transition, Option<Instant>)>,
    line: String,
    /// The last drawn canvas size, and a canvas to hit-test clicks on.
    size: (usize, usize),
    scratch: Canvas,
}

impl CompanionApp {
    pub fn new(model: Companion, trace: Option<File>) -> Self {
        let mut app = CompanionApp {
            model,
            trace: trace.map(BufWriter::new),
            pending: Vec::with_capacity(8),
            line: String::with_capacity(96),
            size: (0, 0),
            scratch: Canvas::default(),
        };
        app.collect(None);
        app
    }

    fn collect(&mut self, at: Option<Instant>) {
        self.pending.extend(self.model.drain().map(|t| (t, at)));
    }

    /// Whether the pixel under the cell `(col, row)` belongs to the dog.
    fn hits_dog(&mut self, col: u16, row: u16) -> bool {
        let (w, h) = self.size;
        self.scratch.resize(w, h);
        self.scratch.clear();
        dog::draw(&mut self.scratch, &self.model.dog, 1.0, None);
        let (x, y) = (col as usize, row as usize * 2);
        self.scratch.get(x, y).is_some() || self.scratch.get(x, y + 1).is_some()
    }

    fn input(&mut self, input: Input, at: Instant) -> Flow {
        self.model.input(input);
        self.collect(Some(at));
        Flow::Redraw
    }
}

impl App for CompanionApp {
    type Event = Poke;

    fn update(&mut self, dt: f32) {
        self.model.step(dt);
        self.collect(None);
    }

    fn draw(&mut self, canvas: &mut Canvas, alpha: f32) {
        let size = (canvas.width(), canvas.height());
        if size != self.size {
            self.size = size;
            self.model.set_width(size.0 as f32);
        }
        dog::draw(canvas, &self.model.dog, alpha, self.model.mark());
    }

    fn activity(&self) -> Activity {
        if self.model.busy() {
            Activity::Active
        } else if self.model.state() == State::Sleep {
            Activity::Idle(2)
        } else {
            Activity::Idle(4)
        }
    }

    fn event(&mut self, event: Event<Poke>, at: Instant) -> Flow {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => Flow::Quit,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Flow::Quit,
                _ => Flow::Continue,
            },
            Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => {
                let input = if self.hits_dog(m.column, m.row) {
                    Input::Pet
                } else {
                    Input::Click(m.column as f32 + 0.5)
                };
                self.input(input, at)
            }
            Event::App(poke) => self.input(Input::Poke(poke), at),
            Event::Resize(..) => Flow::Redraw,
            _ => Flow::Continue,
        }
    }

    fn frame_done(&mut self, at: Instant) {
        let Some(out) = self.trace.as_mut() else {
            self.pending.clear();
            return;
        };
        for (t, input_at) in self.pending.drain(..) {
            self.line.clear();
            t.write(&mut self.line);
            if let Some(i) = input_at {
                use std::fmt::Write as _;
                let ms = (at - i).as_secs_f64() * 1000.0;
                let _ = write!(self.line, " latency_ms={ms:.1}");
            }
            let _ = writeln!(out, "{}", self.line);
        }
        let _ = out.flush();
    }
}
