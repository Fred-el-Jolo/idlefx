//! `idlefx-companion hooks install | status | uninstall`: the hooks that poke
//! the dog, set up in Claude Code, git, hypridle and GNOME Shell.
//!
//! The hook files are the ones shipped in `contrib/`, built into the binary.
//! Each target is changed in the smallest way that can be undone: entries
//! added to `~/.claude/settings.json` (backed up first), a block appended to
//! an existing git hook, one `source =` line appended to `hypridle.conf`, the
//! GNOME extension copied into its own directory. Uninstall removes exactly
//! that. Every target gets one report line: `✓` done, `✗` failed, `–` skipped.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value};

const CLAUDE: &str = include_str!("../contrib/claude/settings.json");
const GIT: [(&str, &str); 4] = [
    ("post-commit", include_str!("../contrib/git/hooks/post-commit")),
    ("pre-push", include_str!("../contrib/git/hooks/pre-push")),
    ("post-checkout", include_str!("../contrib/git/hooks/post-checkout")),
    ("post-merge", include_str!("../contrib/git/hooks/post-merge")),
];
const HYPRIDLE: &str = include_str!("../contrib/hypridle/idlefx-companion.conf");
const GNOME_UUID: &str = "idlefx-companion@idlefx";
const GNOME: [(&str, &str); 2] = [
    ("extension.js", include_str!("../contrib/gnome/idlefx-companion@idlefx/extension.js")),
    ("metadata.json", include_str!("../contrib/gnome/idlefx-companion@idlefx/metadata.json")),
];
/// What marks our Claude hooks and our git hook blocks.
const TAG: &str = "idlefx-companion poke ";
const GIT_MARK: &str = "# >>> idlefx-companion >>>";
const BIN: &str = "idlefx-companion";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Install,
    Status,
    Uninstall,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Claude,
    Git,
    Hypridle,
    Gnome,
}

impl Target {
    const ALL: [Target; 4] = [Target::Claude, Target::Git, Target::Hypridle, Target::Gnome];

    fn name(self) -> &'static str {
        match self {
            Target::Claude => "claude",
            Target::Git => "git",
            Target::Hypridle => "hypridle",
            Target::Gnome => "gnome",
        }
    }
}

/// Where things are, from `$HOME` and the current directory.
struct Ctx {
    home: PathBuf,
    config: PathBuf,
    data: PathBuf,
    template: bool,
}

impl Ctx {
    fn tilde(&self, path: &Path) -> String {
        match path.strip_prefix(&self.home) {
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.display().to_string(),
        }
    }
}

fn report(mark: char, name: &str, text: &str) {
    println!("{mark} {name:<9} {text}");
}

/// The first executable `name` on PATH.
fn which(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| p.metadata().is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0))
}

pub fn main(args: &[String]) -> i32 {
    let action = match args.first().map(String::as_str) {
        Some("install") => Action::Install,
        Some("status") => Action::Status,
        Some("uninstall") => Action::Uninstall,
        _ => {
            eprintln!("usage: idlefx-companion hooks install|status|uninstall [claude] [git] [hypridle] [gnome] [--template]");
            return 2;
        }
    };
    let mut named = Vec::new();
    let mut template = false;
    for a in &args[1..] {
        match a.as_str() {
            "--template" => template = true,
            name => match Target::ALL.into_iter().find(|t| t.name() == name) {
                Some(t) if !named.contains(&t) => named.push(t),
                Some(_) => {}
                None => {
                    eprintln!("idlefx-companion hooks: unknown target {name} (claude, git, hypridle, gnome)");
                    return 2;
                }
            },
        }
    }
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        eprintln!("idlefx-companion hooks: HOME is not set");
        return 2;
    };
    let env_dir = |var: &str, default: &str| {
        std::env::var_os(var)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(default))
    };
    let ctx = Ctx {
        config: env_dir("XDG_CONFIG_HOME", ".config"),
        data: env_dir("XDG_DATA_HOME", ".local/share"),
        home: home.clone(),
        template,
    };

    let explicit = !named.is_empty();
    let targets = if explicit { named } else { Target::ALL.to_vec() };
    let mut failed = false;
    if action != Action::Uninstall {
        match which(BIN) {
            Some(p) => report('✓', "path", &format!("{BIN} on PATH: {}", ctx.tilde(&p))),
            None => {
                report(
                    '✗',
                    "path",
                    &format!("{BIN} is not on PATH: the hooks call it by name (cargo install --path crates/companion)"),
                );
                failed |= action == Action::Install;
            }
        }
    }
    if action == Action::Status {
        let running = crate::control::running();
        report(if running { '✓' } else { '–' }, "dog", if running { "running" } else { "not running" });
    }
    for t in targets {
        let name = t.name();
        if let Err(why) = applies(&ctx, t) {
            if explicit {
                report('✗', name, &why);
                failed = true;
            } else {
                report('–', name, &why);
            }
            continue;
        }
        let result = match action {
            Action::Install => install(&ctx, t),
            Action::Uninstall => uninstall(&ctx, t),
            Action::Status => status(&ctx, t).and_then(|(n, m)| match n {
                _ if n == m => Ok(format!("installed ({n}/{m})")),
                0 => Err("missing".to_string()),
                _ => Err(format!("partial ({n}/{m})")),
            }),
        };
        match result {
            Ok(text) => report('✓', name, &text),
            Err(text) => {
                report('✗', name, &text);
                failed = true;
            }
        }
    }
    i32::from(failed)
}

