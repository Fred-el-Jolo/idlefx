# idlefx-companion

A small procedural dog that lives all day in its own floating, transparent
terminal window. Shared rules (rendering, smoothness, idle CPU) are in the
root `AGENTS.md`; the spec is `docs/2026-10-14-dog-companion-01-spec.md`.

## The dog

- Drawn from parts (body, chest, head, snout, ears, tail, four legs) on a
  light skeleton, about 30x20 pixels standing; a cell is 1 pixel wide and 2
  tall. Critically damped springs move every part toward the current pose,
  so poses blend; the walk cycle follows the ground covered (no foot sliding).
- Poses: stand, walk, sit, lie, sleep, happy, sad, curious (plus think, a
  stretch, dig and alert). Review one with
  `idlefx-companion snapshot --pose <pose>`.
- Code: `src/dog.rs` (body, poses, drawing), `src/brain.rs` (behaviour),
  `src/app.rs` (the live app on the core loop), `src/socket.rs` (events),
  `src/hooks.rs` (the hook installer), `src/launch.rs` (the window).

## Window and resources

- Default window: **64x16** cells (a 64x32-pixel canvas, about 580x320 screen
  pixels with kitty's default font). Try other sizes with
  `idlefx-companion launch --window COLSxROWS`; the dog keeps its size and
  wanders the width it gets.
- **CPU ceiling: 10% of one core**, for the whole window (the terminal and the dog):
  `CPUQuota=10%` on the unit, and the core's CPU budget lowers the walking
  frame rate (60 → 30 → 20) before the quota would throttle it.
- Process targets in the default window: under 1% asleep, under 3% idle,
  under 6% walking; resident memory under 32 MB.
- Frame rates: 60fps while walking or reacting, 4fps idle, 2fps asleep.

## Behaviour

- States: idle, wander, sit, lie, sleep, and the reactions happy, sad,
  curious, thinking, stretch, working (digging), alert; walk when called, run
  after a push. Calm states change after a random time (`--seed` makes it
  repeatable).
- Night is **22:00 to 07:00** local time: the dog settles down and sleeps; a
  reaction wakes it and it goes back to sleep within a minute. The first
  wake-up after 07:00 is a stretch.
- Click the dog to pet it (happy, a heart rises); click elsewhere and it
  walks there. `q`, `Esc` or Ctrl-C quit.
- **Away:** after the `sleep` event (screen locked or idle) the dog sleeps,
  without its routine, until `wake` or a click; other events still get their
  reaction, then it goes back to sleep.
- **Break reminder:** 25 min after the dog starts or after the last `wake`,
  it does a play bow with a steaming cup above its head (6 s), then again
  every 5 min until `sleep`. Petting it during a reminder counts as taking
  the break (the next one comes 25 min later). Only from 07:00 to 22:00.
  `--break-every MIN` changes the 25 (0 turns it off), live and in
  `simulate`.

## Events

`idlefx-companion poke <event>` sends one event to the running dog through
`$XDG_RUNTIME_DIR/idlefx-companion.sock`. It exits 0 silently when no dog is
running, so hooks never fail because of it. Unknown names are ignored.
`idlefx-companion poke --list` (or `poke` alone) lists them:

| Event | Sent by | Reaction |
|---|---|---|
| `hello` | Claude Code: SessionStart | happy greeting |
| `thinking` | Claude Code: UserPromptSubmit | sits, head tilted, ears up, until the next event or 20 s |
| `working` | Claude Code: PreToolUse | digs, until another event or 20 s after the first `working`; more `working` is ignored until another event, so a long task gets one dig |
| `alert` | Claude Code: Notification (`permission_prompt`) | stands tall, ears up, blinking `?`, until the next event or a click (10 min at most) |
| `success` | Claude Code: Stop | happy |
| `failure` | Claude Code: StopFailure | sad for a few seconds |
| `yawn` | Claude Code: PreCompact | a long play-bow stretch |
| `bye` | Claude Code: SessionEnd | lies down |
| `commit` | git: post-commit | happy, with a `!` |
| `push` | git: pre-push | runs to the far edge and back, then happy with a `!` |
| `curious` | git: post-checkout (branch switch), post-merge | looks around |
| `sleep` | hypridle / GNOME: idle 5 min or screen locked | sleeps until `wake` (see Away) |
| `wake` | hypridle / GNOME: back | happy greeting, a new work stretch |

Claude Code's tool failures and `idle_prompt` are left out on purpose: both
fire far too often. git runs no hook when a merge stops on conflicts, so there
is no conflict reaction.

### Installing the hooks

```sh
idlefx-companion hooks install      # everything that applies here
idlefx-companion hooks status       # what is installed; exit 1 if anything is missing
idlefx-companion hooks uninstall    # removes exactly what install added
```

Each takes targets (`claude`, `git`, `hypridle`, `gnome`; none means all that
apply: claude always, git inside a repo, hypridle when
`~/.config/hypr/hypridle.conf` exists, gnome when `gnome-extensions` is on
PATH). Each prints one line per target, `✓` done, `✗` failed, `–` skipped,
with what it did or why, after a first line saying whether
`idlefx-companion` is on PATH (the hooks call it by name). Install and
uninstall exit 1 when a line is `✗`. Running install twice changes nothing.

- **claude:** adds the 8 hooks to `~/.claude/settings.json` (created if
  missing) after copying it to `settings.json.bak`; your other keys keep their
  order; a file that is not valid JSON is left alone (`✗`). Uninstall puts
  the backup back when nothing else changed since.
- **git:** the 4 hooks into the current repo's hooks directory. An existing
  hook is never replaced: the dog's block (between `# >>> idlefx-companion >>>`
  and `# <<< idlefx-companion <<<`) is appended, and it keeps the hook's exit
  status. `--template` also puts them in git's template directory
  (`init.templateDir`, set to `~/.config/git/template` when unset), so new
  clones and `git init` get them. A template directory replaces git's default
  one (sample hooks, `info/exclude`).
