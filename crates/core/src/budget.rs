//! CPU budget: measure CPU use and pick the `Active` frame rate under a limit.
//!
//! The measure comes from the process's own systemd `.service` cgroup when it
//! runs in one (so a terminal launched in the same unit counts too), and from
//! `/proc/self/stat` otherwise. A login session's `.scope` is not used: it
//! holds every other process of the session.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Where CPU time is read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CpuSource {
    /// `cpu.stat` of the process's systemd service cgroup (`usage_usec`).
    Cgroup(PathBuf),
    /// `/proc/self/stat` (`utime` + `stime`).
    Proc,
}

impl CpuSource {
    /// Detect the source for this process.
    pub fn detect() -> Self {
        let cgroup = std::fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
        let source = Self::from_cgroup(&cgroup, Path::new("/sys/fs/cgroup"));
        match &source {
            CpuSource::Cgroup(path) if !path.exists() => CpuSource::Proc,
            _ => source,
        }
    }

    /// Pick the source from the content of `/proc/self/cgroup` (cgroup v2:
    /// `0::<path>`): the cgroup's `cpu.stat` when the path ends in a
    /// `.service` unit, `/proc` otherwise.
    pub fn from_cgroup(content: &str, root: &Path) -> Self {
        let path = content
            .lines()
            .find_map(|line| line.strip_prefix("0::"))
            .map(str::trim)
            .unwrap_or("");
        let leaf = path.rsplit('/').next().unwrap_or("");
        if leaf.ends_with(".service") {
            CpuSource::Cgroup(root.join(path.trim_start_matches('/')).join("cpu.stat"))
        } else {
            CpuSource::Proc
        }
    }
}

/// Reads total CPU time from a [`CpuSource`], reusing one file handle and one
/// buffer.
pub struct CpuMeter {
    source: CpuSource,
    file: Option<File>,
    buf: String,
    tick: f64,
}

impl CpuMeter {
    pub fn new(source: CpuSource) -> Self {
        let path: &Path = match &source {
            CpuSource::Cgroup(p) => p,
            CpuSource::Proc => Path::new("/proc/self/stat"),
        };
        let file = File::open(path).ok();
        // SAFETY: sysconf has no preconditions.
        let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        Self {
            source,
            file,
            buf: String::with_capacity(1024),
            tick: 1.0 / if hz > 0 { hz as f64 } else { 100.0 },
        }
    }

    /// A meter on `/proc/self/stat`: this process only.
    pub fn process() -> Self {
        Self::new(CpuSource::Proc)
    }

    pub fn source(&self) -> &CpuSource {
        &self.source
    }

    /// Total CPU time used so far, or zero if it cannot be read.
    pub fn read(&mut self) -> Duration {
        let Some(file) = self.file.as_mut() else {
            return Duration::ZERO;
        };
        self.buf.clear();
        if file.seek(SeekFrom::Start(0)).is_err() || file.read_to_string(&mut self.buf).is_err() {
            return Duration::ZERO;
        }
        match self.source {
            CpuSource::Cgroup(_) => parse_cpu_stat(&self.buf).unwrap_or(Duration::ZERO),
            CpuSource::Proc => parse_proc_stat(&self.buf, self.tick).unwrap_or(Duration::ZERO),
        }
    }
}

/// `usage_usec` from a cgroup v2 `cpu.stat`.
fn parse_cpu_stat(text: &str) -> Option<Duration> {
    let usec = text
        .lines()
        .find_map(|l| l.strip_prefix("usage_usec "))?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_micros(usec))
}

/// `utime + stime` (fields 14 and 15) from `/proc/<pid>/stat`.
fn parse_proc_stat(text: &str, tick: f64) -> Option<Duration> {
    // The command name (field 2) may contain spaces; fields restart after ')'.
    let rest = &text[text.rfind(')')? + 1..];
    let mut fields = rest.split_whitespace().skip(11);
    let utime: u64 = fields.next()?.parse().ok()?;
    let stime: u64 = fields.next()?.parse().ok()?;
    Some(Duration::from_secs_f64((utime + stime) as f64 * tick))
}

/// `Active` frame rates the governor steps through.
pub const RATES: [u32; 3] = [60, 30, 20];

const WINDOW: Duration = Duration::from_secs(2);
const SAMPLE: Duration = Duration::from_millis(250);
const SLOTS: usize = 16;
const RECOVER: Duration = Duration::from_secs(5);

/// Keeps CPU use under `limit` (a share of one core) by stepping the
/// `Active` frame rate down above 90% of it, and back up after 5 s under 60%.
pub struct Governor {
    limit: f64,
    meter: CpuMeter,
    samples: [(Instant, Duration); SLOTS],
    len: usize,
    head: usize,
    level: usize,
    calm_since: Option<Instant>,
    last: Instant,
}

impl Governor {
    pub fn new(limit: f64, meter: CpuMeter) -> Self {
        let now = Instant::now();
        Self {
            limit,
            meter,
            samples: [(now, Duration::ZERO); SLOTS],
            len: 0,
            head: 0,
            level: 0,
            calm_since: None,
            last: now - SAMPLE,
        }
    }

