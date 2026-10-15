//! `idlefx-companion launch`: the dog's own window, as an isolated user unit.
//!
//! Runs kitty (or `$IDLEFX_TERMINAL`: kitty or foot) with the shipped
//! transparent config for that terminal,
//! under `systemd-run --user` with a 10% CPU quota and a memory cap (64 MB for
//! foot, 128 MB for kitty, which idles around 85 MB).
//! A second launch while the unit runs does nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const UNIT: &str = "idlefx-companion";
pub const APP_ID: &str = "idlefx-companion";
pub const DEFAULT_WINDOW: &str = "64x16";
pub const DEFAULT_TERMINAL: &str = "kitty";
const FOOT_INI: &str = include_str!("../contrib/foot.ini");
const KITTY_CONF: &str = include_str!("../contrib/kitty.conf");
/// Variables the window needs, copied into the unit when they are set.
const PASS_ENV: [&str; 5] = ["WAYLAND_DISPLAY", "DISPLAY", "XDG_RUNTIME_DIR", "XDG_SESSION_TYPE", "LANG"];

/// Check `COLSxROWS`.
pub fn valid_window(window: &str) -> bool {
    let mut parts = window.split('x');
    let ok = |p: Option<&str>| p.and_then(|p| p.parse::<u16>().ok()).is_some_and(|n| n > 0);
    ok(parts.next()) && ok(parts.next()) && parts.next().is_none()
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    if name.contains('/') {
        return Some(PathBuf::from(name));
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

/// The terminals `launch` knows the flags of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terminal {
    Foot,
    Kitty,
}

impl Terminal {
    /// Pick by binary name; anything that is not kitty gets foot's flags.
    pub fn of(path: &Path) -> Self {
        match path.file_name().and_then(|n| n.to_str()) {
            Some("kitty") => Self::Kitty,
            _ => Self::Foot,
        }
    }

    fn memory_max(self) -> &'static str {
        match self {
            Self::Foot => "MemoryMax=64M",
            Self::Kitty => "MemoryMax=128M",
        }
    }

    fn config(self) -> (&'static str, &'static str) {
        match self {
            Self::Foot => ("idlefx-companion-foot.ini", FOOT_INI),
            Self::Kitty => ("idlefx-companion-kitty.conf", KITTY_CONF),
        }
    }

    /// Flags for the config file, the app id and the window size in cells.
    fn args(self, window: &str, config: &Path) -> Vec<String> {
        match self {
            Self::Foot => vec![
                format!("--config={}", config.display()),
                format!("--app-id={APP_ID}"),
                format!("--window-size-chars={window}"),
            ],
            Self::Kitty => {
                let (cols, rows) = window.split_once('x').unwrap_or((window, window));
                vec![
                    format!("--config={}", config.display()),
                    format!("--class={APP_ID}"),
                    "-o".into(),
                    format!("initial_window_width={cols}c"),
                    "-o".into(),
                    format!("initial_window_height={rows}c"),
                ]
            }
        }
    }
}

fn config_path(file: &str) -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(file)
}

/// The full `systemd-run` command line.
pub fn command(window: &str, terminal: &Path, exe: &Path, config: &Path) -> Vec<String> {
    let mut args: Vec<String> = [
        "systemd-run",
        "--user",
        "--quiet",
        &format!("--unit={UNIT}"),
        "--collect",
        "-p",
        "CPUQuota=10%",
        "-p",
        Terminal::of(terminal).memory_max(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for var in PASS_ENV {
        if std::env::var_os(var).is_some() {
            args.push(format!("--setenv={var}"));
        }
    }
    args.push(terminal.display().to_string());
    args.extend(Terminal::of(terminal).args(window, config));
    args.push(exe.display().to_string());
    args
}

/// Run `launch`; returns the process exit code.
pub fn launch(window: &str, dry_run: bool) -> i32 {
    if !valid_window(window) {
        eprintln!("idlefx-companion: --window wants COLSxROWS, like {DEFAULT_WINDOW}");
        return 2;
    }
    let name = std::env::var("IDLEFX_TERMINAL").unwrap_or_else(|_| DEFAULT_TERMINAL.into());
    let terminal = find_in_path(&name);
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("idlefx-companion"));
    let (file, contents) = Terminal::of(Path::new(&name)).config();
    let config = config_path(file);
    let args = command(
        window,
        terminal.as_deref().unwrap_or(Path::new(&name)),
        &exe,
        &config,
    );
    if dry_run {
        println!("{}", args.join(" "));
        return 0;
    }
    let Some(_) = terminal else {
        eprintln!("idlefx-companion: terminal `{name}` not found (set IDLEFX_TERMINAL)");
        return 1;
    };
    let running = Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", UNIT])
        .status()
        .is_ok_and(|s| s.success());
    if running {
        return 0;
    }
    if let Err(e) = std::fs::write(&config, contents) {
        eprintln!("idlefx-companion: cannot write {}: {e}", config.display());
        return 1;
    }
    match Command::new(&args[0]).args(&args[1..]).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("idlefx-companion: cannot run systemd-run: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tail(terminal: &str) -> Vec<String> {
        let args = command("80x20", Path::new(terminal), Path::new("/bin/dog"), Path::new("/run/c"));
        let at = args.iter().position(|a| a == terminal).unwrap();
        args[at + 1..].to_vec()
    }

    #[test]
    fn foot_flags() {
        assert_eq!(
            tail("/usr/bin/foot"),
            ["--config=/run/c", "--app-id=idlefx-companion", "--window-size-chars=80x20", "/bin/dog"]
        );
    }

    #[test]
    fn kitty_flags() {
        assert_eq!(
            tail("/usr/bin/kitty"),
            [
                "--config=/run/c",
                "--class=idlefx-companion",
                "-o",
                "initial_window_width=80c",
                "-o",
                "initial_window_height=20c",
                "/bin/dog",
            ]
        );
    }
}
