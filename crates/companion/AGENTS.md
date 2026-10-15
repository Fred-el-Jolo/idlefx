# idlefx-companion

A small procedural dog that lives all day in its own floating, transparent
terminal window. Shared rules (rendering, smoothness, idle CPU) are in the
root `AGENTS.md`; the spec is `docs/2026-10-14-dog-companion-01-spec.md`.

## The dog

- Drawn from parts (body, chest, head, snout, ears, tail, four legs) on a
  light skeleton, about 30x20 pixels standing; a cell is 1 pixel wide and 2
  tall. Critically damped springs move every part toward the current pose,
  so poses blend; the walk cycle follows the ground covered (no foot sliding).
- Poses: stand, walk, sit, lie, sleep, happy, sad, curious (plus think and a
  stretch). Review one with `idlefx-companion snapshot --pose <pose>`.
- Code: `src/dog.rs` (body, poses, drawing), `src/brain.rs` (behaviour),
  `src/app.rs` (the live app on the core loop), `src/socket.rs` (events),
  `src/launch.rs` (the window).

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
  curious, thinking, stretch; walk when called. Calm states change after a
  random time (`--seed` makes it repeatable).
- Night is **22:00 to 07:00** local time: the dog settles down and sleeps; a
  reaction wakes it and it goes back to sleep within a minute. The first
  wake-up after 07:00 is a stretch.
- Click the dog to pet it (happy, a heart rises); click elsewhere and it
  walks there. `q`, `Esc` or Ctrl-C quit.

## Events

`idlefx-companion poke <event>` sends one event to the running dog through
`$XDG_RUNTIME_DIR/idlefx-companion.sock`. It exits 0 silently when no dog is
running, so hooks never fail because of it. Unknown names are ignored.
`idlefx-companion poke --list` (or `poke` alone) lists them:

| Event | Reaction |
|---|---|
| `hello` | happy greeting |
| `thinking` | sits, head tilted, ears up, until the next event or 60 s |
| `success` | happy |
| `failure` | sad for a few seconds |
| `commit` | happy, with a `!` |

Hook snippets (not installed for you):

- Claude Code, in `~/.claude/settings.json`:

  ```json
  {
    "hooks": {
      "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "idlefx-companion poke thinking" }] }],
      "Stop": [{ "hooks": [{ "type": "command", "command": "idlefx-companion poke success" }] }]
    }
  }
  ```

- git, in a repo's `.git/hooks/post-commit` (executable):

  ```sh
  #!/bin/sh
  idlefx-companion poke commit
  ```

## Managing the running dog

| Command | What it does |
|---|---|
| `idlefx-companion status` | Whether the dog runs, as the launched window or in a terminal, with its pid; exits 3 when it does not |
| `idlefx-companion stop` | Closes the launched window (or `q` in it); says so when nothing runs |
| `idlefx-companion stats` | Every 2 s: the dog's CPU and memory, plus, for the launched window, the unit's CPU against its 10% quota and memory against its 128 MB cap; Ctrl-C to stop |
| `idlefx-companion poke --list` | The events and the dog's reactions |

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
- **GNOME**: install the shipped Shell extension, then log out and back in
  (Wayland):

  ```sh
  cp -r crates/companion/contrib/gnome/idlefx-companion@idlefx ~/.local/share/gnome-shell/extensions/
  gnome-extensions enable idlefx-companion@idlefx
  ```

  It keeps the window above other windows, on all workspaces, at the
  bottom-right.

## Test options

`--trace FILE` (one line per state change: time, from, to, cause, and
`latency_ms` for input), `--stats`, `--duration SECS`, `--state STATE` (start
in a state and hold it until an input), `--clock HH:MM`, `--at X`, `--seed N`,
and `simulate --start HH:MM --duration SECS` (headless, fake clock).