/// Whether a target can be set up here; the reason when it cannot.
fn applies(ctx: &Ctx, t: Target) -> Result<(), String> {
    match t {
        Target::Claude => Ok(()),
        Target::Git if ctx.template || repo_hooks().is_some() => Ok(()),
        Target::Git => Err("not inside a git repository".into()),
        Target::Hypridle if hypridle_conf(ctx).exists() => Ok(()),
        Target::Hypridle => Err(format!("no {} (hypridle is not set up)", ctx.tilde(&hypridle_conf(ctx)))),
        Target::Gnome if which("gnome-extensions").is_some() => Ok(()),
        Target::Gnome => Err("gnome-extensions not found: GNOME Shell is not installed".into()),
    }
}

fn install(ctx: &Ctx, t: Target) -> Result<String, String> {
    match t {
        Target::Claude => claude_install(ctx),
        Target::Git => git_install(ctx),
        Target::Hypridle => hypridle_install(ctx),
        Target::Gnome => gnome_install(ctx),
    }
}

fn uninstall(ctx: &Ctx, t: Target) -> Result<String, String> {
    match t {
        Target::Claude => claude_uninstall(ctx),
        Target::Git => git_uninstall(ctx),
        Target::Hypridle => hypridle_uninstall(ctx),
        Target::Gnome => gnome_uninstall(ctx),
    }
}

/// How many of a target's pieces are in place, out of how many.
fn status(ctx: &Ctx, t: Target) -> Result<(usize, usize), String> {
    match t {
        Target::Claude => claude_status(ctx),
        Target::Git => Ok(git_dirs(ctx, false)?.iter().fold((0, 0), |(n, m), dir| {
            let here = GIT.iter().filter(|(name, _)| has_block(&dir.join(name))).count();
            (n + here, m + GIT.len())
        })),
        Target::Hypridle => {
            let line = hypridle_line(ctx);
            let conf = std::fs::read_to_string(hypridle_conf(ctx)).unwrap_or_default();
            let sourced = conf.lines().any(|l| l == line);
            let listener = same(&hypridle_listener(ctx), HYPRIDLE);
            Ok((usize::from(sourced) + usize::from(listener), 2))
        }
        Target::Gnome => {
            let dir = gnome_dir(ctx);
            Ok((GNOME.iter().filter(|(f, text)| same(&dir.join(f), text)).count(), GNOME.len()))
        }
    }
}

