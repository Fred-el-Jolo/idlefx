//! Managing the running dog from another terminal: `status`, `stop`, `stats`.
//!
//! The launched window is the systemd user unit `idlefx-companion`; a dog run
//! straight in a terminal is found by its process name.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::launch::UNIT;

/// The kernel keeps 15 characters of a process name.
const COMM: &str = "idlefx-companio";
/// Subcommands that are not the dog itself.
const TOOLS: [&str; 9] = [
    "launch", "poke", "snapshot", "simulate", "status", "stop", "stats", "help", "--help",
];
const INTERVAL: Duration = Duration::from_secs(2);

fn systemctl(args: &[&str]) -> Option<std::process::Output> {
    Command::new("systemctl").arg("--user").args(args).output().ok()
}

/// Whether the launched window's unit is running.
fn unit_active() -> bool {
    systemctl(&["is-active", "--quiet", UNIT]).is_some_and(|o| o.status.success())
}

/// The unit's cgroup directory, when it runs.
fn unit_cgroup() -> Option<PathBuf> {
    if !unit_active() {
        return None;
    }
    let out = systemctl(&["show", "-p", "ControlGroup", "--value", UNIT])?;
    let cg = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let dir = PathBuf::from(format!("/sys/fs/cgroup{cg}"));
    (!cg.is_empty() && dir.join("cpu.stat").exists()).then_some(dir)
}

fn is_dog(pid: u32) -> bool {
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
    if comm.trim() != COMM {
        return false;
    }
    let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    let first_arg = cmdline.split(|b| *b == 0).nth(1).unwrap_or_default();
    !TOOLS.iter().any(|t| t.as_bytes() == first_arg)
}

/// The dog's pid: the one in the launched unit first, else any running dog.
fn find_dog(cgroup: Option<&Path>) -> Option<u32> {
    let me = std::process::id();
    let in_unit: Vec<u32> = cgroup
        .and_then(|cg| std::fs::read_to_string(cg.join("cgroup.procs")).ok())
        .map(|procs| procs.lines().filter_map(|l| l.parse().ok()).collect())
        .unwrap_or_default();
    let anywhere = std::fs::read_dir("/proc")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse().ok());
    in_unit
        .into_iter()
        .chain(anywhere)
        .find(|&pid| pid != me && is_dog(pid))
}

/// `status`: exit 0 when the dog runs, 3 when it does not (like systemctl).
pub fn status() -> i32 {
    let cgroup = unit_cgroup();
    match (cgroup.is_some(), find_dog(cgroup.as_deref())) {
        (true, pid) => {
            let pid = pid.map_or(String::new(), |p| format!(", dog pid {p}"));
            println!("idlefx-companion: running in its window (unit {UNIT}{pid})");
            let _ = std::io::stdout().flush();
            let _ = Command::new("systemctl")
                .args(["--user", "status", UNIT, "--no-pager", "--lines=0"])
                .status();
            0
        }
        (false, Some(pid)) => {
            println!("idlefx-companion: running in a terminal (pid {pid}), not as the launched window");
            0
        }
        (false, None) => {
            println!("idlefx-companion is not running");
            3
        }
    }
}

/// `stop`: stop the launched window; a no-op when it is not running.
pub fn stop() -> i32 {
    if !unit_active() {
        println!("idlefx-companion is not running");
        return 0;
    }
    match Command::new("systemctl").args(["--user", "stop", UNIT]).status() {
        Ok(s) if s.success() => {
            println!("idlefx-companion stopped");
            0
        }
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("idlefx-companion: cannot run systemctl: {e}");
            1
        }
    }
}

fn read(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// CPU ticks (utime + stime) of a process.
fn ticks(pid: u32) -> Option<u64> {
    let stat = read(format!("/proc/{pid}/stat"))?;
    let mut fields = stat[stat.rfind(')')? + 1..].split_whitespace().skip(11);
    Some(fields.next()?.parse::<u64>().ok()? + fields.next()?.parse::<u64>().ok()?)
}

fn rss_kb(pid: u32) -> Option<u64> {
    read(format!("/proc/{pid}/status"))?
        .lines()
        .find_map(|l| l.strip_prefix("VmRSS:"))?
        .trim()
        .trim_end_matches("kB")
        .trim()
        .parse()
        .ok()
}

fn cgroup_usec(cg: &Path) -> Option<u64> {
    read(cg.join("cpu.stat"))?
        .lines()
        .find_map(|l| l.strip_prefix("usage_usec "))?
        .trim()
        .parse()
        .ok()
}

fn cgroup_bytes(cg: &Path, file: &str) -> Option<u64> {
    read(cg.join(file))?.trim().parse().ok()
}

/// The unit's CPU quota in percent of one core, from `cpu.max`.
fn cgroup_quota(cg: &Path) -> Option<f64> {
    let max = read(cg.join("cpu.max"))?;
    let mut parts = max.split_whitespace();
    let quota: f64 = parts.next()?.parse().ok()?;
    let period: f64 = parts.next()?.parse().ok()?;
    Some(100.0 * quota / period)
}

/// `stats`: CPU and memory every 2 s until Ctrl-C or the dog exits.
pub fn stats() -> i32 {
    let cgroup = unit_cgroup();
    let Some(pid) = find_dog(cgroup.as_deref()) else {
        println!("idlefx-companion is not running");
        return 0;
    };
    // SAFETY: sysconf has no preconditions.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f64;
    let secs = INTERVAL.as_secs_f64();
    let unit = if cgroup.is_some() { format!(", unit {UNIT}") } else { String::new() };
    println!("dog pid {pid}{unit} (Ctrl-C to stop)");
    let _ = std::io::stdout().flush();
    let mut last = ticks(pid);
    let mut last_unit = cgroup.as_deref().and_then(cgroup_usec);
    loop {
        std::thread::sleep(INTERVAL);
        let (Some(now), Some(before)) = (ticks(pid), last) else {
            println!("idlefx-companion exited");
            return 0;
        };
        let cpu = now.saturating_sub(before) as f64 / hz / secs * 100.0;
        let rss = rss_kb(pid).unwrap_or(0) as f64 / 1024.0;
        let mut line = format!("{}  dog  cpu {cpu:5.2}%  rss {rss:5.1} MB", clock());
        if let Some(cg) = cgroup.as_deref() {
            let used = cgroup_usec(cg);
            if let (Some(u1), Some(u0)) = (used, last_unit) {
                let unit_cpu = u1.saturating_sub(u0) as f64 / 1e6 / secs * 100.0;
                let quota = cgroup_quota(cg).map_or("no quota".into(), |q| format!("of {q:.0}%"));
                let mem = cgroup_bytes(cg, "memory.current").unwrap_or(0) as f64 / 1_048_576.0;
                let cap = cgroup_bytes(cg, "memory.max")
                    .map_or("no cap".into(), |m| format!("of {:.0}", m as f64 / 1_048_576.0));
                line += &format!("   |   unit  cpu {unit_cpu:5.2}% {quota}  mem {mem:5.1} MB {cap}");
            }
            last_unit = used;
        }
        println!("{line}");
        let _ = std::io::stdout().flush();
        last = Some(now);
    }
}

/// Local wall-clock time, `HH:MM:SS`.
fn clock() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as libc::time_t;
    // SAFETY: localtime_r writes only into `tm`, which we own.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&secs, &mut tm) }.is_null() {
        return String::from("--:--:--");
    }
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}