    /// The current `Active` frame rate.
    pub fn rate(&self) -> u32 {
        RATES[self.level]
    }

    /// Sample CPU use (at most every 250 ms) and adjust the rate.
    pub fn tick(&mut self, now: Instant) {
        if now - self.last < SAMPLE {
            return;
        }
        self.last = now;
        let cpu = self.meter.read();
        self.samples[self.head] = (now, cpu);
        self.head = (self.head + 1) % SLOTS;
        self.len = (self.len + 1).min(SLOTS);
        if let Some(usage) = self.usage(now) {
            self.adjust(usage, now);
        }
    }

    /// CPU share over the window, oldest sample within 2 s to the newest.
    fn usage(&self, now: Instant) -> Option<f64> {
        let newest = self.samples[(self.head + SLOTS - 1) % SLOTS];
        let oldest = (0..self.len)
            .map(|i| self.samples[(self.head + SLOTS - self.len + i) % SLOTS])
            .find(|(t, _)| now - *t <= WINDOW)?;
        let wall = (newest.0 - oldest.0).as_secs_f64();
        (wall >= SAMPLE.as_secs_f64())
            .then(|| (newest.1.saturating_sub(oldest.1)).as_secs_f64() / wall)
    }

    fn adjust(&mut self, usage: f64, now: Instant) {
        if usage > 0.9 * self.limit {
            self.calm_since = None;
            if self.level + 1 < RATES.len() {
                self.level += 1;
                self.restart_window(now);
            }
        } else if usage < 0.6 * self.limit {
            let since = *self.calm_since.get_or_insert(now);
            if now - since >= RECOVER && self.level > 0 {
                self.level -= 1;
                self.calm_since = None;
                self.restart_window(now);
            }
        } else {
            self.calm_since = None;
        }
    }

    /// Judge the new rate on fresh samples only.
    fn restart_window(&mut self, now: Instant) {
        let newest = self.samples[(self.head + SLOTS - 1) % SLOTS];
        self.samples[0] = (now, newest.1);
        self.head = 1;
        self.len = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_service_uses_cgroup() {
        let content = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/idlefx-companion.service\n";
        assert_eq!(
            CpuSource::from_cgroup(content, Path::new("/sys/fs/cgroup")),
            CpuSource::Cgroup(PathBuf::from(
                "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/app.slice/idlefx-companion.service/cpu.stat"
            ))
        );
    }

    #[test]
    fn source_scope_uses_proc() {
        let content = "0::/user.slice/user-1000.slice/session-c8.scope\n";
        assert_eq!(
            CpuSource::from_cgroup(content, Path::new("/sys/fs/cgroup")),
            CpuSource::Proc
        );
    }

    #[test]
    fn source_app_scope_inside_user_service_uses_proc() {
        // A terminal started from a desktop lives in an app scope under
        // user@.service: the leaf is a scope, so it is not ours alone.
        let content = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-foot-1234.scope\n";
        assert_eq!(
            CpuSource::from_cgroup(content, Path::new("/sys/fs/cgroup")),
            CpuSource::Proc
        );
    }

    #[test]
    fn source_missing_cgroup_uses_proc() {
        assert_eq!(CpuSource::from_cgroup("", Path::new("/x")), CpuSource::Proc);
    }

    #[test]
    fn parses_cpu_stat() {
        let text = "usage_usec 1234567\nuser_usec 1000000\nsystem_usec 234567\n";
        assert_eq!(parse_cpu_stat(text), Some(Duration::from_micros(1_234_567)));
    }

    #[test]
    fn parses_proc_stat_with_spaces_in_name() {
        let text = "42 (my (odd) app) S 1 42 42 0 -1 4194560 100 0 0 0 250 50 0 0 20 0 1 0 1 1 1";
        assert_eq!(parse_proc_stat(text, 0.01), Some(Duration::from_secs(3)));
    }

    fn governor_with(usages: &[f64], limit: f64) -> Governor {
        let mut g = Governor::new(limit, CpuMeter::new(CpuSource::Cgroup("/nonexistent".into())));
        let start = Instant::now();
        let mut cpu = Duration::ZERO;
        for (i, u) in usages.iter().enumerate() {
            let now = start + SAMPLE * (i as u32 + 1);
            cpu += SAMPLE.mul_f64(*u);
            g.samples[g.head] = (now, cpu);
            g.head = (g.head + 1) % SLOTS;
            g.len = (g.len + 1).min(SLOTS);
            if let Some(usage) = g.usage(now) {
                g.adjust(usage, now);
            }
        }
        g
    }

    #[test]
    fn governor_steps_down_over_budget() {
        let g = governor_with(&[0.2; 12], 0.10);
        assert_eq!(g.rate(), 20);
    }

    #[test]
    fn governor_stays_up_under_budget() {
        let g = governor_with(&[0.03; 40], 0.10);
        assert_eq!(g.rate(), 60);
    }

    #[test]
    fn governor_recovers_after_calm() {
        let mut usages = vec![0.2; 6];
        usages.extend(vec![0.02; 60]);
        let g = governor_with(&usages, 0.10);
        assert_eq!(g.rate(), 60);
    }
}