fn same(path: &Path, text: &str) -> bool {
    std::fs::read(path).is_ok_and(|b| b == text.as_bytes())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

// ---- Claude Code ----

fn settings(ctx: &Ctx) -> PathBuf {
    ctx.home.join(".claude/settings.json")
}

fn backup(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

/// Our hook entries, by event, in the snippet's order.
fn claude_snippet() -> Map<String, Value> {
    let v: Value = serde_json::from_str(CLAUDE).expect("contrib/claude/settings.json is valid JSON");
    match v.get("hooks") {
        Some(Value::Object(m)) => m.clone(),
        _ => panic!("contrib/claude/settings.json has a hooks object"),
    }
}

/// Whether a hook entry (`{matcher?, hooks: [...]}`) is ours.
fn is_ours(entry: &Value) -> bool {
    entry["hooks"]
        .as_array()
        .is_some_and(|hs| hs.iter().any(|h| h["command"].as_str().is_some_and(|c| c.contains(TAG))))
}

/// A settings file: its bytes and its top-level object.
type Settings = (Vec<u8>, Map<String, Value>);

/// The settings file, parsed; `None` when it does not exist.
fn read_settings(ctx: &Ctx, path: &Path) -> Result<Option<Settings>, String> {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read {}: {e}", ctx.tilde(path))),
    };
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "{} is not valid JSON (line {}, column {}): nothing written",
            ctx.tilde(path),
            e.line(),
            e.column()
        )
    })?;
    match value {
        Value::Object(root) => Ok(Some((bytes, root))),
        _ => Err(format!("{} is not a JSON object: nothing written", ctx.tilde(path))),
    }
}

fn hooks_of<'a>(ctx: &Ctx, path: &Path, root: &'a mut Map<String, Value>) -> Result<&'a mut Map<String, Value>, String> {
    root.entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| format!("\"hooks\" in {} is not an object: nothing written", ctx.tilde(path)))
}

fn pretty(root: &Map<String, Value>) -> Vec<u8> {
    let mut out = serde_json::to_vec_pretty(root).expect("a JSON object serializes");
    out.push(b'\n');
    out
}

fn claude_install(ctx: &Ctx) -> Result<String, String> {
    let path = settings(ctx);
    let (old, mut root) = match read_settings(ctx, &path)? {
        Some((bytes, root)) => (Some(bytes), root),
        None => (None, Map::new()),
    };
    let snippet = claude_snippet();
    let hooks = hooks_of(ctx, &path, &mut root)?;
    let mut added = 0;
    for (event, entries) in &snippet {
        let list = hooks
            .entry(event.as_str())
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| format!("hooks.{event} in {} is not a list: nothing written", ctx.tilde(&path)))?;
        if !list.iter().any(is_ours) {
            list.extend(entries.as_array().into_iter().flatten().cloned());
            added += 1;
        }
    }
    if added == 0 {
        return Ok(format!("{}: already installed ({} hooks)", ctx.tilde(&path), snippet.len()));
    }
    if let Some(bytes) = &old {
        write(&backup(&path), bytes)?;
    }
    write(&path, &pretty(&root))?;
    let note = if old.is_some() { ", backup settings.json.bak" } else { "" };
    Ok(format!("{}: {added} hooks added{note}", ctx.tilde(&path)))
}

fn claude_uninstall(ctx: &Ctx) -> Result<String, String> {
    let path = settings(ctx);
    let Some((_, mut root)) = read_settings(ctx, &path)? else {
        return Ok(format!("{}: nothing to remove", ctx.tilde(&path)));
    };
    let mut removed = 0;
    if let Some(Value::Object(hooks)) = root.get_mut("hooks") {
        hooks.retain(|_, list| {
            let Some(entries) = list.as_array_mut() else { return true };
            let before = entries.len();
            entries.retain(|e| !is_ours(e));
            removed += before - entries.len();
            // A list we emptied goes with our entries.
            !(entries.is_empty() && before > 0)
        });
        if hooks.is_empty() && removed > 0 {
            root.shift_remove("hooks");
        }
    }
    if removed == 0 {
        return Ok(format!("{}: nothing to remove", ctx.tilde(&path)));
    }
    // Unchanged since the install: put the old file back, byte for byte.
    let bak = backup(&path);
    let saved = std::fs::read(&bak)
        .ok()
        .filter(|b| serde_json::from_slice::<Value>(b).is_ok_and(|v| v == Value::Object(root.clone())));
    let how = if let Some(bytes) = saved {
        write(&path, &bytes)?;
        let _ = std::fs::remove_file(&bak);
        ", restored from settings.json.bak"
    } else if root.is_empty() && !bak.exists() {
        // The install created the file.
        std::fs::remove_file(&path).map_err(|e| format!("cannot remove {}: {e}", ctx.tilde(&path)))?;
        ", file removed"
    } else {
        write(&path, &pretty(&root))?;
        ""
    };
    Ok(format!("{}: {removed} hooks removed{how}", ctx.tilde(&path)))
}

