//! `idlefx-companion launch`: the dog's own window, as an isolated user unit.
//!
//! Runs foot (or `$IDLEFX_TERMINAL`) with the shipped transparent config,
//! under `systemd-run --user` with a 10% CPU quota and a 64 MB memory cap.
//! A second launch while the unit runs does nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const UNIT: &str = "idlefx-companion";
pub const APP_ID: &str = "idlefx-companion";
pub const DEFAULT_WINDOW: &str = "64x16";
const FOOT_INI: &str = include_str!("../contrib/foot.ini");
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

fn config_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join("idlefx-companion-foot.ini")
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
        "MemoryMax=64M",
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
    args.push(format!("--config={}", config.display()));
    args.push(format!("--app-id={APP_ID}"));
    args.push(format!("--window-size-chars={window}"));
    args.push(exe.display().to_string());
    args
}

/// Run `launch`; returns the process exit code.
pub fn launch(window: &str, dry_run: bool) -> i32 {
    if !valid_window(window) {
        eprintln!("idlefx-companion: --window wants COLSxROWS, like {DEFAULT_WINDOW}");
        return 2;
    }
    let name = std::env::var("IDLEFX_TERMINAL").unwrap_or_else(|_| "foot".into());
    let terminal = find_in_path(&name);
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("idlefx-companion"));
    let config = config_path();
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
    if let Err(e) = std::fs::write(&config, FOOT_INI) {
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
