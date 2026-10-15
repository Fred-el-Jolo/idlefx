//! Terminal session: set the terminal up, and always put it back.
//!
//! Entering switches to raw mode and the alternate screen and hides the
//! cursor; optionally it turns on SGR mouse reporting and the Kitty keyboard
//! protocol. Leaving — on drop, or from the panic hook before the panic
//! message is printed — undoes all of it in reverse order.

use std::io::{self, Stdout, Write};
use std::sync::Once;
use std::sync::atomic::{AtomicBool, Ordering};

use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::terminal::{
    self, BeginSynchronizedUpdate, EndSynchronizedUpdate, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::crossterm::{cursor, execute, queue};
use ratatui::Frame;

/// What to turn on besides raw mode and the alternate screen.
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionOptions {
    /// SGR mouse reporting (clicks, drags, motion).
    pub mouse: bool,
    /// Kitty keyboard protocol with key release events, when supported.
    pub keyboard_enhancement: bool,
}

static ACTIVE: AtomicBool = AtomicBool::new(false);
static MOUSE: AtomicBool = AtomicBool::new(false);
static KEYBOARD: AtomicBool = AtomicBool::new(false);
static HOOK: Once = Once::new();

/// The terminal, set up for full-screen drawing. Restores it when dropped.
pub struct Session {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Session {
    pub fn enter(options: SessionOptions) -> io::Result<Self> {
        HOOK.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                restore();
                previous(info);
            }));
        });

        terminal::enable_raw_mode()?;
        ACTIVE.store(true, Ordering::SeqCst);
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen, cursor::Hide)?;
        if options.mouse {
            execute!(out, EnableMouseCapture)?;
            MOUSE.store(true, Ordering::SeqCst);
        }
        if options.keyboard_enhancement && terminal::supports_keyboard_enhancement()? {
            execute!(
                out,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                        | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                )
            )?;
            KEYBOARD.store(true, Ordering::SeqCst);
        }
        let terminal = Terminal::new(CrosstermBackend::new(out))?;
        Ok(Self { terminal })
    }

    /// Draw one frame, bracketed by synchronized-output begin and end.
    pub fn draw(&mut self, render: impl FnOnce(&mut Frame)) -> io::Result<()> {
        queue!(self.terminal.backend_mut(), BeginSynchronizedUpdate)?;
        self.terminal.draw(render)?;
        execute!(self.terminal.backend_mut(), EndSynchronizedUpdate)?;
        Ok(())
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        restore();
    }
}

/// Undo everything [`Session::enter`] did. Safe to call more than once.
pub fn restore() {
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    let mut out = io::stdout();
    if KEYBOARD.swap(false, Ordering::SeqCst) {
        let _ = queue!(out, PopKeyboardEnhancementFlags);
    }
    if MOUSE.swap(false, Ordering::SeqCst) {
        let _ = queue!(out, DisableMouseCapture);
    }
    let _ = queue!(out, LeaveAlternateScreen, cursor::Show);
    let _ = out.flush();
    let _ = terminal::disable_raw_mode();
}