fn claude_status(ctx: &Ctx) -> Result<(usize, usize), String> {
    let snippet = claude_snippet();
    let path = settings(ctx);
    let Some((_, root)) = read_settings(ctx, &path)? else {
        return Ok((0, snippet.len()));
    };
    let n = snippet
        .keys()
        .filter(|event| {
            root.get("hooks")
                .and_then(|h| h.get(event.as_str()))
                .and_then(Value::as_array)
                .is_some_and(|l| l.iter().any(is_ours))
        })
        .count();
    Ok((n, snippet.len()))
}

// ---- git ----

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The current repository's hooks directory.
fn repo_hooks() -> Option<PathBuf> {
    let dir = git(&["rev-parse", "--git-path", "hooks"])?;
    let cwd = std::env::current_dir().ok()?;
    Some(cwd.join(dir))
}

fn default_template(ctx: &Ctx) -> PathBuf {
    ctx.config.join("git/template")
}

/// The configured template directory, `~` expanded.
fn template_dir(ctx: &Ctx) -> Option<PathBuf> {
    let dir = git(&["config", "--global", "--get", "init.templateDir"])?;
    match dir.strip_prefix("~/") {
        Some(rest) => Some(ctx.home.join(rest)),
        None => Some(PathBuf::from(dir)),
    }
}

/// The hooks directories to work on: the repo's, and with `--template` the
/// template's (set up when `create` and not configured yet).
fn git_dirs(ctx: &Ctx, create: bool) -> Result<Vec<PathBuf>, String> {
    let mut dirs: Vec<PathBuf> = repo_hooks().into_iter().collect();
    if ctx.template {
        let dir = match template_dir(ctx) {
            Some(d) => d,
            None if create => {
                let d = default_template(ctx);
                git(&["config", "--global", "init.templateDir", &d.to_string_lossy()])
                    .ok_or("cannot set git's init.templateDir")?;
                d
            }
            None => default_template(ctx),
        };
        dirs.push(dir.join("hooks"));
    }
    Ok(dirs)
}

/// A hook's block: the shipped file without its `#!` line.
fn block(text: &str) -> &str {
    text.split_once('\n').map_or("", |(_, rest)| rest)
}

fn has_block(path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|t| t.contains(GIT_MARK))
}

fn git_install(ctx: &Ctx) -> Result<String, String> {
    use std::os::unix::fs::PermissionsExt;
    let mut done = Vec::new();
    for dir in git_dirs(ctx, true)? {
        let (mut created, mut appended) = (0, Vec::new());
        for (name, text) in GIT {
            let path = dir.join(name);
            match std::fs::read_to_string(&path) {
                Ok(old) if old.contains(GIT_MARK) => {}
                Ok(old) => {
                    write(&path, format!("{old}\n{}", block(text)).as_bytes())?;
                    appended.push(name);
                }
                Err(_) => {
                    write(&path, text.as_bytes())?;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                        .map_err(|e| format!("cannot make {} executable: {e}", path.display()))?;
                    created += 1;
                }
            }
        }
        let what = match (created, appended.is_empty()) {
            (0, true) => "already installed".to_string(),
            (_, true) => format!("{created} hooks created"),
            (0, false) => format!("appended to {}", appended.join(", ")),
            (_, false) => format!("{created} hooks created, appended to {}", appended.join(", ")),
        };
        done.push(format!("{}: {what}", ctx.tilde(&dir)));
    }
    Ok(done.join("; "))
}

fn git_uninstall(ctx: &Ctx) -> Result<String, String> {
    let mut done = Vec::new();
    for dir in git_dirs(ctx, false)? {
        let mut removed = 0;
        for (name, text) in GIT {
            let path = dir.join(name);
            let Ok(now) = std::fs::read_to_string(&path) else { continue };
            if now == text {
                std::fs::remove_file(&path).map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
                removed += 1;
            } else if let Some(at) = now.find(&format!("\n{}", block(text))) {
                let restored = format!("{}{}", &now[..at], &now[at + 1 + block(text).len()..]);
                write(&path, restored.as_bytes())?;
                removed += 1;
            }
        }
        done.push(format!("{}: {removed} hooks removed", ctx.tilde(&dir)));
        // Our own template directory goes once it is empty.
        if ctx.template && dir == default_template(ctx).join("hooks") && is_empty_dir(&dir) {
            let _ = std::fs::remove_dir(&dir);
            let _ = std::fs::remove_dir(default_template(ctx));
            let _ = git(&["config", "--global", "--unset", "init.templateDir"]);
        }
    }
    Ok(done.join("; "))
}