- **hypridle:** writes `~/.config/hypr/idlefx-companion-hypridle.conf` and
  appends one `source =` line to `hypridle.conf`; restart hypridle.
- **gnome:** copies the extension to
  `~/.local/share/gnome-shell/extensions/idlefx-companion@idlefx` and enables
  it; log out and back in on Wayland.

### The hook files

They ship in `crates/companion/contrib/` (the installer has the same files
built in), for manual setups:

- `contrib/claude/settings.json`: the `hooks` object to merge into
  `~/.claude/settings.json`.
- `contrib/git/hooks/post-commit`, `contrib/git/hooks/pre-push`,
  `contrib/git/hooks/post-checkout`, `contrib/git/hooks/post-merge`: copy into
  `.git/hooks/` (executable), or append everything but the first line to a
  hook you already have.
- `contrib/hypridle/idlefx-companion.conf`: a listener (5 min idle → `sleep`,
  resume → `wake`); add `source = /path/to/it` to `hypridle.conf`. For a
  manual lock (`loginctl lock-session`), hypridle can also run
  `on_lock_cmd = idlefx-companion poke sleep` and
  `on_unlock_cmd = idlefx-companion poke wake` in its `general` block.
- `contrib/gnome/idlefx-companion@idlefx`: the GNOME Shell extension (window
  placement, plus `sleep` on lock or 5 min idle and `wake` on return; it finds
  the binary on PATH or in `~/.cargo/bin`).

Every hook exits 0, fast, whether the dog runs or not and whether
`idlefx-companion` is installed or not: it never blocks Claude, a commit or a
push.

## Managing the running dog

| Command | What it does |
|---|---|
| `idlefx-companion status` | Whether the dog runs, as the launched window or in a terminal, with its pid; exits 3 when it does not |
| `idlefx-companion stop` | Closes the launched window (or `q` in it); says so when nothing runs |
| `idlefx-companion stats` | Every 2 s: the dog's CPU and memory, plus, for the launched window, the unit's CPU against its 10% quota and memory against its 128 MB cap; Ctrl-C to stop |
| `idlefx-companion poke --list` | The events and the dog's reactions |
| `idlefx-companion hooks status` | Which hooks are installed, and whether the dog runs |

The mise tasks `status`, `stop`, `stats` and `poke` run the same commands from the repo.

## Launch and desktop setup

```sh
cargo install --path crates/companion   # puts idlefx-companion in ~/.cargo/bin
idlefx-companion launch                  # or: launch --window 80x20
```

`launch` runs `systemd-run --user --unit=idlefx-companion --collect
-p CPUQuota=10% -p MemoryMax=128M kitty --config=<transparent kitty.conf>
--class=idlefx-companion -o initial_window_width=64c
-o initial_window_height=16c idlefx-companion`.
A second launch while it runs does nothing. `--dry-run` prints the command;
`IDLEFX_TERMINAL` points it at another kitty binary. The window is kitty
only: kitty idles around 85 MB, hence the 128 MB cap. The shipped
`contrib/kitty.conf` (`background_opacity 0`) makes the default background
fully transparent, with no decorations and no padding; passed with
`--config`, it replaces your own kitty.conf for this window only.

- **Hyprland** (0.53+ rule syntax): add to `hyprland.conf`

  ```
  source = /path/to/idlefx/crates/companion/contrib/hyprland/idlefx-companion.conf
  ```

  The rule floats the window, pins it on every workspace, removes border,
  shadow, blur and rounding, skips the initial focus and puts it at the
  bottom-right.
- **GNOME**: install the shipped Shell extension
  (`idlefx-companion hooks install gnome`), then log out and back in
  (Wayland). It keeps the window above other windows, on all workspaces, at
  the bottom-right, and tells the dog when you lock the screen or go idle.

## Test options

`--trace FILE` (one line per state change: time, from, to, cause, and
`latency_ms` for input), `--stats`, `--duration SECS`, `--state STATE` (start
in a state and hold it until an input), `--clock HH:MM`, `--at X`, `--seed N`,
`--break-every MIN`, and `simulate --start HH:MM --duration SECS` (headless,
fake clock). `crates/companion/tests/dog.sh <case>` and
`crates/companion/tests/hooks.sh <case>` check the events, the break reminder
and the installer end to end (the latter on a scratch `$HOME`).
