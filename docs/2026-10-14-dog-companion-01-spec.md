---
status: "acked 2026-10-14 #6f47efe3"
effort: E3
---

# Dog companion v1, on a first idlefx-core

## Problem

idlefx has a workspace but no shared core and no content: `idlefx-core` is an empty library and `idlefx-companion` an empty binary. The brainstorm settled the first content: a small dog that lives all day in its own floating, transparent terminal window, moves smoothly, reacts to its own routine, to the user and to outside events, and costs almost nothing when nothing happens. Building it is also the first real use of the core, so the core's first pieces (terminal session, transparent half-block canvas, frame loop with activity levels) are designed here, from a concrete need.

## Goal

`idlefx-companion` shows a procedurally animated dog in a small floating window with a transparent background, on Hyprland and on GNOME. It walks, sits, lies down and sleeps on its own, follows the time of day, reacts to a mouse click, and reacts to events sent with `idlefx-companion poke <event>`. It renders at up to 60fps only while something moves fast, never uses more than 10% of a core, and uses well under 1% while it sleeps. It runs on reusable `idlefx-core` pieces: terminal session, transparent half-block canvas, frame loop with a CPU budget.
Said:
- "companion: a creature (dog, cute alien/animal...)"
- "lives in a standalone, isolated (systemd-run ?) process in a dedicated floating terminal"
- "Not necessarily big, but moving seamlessly. I don't want it to use lots of ressources also."
- "Companion seems a quick win yes. Go with a dog"
- reactions: "Self + you + events" (its own routine, time of day, mouse petting, an event socket with a poke command)
- art: "Try procedural first, we might change this afterwards"
- desktop: "Both equally" (Hyprland and GNOME)
- "only keep std size, i will better understand when i will see it. Remove the extra sizes."
- "Try to keep CPU under 10% under all circunstances."
Assumed:
- one terminal cell is 1 pixel wide and 2 pixels tall; the dog is about 30x20 pixels (about 270x180 screen pixels with foot's default font)
- the default window is 64x16 cells (a 64x32-pixel canvas, about 580x290 screen pixels with foot's default font); `launch --window COLSxROWS` tries other sizes; the dog keeps its size and wanders the width of whatever window it gets
- "under 10%" means 10% of one core, for the dog's whole window (foot and the companion together) when launched with `launch`, and for the companion process alone otherwise; when the budget is tight, a lower frame rate while moving is preferred over stutter
- v1 event names: `hello`, `thinking`, `success`, `failure`, `commit`; any other name is ignored
- night is 22:00 to 07:00 local time: the dog sleeps unless woken, and goes back to sleep after a reaction
- `q` and Ctrl-C quit; there is no other keyboard control
- the dog has no saved memory between runs (no name, mood or familiarity yet)
- Claude Code and git hooks are documented as snippets, not installed in the user's settings
- the ship brainstorm answers are recorded in `crates/ship/AGENTS.md` as part of this work, so they are not lost
- Hyprland's rule syntax changed across versions: the rules file is written for the Hyprland you run, and I'll ask for `hyprctl version` before writing it
- this machine (over SSH) has neither Hyprland, GNOME nor foot, so the window checks of S8, foot's share of the CPU, and the look of S4 are confirmed by you on your desktop; `launch` is tested here with a stand-in for foot

## Out of scope

The ship game and the idle game (beyond recording the ship brainstorm), other creatures, system watchers (away state, CPU load, battery), saved memory between runs, moving the window across the screen, click-through on transparent pixels, gamepad input, tachyonfx effects, sound, and installing hooks into the user's Claude Code or git settings.

## Constraints

- CPU never above 10% of one core, in any state. Enforced twice: the launched unit has `CPUQuota=10%` (foot and the companion together), and the core's frame loop keeps the app under its CPU budget by lowering the frame rate before the quota throttles it.
- CPU of the companion process in the default 64x16 window, one core, Linux `/proc` accounting, averaged over 10 s: under 1% asleep, under 3% idle awake (breathing, looking around), under 6% walking. These stay below 10% so foot's share of the unit's quota has room.
- Resident memory under 32 MB.
- Frame rates: up to 60fps while walking or reacting (lowered to 30 or 20 by the CPU budget if needed); at most 6fps for small idle motion (breathing, blinking, sleep); no frame at all when nothing changes.
- A poke or a mouse click shows its reaction within 100 ms.
- No heap allocation per frame once the window size is stable (AGENTS.md rule).
- Every frame wrapped in synchronized output; the terminal is restored on exit and on panic.
- The rendering rules of AGENTS.md hold: time-based animation, fixed simulation timestep with interpolation, springs or easing for motion.
- Transparent pixels are drawn with the terminal's default background, so foot's `colors.alpha` shows the desktop through them; dog pixels are opaque, without anti-aliasing.
- The window runs in foot under `systemd-run --user` with `CPUQuota=10%` and `MemoryMax=64M`.
- New dependencies limited to small, pure-Rust crates (local time, randomness); third-party versions already in `Cargo.lock` do not change.
- Clippy clean with `-D warnings` across the workspace.

## Approaches

1. Core as a small framework with an `App` trait, and a procedural dog (chosen). `idlefx-core` owns the terminal session and the frame loop and calls the app: `update(dt)` on a fixed step, `draw(canvas, alpha)` to interpolate, `activity()` to pick the frame rate (active, idle at N fps, asleep), and `event()` for input and outside wake-ups. The dog is a set of parts (body, head, ears, tail, legs) on a light skeleton; each pose is a set of targets, and critically damped springs move the parts toward them, so any pose blends into any other and the motion stays continuous. The timing rules of AGENTS.md live in one place, and every future app (ship, idle game) gets them for free.
2. Core as utilities only (terminal guard, canvas, timer), each app writing its own loop. More freedom, but each app re-implements fixed steps, interpolation and adaptive frame rate, which is exactly what AGENTS.md wants done once. Rejected.
3. Pixel-art frames for the dog. A classic look, but stepped motion, and every new pose needs new frames. Kept as a later option, as the user said ("we might change this afterwards"); a hybrid (pixel-art body, procedural tail and ears) is the middle ground if procedural shapes look too plain.

## S1 — Core: terminal session

`idlefx-core` sets the terminal up and always puts it back. Entering: raw mode, alternate screen, hidden cursor, and on request SGR mouse reporting and the Kitty keyboard protocol (when the terminal supports it). Leaving, on a normal exit and on a panic: everything undone in reverse order, the cursor shown, and the panic message printed after the restore so it stays readable. It also offers the frame wrapper that brackets each frame in synchronized output. The plasma example moves onto it.
Accepted when:
- after a normal exit the output contains the leave-alternate-screen and show-cursor sequences, and mouse reporting is turned off when it was on
- after a forced panic (a test-only flag) the same restore sequences are printed and the panic message appears after them
- every frame drawn through the core is bracketed by `CSI ? 2026 h` and `CSI ? 2026 l`
- the plasma example uses the core session and still passes its earlier checks (half-blocks, 55–62 fps, `q` quits)

## S2 — Core: transparent half-block canvas

A pixel canvas two pixels per cell, where a pixel is either a color or transparent. The blit to the ratatui buffer picks the glyph per cell: both transparent → a space on the default background; top only → `▀` with the top color on the default background; bottom only → `▄` likewise; both → `▀` with top as foreground and bottom as background. Drawing primitives are what the procedural dog needs: filled ellipse, filled capsule (thick segment with round ends), filled rectangle, and a single pixel, all hard-edged, with coordinates in pixels as floats. The canvas reallocates only when its size changes.
Accepted when:
- unit tests cover the four cell cases, and the transparent cases leave the cell background as the terminal default (`Color::Reset`)
- each primitive draws its documented pixels on a small canvas (unit tests with expected pixel maps)
- redrawing at a stable size performs no heap allocation (a test with a counting allocator)

## S3 — Core: frame loop with activity levels and a CPU budget

`idlefx_core::run(app, options)` drives an `App`: a fixed simulation step of 1/120 s (`update`), a draw per frame with the interpolation factor (`draw`), and the app's `activity()` after each step: `Active` (up to 60fps), `Idle(n)` (n fps, at most 6), or `Asleep` (no frames after the current one; the loop blocks until input, a resize, an outside wake-up or the app's next timer). Outside threads wake the loop through a handle (`Waker`) carrying an app event. Input events (keys, mouse, resize) reach `App::event`. A `--frames N` style benchmark option and frame stats stay available to every app.
The options carry a CPU budget (a share of one core). The loop measures CPU use over a sliding 2 s window: from its own cgroup's `cpu.stat` when that cgroup is a systemd `.service` unit (as `launch` creates), so the terminal's rendering counts too; otherwise, for example in a login session's `.scope`, from the process (`/proc/self/stat`). Above 90% of the budget it steps the `Active` rate down (60 → 30 → 20fps); back under 60% for 5 s, it steps up again. The simulation keeps real time whatever the frame rate.
Accepted when:
- an app reporting `Active` renders between 55 and 62 fps when it is well under budget
- an app reporting `Idle(4)` renders between 3.5 and 4.5 fps over 5 s
- an app reporting `Asleep` writes no frame after its first one and uses under 0.1 s of CPU in 10 s
- a `Waker` event sent to a sleeping app reaches `App::event` and produces a frame within 50 ms
- a deliberately expensive test app with a 10% budget drops its frame rate and, after a 2 s warm-up, stays under 10% CPU averaged over 10 s
- the budget reads the cgroup's `cpu.stat` when the process's cgroup is a `.service` unit, and `/proc/self/stat` otherwise
- with a slow terminal, frames are skipped rather than queued: the simulation keeps real time

## S4 — Dog: procedural body and animation

The dog is drawn from parts on a light skeleton: body, chest, head, snout, two ears, tail, four legs, eye, nose, about 30x20 pixels standing. A pose is a set of targets (body height and tilt, head angle, ear lift, tail angle and wag, leg angles); critically damped springs move every part toward the current pose's targets, so a change of pose blends instead of jumping. On top: a walk cycle whose leg phase follows the actual ground speed (no foot sliding), tail wag as an oscillator whose amplitude follows the mood, breathing, blinking, and ears that lag a little behind head motion. Poses: stand, walk, sit, lie, sleep (curled, slow breathing, a small `z` drifting up), happy (fast wag, small hop), sad (ears and tail down), curious (head tilt). The dog faces left or right and turns through a short squash rather than an instant flip. One palette: a warm tan coat, darker ears and tail tip, white chest, black eye and nose. A snapshot option renders a chosen pose to the terminal for review.
Accepted when:
- each of the 8 poses renders through the snapshot option without a panic, inside a 64x32 pixel canvas
- standing, the dog's bounding box is between 26 and 34 pixels wide and between 16 and 24 pixels tall
- between any two poses, no part moves more than 2 pixels between two consecutive 60fps frames (a continuity test over every pose pair)
- while walking, the planted foot moves less than 1 pixel against the ground per stride
- the user, looking at it in foot, finds the dog reads as a cute dog and moves smoothly (manual)

## S5 — Dog: behaviour

A behaviour state machine on top of the poses: idle (stand, look around, scratch), wander (walk to a random spot in the window, easing in and out), sit, lie down, sleep, and the reactions (happy, sad, curious). Idle states pick the next one by weighted randomness with randomized durations (a seed option makes runs repeatable). Time of day biases the choice: from 22:00 to 07:00 it goes to sleep and stays asleep unless something wakes it, then returns to sleep within a minute; the first wake-up after 07:00 starts with a stretch and a yawn. Each state reports the frame-loop activity it needs: walking and reactions `Active`, breathing and blinking `Idle(n)`, and between two blinks or `z`s nothing moves, so the loop sleeps. A trace option writes one line per state change (time, from, to, cause) to a file. A headless simulate option (start time, duration) runs the behaviour on a fake clock, faster than real time and without a terminal, for tests.
Accepted when:
- with a fixed seed, a headless 10-minute daytime simulation visits idle, wander, sit, lie and sleep at least once each (trace file)
- in a headless simulation starting at 23:30, the dog is asleep within 2 simulated minutes and stays asleep for the next 10 simulated minutes without input
- the process CPU limits of the Constraints hold for the asleep, idle and walking states (measured from `/proc` over 10 s each, in a real 64x16 terminal session)
- the trace file shows each state change with its cause

## S6 — Interaction: mouse

Mouse reporting is on in the companion's window. A left click on a dog pixel is petting: the happy reaction (fast wag, hop, a small heart rising) for 2 to 4 seconds. A left click elsewhere in the window makes the dog look at the spot, then walk to it. Clicks wake a sleeping dog; at night it goes back to sleep afterwards. Hit testing uses the dog's drawn pixels. `q` and Ctrl-C quit.
Accepted when:
- an SGR mouse click on a dog pixel starts the happy reaction within 100 ms (trace file)
- a click on an empty spot makes the dog walk and stop within 2 pixels of that x position (trace file)
- a click wakes a sleeping dog (trace file)
- `q` quits within 1 s and the terminal is restored

## S7 — Events: socket and poke

The running companion listens on a Unix socket at `$XDG_RUNTIME_DIR/idlefx-companion.sock` on a blocking thread that wakes the frame loop through the core `Waker` (no polling). `idlefx-companion poke <event>` connects, sends one line and exits. Events: `hello` → happy greeting; `thinking` → sits, head tilted, ears up, until the next event or 60 s; `success` → happy; `failure` → sad for a few seconds; `commit` → happy with a short bark mark (`!`). Unknown events are ignored by the companion. If no companion is running, `poke` exits 0 silently, so hooks never fail because of it. A stale socket file left by a crash is replaced at start. The companion's AGENTS.md gives copy-paste snippets for Claude Code hooks (e.g. `thinking` on prompt submit, `success` on stop) and a git `post-commit` hook.
Accepted when:
- each of the 5 events reaches the dog and starts its reaction within 100 ms (trace file)
- an unknown event changes nothing and the companion keeps running
- `poke` with no companion running exits 0 within 200 ms and prints nothing
- the companion starts cleanly when a stale socket file exists
- while waiting for events with the dog asleep, the companion uses under 0.1 s of CPU in 10 s

## S8 — Window: launch on Hyprland and GNOME

`idlefx-companion launch [--window COLSxROWS]` (or the shipped script it mirrors) starts the window as an isolated user unit: `systemd-run --user --unit=idlefx-companion --collect -p CPUQuota=10% -p MemoryMax=64M foot --config=<shipped foot.ini> --app-id=idlefx-companion --window-size-chars=<COLSxROWS> idlefx-companion`, with 64x16 by default. A second launch while it runs does nothing. The shipped `foot.ini` sets a fully transparent default background (`colors.alpha=0`), no client-side decorations, no padding. Hyprland: a shipped rules file to `source` makes the window float, pinned on every workspace, borderless, without shadow, blur or rounding, not focused on open, at the bottom-right of the screen; it matches by app id, so it works at any window size. GNOME: a small shipped GNOME Shell extension recognises the window by its app id and keeps it above other windows, on all workspaces, at the bottom-right; install steps are in the companion's AGENTS.md.
Accepted when:
- `launch` starts the unit with `CPUQuota=10%` and `MemoryMax=64M`, visible in `systemctl --user show idlefx-companion`
- `launch` passes `--window-size-chars=64x16` to foot by default, and `--window 80x20` passes `--window-size-chars=80x20`
- a second `launch` exits 0 and leaves exactly one unit running
- on Hyprland, the dog appears floating, borderless, on every workspace, with the desktop visible around it (manual)
- on GNOME, with the extension enabled, the dog stays above other windows on every workspace, with the desktop visible around it (manual)
- on the user's desktop, the unit's CPU stays under 10% while the dog walks, with motion still smooth (manual, `systemctl --user status idlefx-companion` or `systemd-cgtop`)

## S9 — Docs: what was decided

The repo records what the brainstorm settled. Root AGENTS.md: the core's role and first pieces (session, canvas, loop with its CPU budget). `crates/companion/AGENTS.md`: the dog, its window size and the `--window` option, the 10% CPU ceiling, its states and reactions, the events, the launch and desktop setup, the hook snippets. `crates/ship/AGENTS.md`: the ship answers: keyboard as the standard input with an optional gamepad, an in-terminal control panel (sliders, knobs) if keyboard precision is not enough, several cameras (cockpit, top, side) especially for big ships, realistic piloting feel without extra realism, accelerated travel time.
Accepted when:
- `crates/ship/AGENTS.md` states the five ship answers
- `crates/companion/AGENTS.md` gives the default window size and the `--window` option, the 10% CPU ceiling, the 5 events, the launch command, and the Hyprland and GNOME setup steps
- root AGENTS.md describes the core's three pieces

## Decisions

- 2026-10-14: one dog size only (std). Large (2x) and XL (4x) were dropped: the sizing math (1 cell = 1x2 pixels, a 5x window around a 4x dog fills the screen) is easier to judge once the std dog is on screen; `launch --window` lets the user try window sizes meanwhile.
- 2026-10-14: the 10% CPU ceiling covers the whole launched unit (foot and the companion), enforced by `CPUQuota=10%` and by the frame loop's CPU budget, which lowers the frame rate before the quota throttles.
- 2026-10-14: the first version of this spec gave a 48x14-cell window a 96x28-pixel canvas; a cell is 1 pixel wide and 2 tall, so that was wrong. The default window is now 64x16 cells (64x32 pixels).

## Open questions