fn is_empty_dir(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|mut d| d.next().is_none())
}

// ---- hypridle ----

fn hypridle_conf(ctx: &Ctx) -> PathBuf {
    ctx.config.join("hypr/hypridle.conf")
}

fn hypridle_listener(ctx: &Ctx) -> PathBuf {
    ctx.config.join("hypr/idlefx-companion-hypridle.conf")
}

fn hypridle_line(ctx: &Ctx) -> String {
    format!("source = {}", hypridle_listener(ctx).display())
}

fn hypridle_install(ctx: &Ctx) -> Result<String, String> {
    let conf = hypridle_conf(ctx);
    let listener = hypridle_listener(ctx);
    let line = hypridle_line(ctx);
    let old = std::fs::read_to_string(&conf).map_err(|e| format!("cannot read {}: {e}", ctx.tilde(&conf)))?;
    let sourced = old.lines().any(|l| l == line);
    let fresh = same(&listener, HYPRIDLE);
    if sourced && fresh {
        return Ok(format!("{}: already installed", ctx.tilde(&conf)));
    }
    if !fresh {
        write(&listener, HYPRIDLE.as_bytes())?;
    }
    if !sourced {
        write(&conf, format!("{old}\n{line}\n").as_bytes())?;
    }
    Ok(format!("{}: source line added; restart hypridle to load it", ctx.tilde(&conf)))
}

fn hypridle_uninstall(ctx: &Ctx) -> Result<String, String> {
    let conf = hypridle_conf(ctx);
    let old = std::fs::read_to_string(&conf).map_err(|e| format!("cannot read {}: {e}", ctx.tilde(&conf)))?;
    let added = format!("\n{}\n", hypridle_line(ctx));
    let mut removed = false;
    if let Some(at) = old.find(&added) {
        write(&conf, format!("{}{}", &old[..at], &old[at + added.len()..]).as_bytes())?;
        removed = true;
    }
    let listener = hypridle_listener(ctx);
    if listener.exists() {
        std::fs::remove_file(&listener).map_err(|e| format!("cannot remove {}: {e}", ctx.tilde(&listener)))?;
        removed = true;
    }
    Ok(format!(
        "{}: {}",
        ctx.tilde(&conf),
        if removed { "source line and listener removed; restart hypridle" } else { "nothing to remove" }
    ))
}

// ---- GNOME Shell ----

fn gnome_dir(ctx: &Ctx) -> PathBuf {
    ctx.data.join("gnome-shell/extensions").join(GNOME_UUID)
}

fn gnome_extensions(args: &[&str]) -> Result<(), String> {
    let out = Command::new("gnome-extensions")
        .args(args)
        .output()
        .map_err(|e| format!("cannot run gnome-extensions: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn gnome_install(ctx: &Ctx) -> Result<String, String> {
    let dir = gnome_dir(ctx);
    let fresh = GNOME.iter().all(|(f, text)| same(&dir.join(f), text));
    for (f, text) in GNOME {
        if !same(&dir.join(f), text) {
            write(&dir.join(f), text.as_bytes())?;
        }
    }
    gnome_extensions(&["enable", GNOME_UUID])
        .map_err(|e| format!("{}: copied, but enabling it failed: {e}", ctx.tilde(&dir)))?;
    Ok(if fresh {
        format!("{}: already installed", ctx.tilde(&dir))
    } else {
        format!("{}: copied and enabled; log out and back in to load it on Wayland", ctx.tilde(&dir))
    })
}

fn gnome_uninstall(ctx: &Ctx) -> Result<String, String> {
    let dir = gnome_dir(ctx);
    if !dir.exists() {
        return Ok(format!("{}: nothing to remove", ctx.tilde(&dir)));
    }
    let _ = gnome_extensions(&["disable", GNOME_UUID]);
    std::fs::remove_dir_all(&dir).map_err(|e| format!("cannot remove {}: {e}", ctx.tilde(&dir)))?;
    Ok(format!("{}: disabled and removed", ctx.tilde(&dir)))
}
