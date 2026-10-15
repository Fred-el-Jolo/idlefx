//! `idlefx-companion launch`: the dog's own window, as an isolated user unit.
//!
//! Runs kitty with the shipped transparent `kitty.conf`, under
//! `systemd-run --user` with a 10% CPU quota and a 128 MB memory cap (kitty
//! idles around 85 MB). `$IDLEFX_TERMINAL` points it at another kitty binary.
//! A second launch while the unit runs does nothing.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const UNIT: &str = "idlefx-companion";
pub const APP_ID: &str = "idlefx-companion";
pub const DEFAULT_WINDOW: &str = "64x16";
const KITTY: &str = "kitty";
const KITTY_CONF: &str = include_str!("../contrib/kitty.conf");
const CONFIG_FILE: &str = "idlefx-companion-kitty.conf";
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
    dir.join(CONFIG_FILE)
}

/// The full `systemd-run` command line.
pub fn command(window: &str, kitty: &Path, exe: &Path, config: &Path) -> Vec<String> {
    let mut args: Vec<String> = [
        "systemd-run",
        "--user",
        "--quiet",
        &format!("--unit={UNIT}"),
        "--collect",
        "-p",
        "CPUQuota=10%",
        "-p",
        "MemoryMax=128M",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for var in PASS_ENV {
        if std::env::var_os(var).is_some() {
            args.push(format!("--setenv={var}"));
        }
    }
    let (cols, rows) = window.split_once('x').unwrap_or((window, window));
    args.push(kitty.display().to_string());
    args.push(format!("--config={}", config.display()));
    args.push(format!("--class={APP_ID}"));
    args.push("-o".into());
    args.push(format!("initial_window_width={cols}c"));
    args.push("-o".into());
    args.push(format!("initial_window_height={rows}c"));
    args.push(exe.display().to_string());
    args
}

/// Run `launch`; returns the process exit code.
pub fn launch(window: &str, dry_run: bool) -> i32 {
    if !valid_window(window) {
        eprintln!("idlefx-companion: --window wants COLSxROWS, like {DEFAULT_WINDOW}");
        return 2;
    }
    let name = std::env::var("IDLEFX_TERMINAL").unwrap_or_else(|_| KITTY.into());
    let kitty = find_in_path(&name);
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("idlefx-companion"));
    let config = config_path();
    let args = command(
        window,
        kitty.as_deref().unwrap_or(Path::new(&name)),
        &exe,
        &config,
    );
    if dry_run {
        println!("{}", args.join(" "));
        return 0;
    }
    let Some(_) = kitty else {
        eprintln!("idlefx-companion: `{name}` not found: install kitty, or set IDLEFX_TERMINAL to its path");
        return 1;
    };
    let running = Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", UNIT])
        .status()
        .is_ok_and(|s| s.success());
    if running {
        return 0;
    }
    if let Err(e) = std::fs::write(&config, KITTY_CONF) {
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

    #[test]
    fn kitty_flags() {
        let args = command("80x20", Path::new("/usr/bin/kitty"), Path::new("/bin/dog"), Path::new("/run/c"));
        assert!(args.contains(&"MemoryMax=128M".to_string()));
        let at = args.iter().position(|a| a == "/usr/bin/kitty").unwrap();
        assert_eq!(
            args[at + 1..],
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

    #[test]
    fn window_must_be_cols_x_rows() {
        assert!(valid_window("64x16"));
        assert!(!valid_window("64"));
        assert!(!valid_window("0x16"));
        assert!(!valid_window("64x16x2"));
    }
}
