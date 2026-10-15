//! The event socket: `idlefx-companion poke <event>` talks to the running dog.
//!
//! The companion listens on `$XDG_RUNTIME_DIR/idlefx-companion.sock` from a
//! thread that blocks in `accept` (no polling) and wakes the frame loop
//! through the core `Waker`. Unknown event names are dropped there.

use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

use idlefx_core::Waker;

use crate::brain::Poke;

pub fn path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // SAFETY: getuid has no preconditions.
            let uid = unsafe { libc::getuid() };
            PathBuf::from(format!("/tmp/idlefx-companion-{uid}"))
        });
    dir.join("idlefx-companion.sock")
}

/// The bound socket; removes its file when dropped.
pub struct Listener {
    path: PathBuf,
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Start listening. `None` when another companion already answers on the
/// socket, or the socket cannot be bound. A stale file left by a crash is
/// replaced.
pub fn listen(waker: Waker<Poke>) -> Option<Listener> {
    let path = path();
    if path.exists() {
        if UnixStream::connect(&path).is_ok() {
            return None;
        }
        let _ = std::fs::remove_file(&path);
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let listener = UnixListener::bind(&path).ok()?;
    std::thread::Builder::new()
        .name("idlefx-socket".into())
        .spawn(move || {
            let mut buf = [0u8; 64];
            for conn in listener.incoming() {
                let Ok(mut conn) = conn else { continue };
                let _ = conn.set_read_timeout(Some(Duration::from_millis(200)));
                let mut len = 0;
                while len < buf.len() {
                    match conn.read(&mut buf[len..]) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => len += n,
                    }
                    if buf[..len].contains(&b'\n') {
                        break;
                    }
                }
                let text = std::str::from_utf8(&buf[..len]).unwrap_or("");
                let name = text.lines().next().unwrap_or("").trim();
                if let Some(poke) = Poke::parse(name)
                    && !waker.wake(poke)
                {
                    break;
                }
            }
        })
        .ok()?;
    Some(Listener { path })
}

/// Send one event to the running companion. Silent when none is running, so
/// hooks never fail because of it.
pub fn poke(name: &str) {
    if let Ok(mut stream) = UnixStream::connect(path()) {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
        let _ = writeln!(stream, "{name}");
    }
}
