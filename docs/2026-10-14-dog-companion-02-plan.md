---
spec: 2026-10-14-dog-companion-01-spec.md
isa: 20261014-175022_dog-companion
closed: 2026-10-15
---

# Plan — Dog companion v1, on a first idlefx-core

## Problem

idlefx has a workspace but no shared core and no content: `idlefx-core` is an empty library and `idlefx-companion` an empty binary. The brainstorm settled the first content: a small dog that lives all day in its own floating, transparent terminal window, moves smoothly, reacts to its own routine, to the user and to outside events, and costs almost nothing when nothing happens. Building it is also the first real use of the core, so the core's first pieces (terminal session, transparent half-block canvas, frame loop with activity levels) are designed here, from a concrete need.

## Vision

A small tan dog lives in the corner of the screen all day. Most of the time it breathes, blinks, looks around, naps; now and then it trots across its patch of desktop, sits, lies down. Click it and it wags and hops; tell it a build passed and it celebrates; at night it curls up and sleeps. Every movement eases in and out, nothing jumps, and a glance at `systemd-cgtop` shows it costs next to nothing. Underneath, `idlefx-core` is now a real foundation the ship and the idle game will reuse.

## Out of Scope

The ship game and the idle game (beyond recording the ship brainstorm), other creatures, system watchers (away state, CPU load, battery), saved memory between runs, moving the window across the screen, click-through on transparent pixels, gamepad input, tachyonfx effects, sound, and installing hooks into the user's Claude Code or git settings.

## Principles

- Idle is the normal state: the cheapest path (no frame, blocked thread) is the default, and motion has to earn its frames.
- One place for timing: fixed step, interpolation, frame-rate levels and the CPU budget live in the core, never in the app.
- Everything is testable headless: a trace file, a fake-clock simulation, stats lines and pty runs prove behaviour without a desktop; only the look and the desktop window are judged by the user.
- Test seams are explicit options (`--trace`, `--stats`, `--duration`, `--state`, `--clock`, `--seed`), not hidden code paths.
- No new third-party crate when std or an already-locked crate (`libc`) does the job.

## Constraints

- CPU never above 10% of one core, in any state. Enforced twice: the launched unit has `CPUQuota=10%` (kitty and the companion together), and the core's frame loop keeps the app under its CPU budget by lowering the frame rate before the quota throttles it.
- CPU of the companion process in the default 64x16 window, one core, Linux `/proc` accounting, averaged over 10 s: under 1% asleep, under 3% idle awake (breathing, looking around), under 6% walking. These stay below 10% so kitty's share of the unit's quota has room.
- Resident memory of the companion process under 32 MB; the launched unit (kitty, which idles around 85 MB, and the companion) is capped at `MemoryMax=128M`.
- Frame rates: up to 60fps while walking or reacting (lowered to 30 or 20 by the CPU budget if needed); at most 6fps for small idle motion (breathing, blinking, sleep); no frame at all when nothing changes.
- A poke or a mouse click shows its reaction within 100 ms.
- No heap allocation per frame once the window size is stable (AGENTS.md rule).
- Every frame wrapped in synchronized output; the terminal is restored on exit and on panic.
- The rendering rules of AGENTS.md hold: time-based animation, fixed simulation timestep with interpolation, springs or easing for motion.
- Transparent pixels are drawn with the terminal's default background, so kitty's `background_opacity 0` shows the desktop through them; dog pixels are opaque, without anti-aliasing.
- The window runs in kitty under `systemd-run --user` with `CPUQuota=10%` and `MemoryMax=128M`.
- New dependencies limited to small, pure-Rust crates (local time, randomness); third-party versions already in `Cargo.lock` do not change.
- Clippy clean with `-D warnings` across the workspace.

## Goal

`idlefx-companion` shows a procedurally animated dog in a small floating kitty window with a transparent background, on Hyprland and on GNOME. It walks, sits, lies down and sleeps on its own, follows the time of day, reacts to a mouse click, and reacts to events sent with `idlefx-companion poke <event>`. It renders at up to 60fps only while something moves fast, never uses more than 10% of a core, and uses well under 1% while it sleeps. It runs on reusable `idlefx-core` pieces: terminal session, transparent half-block canvas, frame loop with a CPU budget.

## Criteria

- [x] ISC-0: Common ground
  - [x] ISC-0.1: The testapp example and the companion build in release
  - [x] ISC-0.2: testapp prints frames, fps, cpu_ms and cpu_pct when it exits
  - [x] ISC-0.3: Companion --duration exits on time and --stats prints its stats
  - [x] ISC-0.4: A live --trace run writes time, from, to and cause lines
- [x] ISC-1: Core: terminal session
  - [x] ISC-1.1: A normal exit restores the screen, the cursor and mouse reporting
  - [x] ISC-1.2: After a panic the terminal is restored before the panic message
  - [x] ISC-1.3: Every core frame is bracketed by synchronized-output begin and end
  - [x] ISC-1.4: The plasma example runs on the core session
    - [x] ISC-1.4.1: plasma.rs uses idlefx_core and no longer calls ratatui init or restore
    - [x] ISC-1.4.2: The plasma example still draws truecolor half-block pixels
    - [x] ISC-1.4.3: A 120-frame plasma run still reports between 55 and 62 fps
    - [x] ISC-1.4.4: Pressing q still quits the plasma example within two seconds
- [x] ISC-2: Core: transparent half-block canvas
  - [x] ISC-2.1: Four canvas cell-case tests pass, transparent halves keep Color::Reset
  - [x] ISC-2.2: Ellipse, capsule, rectangle and pixel primitives match expected pixel maps
  - [x] ISC-2.3: Redrawing the canvas at a stable size performs no heap allocation
- [x] ISC-3: Core: frame loop with activity levels and a CPU budget
  - [x] ISC-3.1: An Active app under budget renders between 55 and 62 fps
  - [x] ISC-3.2: An Idle(4) app renders between 3.5 and 4.5 fps over 5 seconds
  - [x] ISC-3.3: An Asleep app draws one frame and uses under 0.1 s CPU in 10 s
  - [x] ISC-3.4: A Waker event reaches a sleeping app and draws a frame within 50 ms
  - [x] ISC-3.5: An expensive app with a 10% budget lowers its fps and stays under 10% CPU
  - [x] ISC-3.6: The budget reads cpu.stat in a .service cgroup and /proc otherwise
  - [x] ISC-3.7: With 50 ms frame writes, frames drop and simulation keeps real time
- [x] ISC-4: Dog: procedural body and animation
  - [x] ISC-4.1: Snapshot renders each of the 8 poses with half-blocks and exits 0
  - [x] ISC-4.2: Every pose stays inside a 64x32 pixel canvas
  - [x] ISC-4.3: The standing dog measures 26-34 by 16-24 pixels
  - [x] ISC-4.4: No part moves over 2 pixels per 60fps frame between any two poses
  - [x] ISC-4.5: A planted foot slides under 1 pixel per walking stride
  - [x] ISC-4.6: The user finds the dog cute and its motion smooth in kitty
- [x] ISC-5: Dog: behaviour
  - [x] ISC-5.1: A seeded 10-minute daytime simulation visits idle, wander, sit, lie and sleep
  - [x] ISC-5.2: Starting at 23:30 the dog sleeps by 23:32 and stays asleep 10 minutes
  - [x] ISC-5.3: The process CPU limits hold in each state in a 64x16 session
    - [x] ISC-5.3.1: The sleeping dog uses under 1% CPU over 10 s
    - [x] ISC-5.3.2: The idle awake dog uses under 3% CPU over 10 s
    - [x] ISC-5.3.3: The walking dog uses under 6% CPU over 10 s
  - [x] ISC-5.4: Every simulated trace line names a known cause for its state change
- [x] ISC-6: Interaction: mouse
  - [x] ISC-6.1: A click on a dog pixel starts the happy reaction within 100 ms
  - [x] ISC-6.2: A click on empty ground makes the dog walk and stop within 2 px
  - [x] ISC-6.3: A click wakes a sleeping dog
  - [x] ISC-6.4: Pressing q quits the companion within 1 s and restores the terminal
- [x] ISC-7: Events: socket and poke
  - [x] ISC-7.1: Each of the 5 poked events starts its reaction within 100 ms
  - [x] ISC-7.2: An unknown event changes nothing and the companion keeps running
  - [x] ISC-7.3: poke without a running companion exits 0 silently within 200 ms
  - [x] ISC-7.4: The companion starts and takes pokes despite a stale socket file
  - [x] ISC-7.5: The idlefx-socket thread stays blocked while no event arrives
- [x] ISC-8: Window: launch on Hyprland and GNOME
  - [x] ISC-8.1: launch starts the unit with CPUQuota=10% and MemoryMax=128M
  - [x] ISC-8.2: launch passes kitty --class and 64x16 cells by default, 80x20 with --window
  - [x] ISC-8.3: A second launch exits 0 and leaves exactly one unit
  - [x] ISC-8.4: The shipped kitty.conf sets opacity 0, no decorations and no padding
  - [x] ISC-8.5: The GNOME extension and Hyprland rules ship and match the app id
  - [x] ISC-8.6: On Hyprland the dog floats borderless on every workspace with desktop showing
  - [x] ISC-8.7: On GNOME with the extension the dog stays on top everywhere with desktop showing
  - [x] ISC-8.8: On the user's desktop the unit stays under 10% CPU while the dog walks smoothly
- [x] ISC-9: Docs: what was decided
  - [x] ISC-9.1: crates/ship/AGENTS.md states the five ship answers
  - [x] ISC-9.2: crates/companion/AGENTS.md documents window, CPU ceiling, events, launch and desktops
  - [x] ISC-9.3: Root AGENTS.md describes the session, canvas and frame loop with its CPU budget
- [x] ISC-10: The walking dog renders between 55 and 62 fps
- [x] ISC-11: The idle awake dog renders at most 6 fps
- [x] ISC-12: The companion's resident memory stays under 32 MB
- [x] ISC-13: Anti: cargo clippy reports any warning across the workspace
- [x] ISC-14: Anti: a third-party version already in Cargo.lock changes
- [ ] ISC-15: [DROPPED — stricter than the spec, which allows small pure-Rust crates; ISC-14 already guards the locked versions]
- [x] ISC-16: Anti: the build installs hooks in Claude Code settings or git
- [x] ISC-17: Anti: foot support or mentions remain in the companion's launch, contrib or docs

## Test Strategy

```yaml
- isc: ISC-0.1
  serves: S1+S3+S5+S6+S7+S8
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && cargo build --release -q --workspace --all-targets && test -x target/release/examples/testapp && test -x target/release/idlefx-companion"
  why: every pty probe runs these two binaries
- isc: ISC-0.2
  serves: S1+S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode active --frames 10' /dev/null) && grep -aq 'frames=10 ' <<<\"$out\" && grep -aqE 'fps=[0-9.]+' <<<\"$out\" && grep -aqE 'cpu_ms=[0-9]+' <<<\"$out\" && grep -aqE 'cpu_pct=[0-9.]+' <<<\"$out\""
  why: the session and frame-loop probes read these stats
- isc: ISC-0.3
  serves: S5+S6+S7
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && s=$(date +%s%N) && out=$(XDG_RUNTIME_DIR=$d timeout 5 script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --duration 1.5 --stats' /dev/null) && e=$(( ($(date +%s%N)-s)/1000000 )) && test $e -ge 1400 -a $e -lt 3000 && grep -aqE 'cpu_pct=[0-9.]+' <<<\"$out\" && grep -aqE 'fps=[0-9.]+' <<<\"$out\" && grep -aqE 'max_rss_kb=[0-9]+' <<<\"$out\""
  why: "live probes need a bounded run and CPU, fps and memory figures"
- isc: ISC-0.4
  serves: S5+S6+S7
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && t=$(mktemp) && XDG_RUNTIME_DIR=$d timeout 5 script -qec \"stty rows 16 cols 64; ./target/release/idlefx-companion --duration 1 --trace $t\" /dev/null >/dev/null && test -s $t && awk '!/^[0-9][0-9]:[0-9][0-9]:[0-9][0-9](\\.[0-9]+)? [a-z-]+ [a-z]+ [a-z:]+/{bad=1} END{exit (bad || NR==0)}' $t"
  why: "behaviour, mouse and event probes read the trace file"
- isc: ISC-1.1
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode active --frames 10 --mouse' /dev/null) && grep -aq $'\\e\\[?1049l' <<<\"$out\" && grep -aq $'\\e\\[?25h' <<<\"$out\" && grep -aq $'\\e\\[?1000l' <<<\"$out\""
- isc: ISC-1.2
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode active --panic-after 5' /dev/null); m=$'\\e[?1049l'; rest=${out##*\"$m\"}; test \"$rest\" != \"$out\" && grep -aq $'\\e\\[?25h' <<<\"$out\" && grep -aq 'panicked' <<<\"$rest\""
- isc: ISC-1.3
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode active --frames 30' /dev/null) && test \"$(grep -ao $'\\e\\[?2026h' <<<\"$out\" | wc -l)\" -ge 30 && test \"$(grep -ao $'\\e\\[?2026l' <<<\"$out\" | wc -l)\" -ge 30"
- isc: ISC-1.4.1
  anchors_to: S1
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && grep -q 'idlefx_core' crates/core/examples/plasma.rs && ! grep -qE 'ratatui::(init|restore)' crates/core/examples/plasma.rs"
- isc: ISC-1.4.2
  anchors_to: S1
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && script -qec 'stty rows 40 cols 120; ./target/release/examples/plasma --frames 30' /dev/null | grep -aqE $'\\e\\[38;2;[0-9;]+m▀'"
  fails-when: no truecolor SGR directly followed by ▀ in the plasma output
- isc: ISC-1.4.3
  anchors_to: S1
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && script -qec 'stty rows 50 cols 160; ./target/release/examples/plasma --frames 120' /dev/null | grep -ao 'fps=[0-9.]*' | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v>=55 && v<=62)}'"
  fails-when: the plasma fps stat is missing or outside 55-62
- isc: ISC-1.4.4
  anchors_to: S1
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && (sleep 0.7; printf q) | timeout 3 script -qec 'stty rows 40 cols 120; ./target/release/examples/plasma' /dev/null >/dev/null"
  fails-when: the plasma example ignores q and timeout kills it
- isc: ISC-2.1
  anchors_to: S2
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-core --lib canvas::tests::cell_ 2>&1) && grep -qE 'test result: ok\\. ([4-9]|[1-9][0-9]+) passed' <<<\"$out\""
- isc: ISC-2.2
  anchors_to: S2
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-core --lib canvas::tests::prim_ 2>&1) && grep -qE 'test result: ok\\. ([4-9]|[1-9][0-9]+) passed' <<<\"$out\""
- isc: ISC-2.3
  anchors_to: S2
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-core --test no_alloc 2>&1) && grep -qE 'test result: ok\\. [1-9][0-9]* passed' <<<\"$out\""
- isc: ISC-3.1
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode active --frames 120' /dev/null | grep -ao 'fps=[0-9.]*' | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v>=55 && v<=62)}'"
- isc: ISC-3.2
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode idle --idle-fps 4 --duration 5' /dev/null | grep -ao 'fps=[0-9.]*' | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v>=3.5 && v<=4.5)}'"
- isc: ISC-3.3
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode asleep --duration 10' /dev/null) && f=$(grep -ao 'frames=[0-9]*' <<<\"$out\" | tail -1 | cut -d= -f2) && c=$(grep -ao 'cpu_ms=[0-9]*' <<<\"$out\" | tail -1 | cut -d= -f2) && test -n \"$f\" -a -n \"$c\" && test \"$f\" -le 1 -a \"$c\" -lt 100"
- isc: ISC-3.4
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode asleep --wake-after-ms 500 --duration 1.5' /dev/null) && w=$(grep -ao 'wake_latency_us=[0-9]*' <<<\"$out\" | tail -1 | cut -d= -f2) && f=$(grep -ao 'frames=[0-9]*' <<<\"$out\" | tail -1 | cut -d= -f2) && test -n \"$w\" -a -n \"$f\" && test \"$w\" -lt 50000 -a \"$f\" -eq 2"
- isc: ISC-3.5
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode active --burn-ms 4 --budget 0.10 --duration 12 --measure-after 2' /dev/null) && c=$(grep -ao 'cpu_pct=[0-9.]*' <<<\"$out\" | tail -1 | cut -d= -f2) && f=$(grep -ao 'fps=[0-9.]*' <<<\"$out\" | tail -1 | cut -d= -f2) && awk -v c=\"$c\" -v f=\"$f\" 'BEGIN{exit !(c!=\"\" && f!=\"\" && c<10 && f<55)}'"
- isc: ISC-3.6
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-core --lib budget::tests::source_ 2>&1) && grep -qE 'test result: ok\\. ([2-9]|[1-9][0-9]+) passed' <<<\"$out\""
- isc: ISC-3.7
  anchors_to: S3
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(script -qec 'stty rows 16 cols 64; ./target/release/examples/testapp --mode active --slow-write-ms 50 --duration 3' /dev/null) && r=$(grep -ao 'sim_wall=[0-9.]*' <<<\"$out\" | tail -1 | cut -d= -f2) && f=$(grep -ao 'fps=[0-9.]*' <<<\"$out\" | tail -1 | cut -d= -f2) && awk -v r=\"$r\" -v f=\"$f\" 'BEGIN{exit !(r!=\"\" && f!=\"\" && r>=0.95 && r<=1.05 && f<25)}'"
- isc: ISC-4.1
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && for pose in stand walk sit lie sleep happy sad curious; do out=$(script -qec \"stty rows 16 cols 64; ./target/release/idlefx-companion snapshot --pose $pose; echo rc=\\$?\" /dev/null) && grep -aq 'rc=0' <<<\"$out\" && grep -aqE '▀|▄' <<<\"$out\" || exit 1; done"
- isc: ISC-4.2
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion dog::tests::poses_fit_canvas 2>&1) && grep -qE 'test result: ok\\. [1-9][0-9]* passed' <<<\"$out\""
- isc: ISC-4.3
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion dog::tests::stand_bbox_size 2>&1) && grep -qE 'test result: ok\\. [1-9][0-9]* passed' <<<\"$out\""
- isc: ISC-4.4
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion dog::tests::pose_transitions_continuous 2>&1) && grep -qE 'test result: ok\\. [1-9][0-9]* passed' <<<\"$out\""
- isc: ISC-4.5
  anchors_to: S4
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && out=$(cargo test -q -p idlefx-companion dog::tests::walk_no_foot_sliding 2>&1) && grep -qE 'test result: ok\\. [1-9][0-9]* passed' <<<\"$out\""
- isc: ISC-4.6
  anchors_to: S4
  kind: manual
- isc: ISC-5.1
  anchors_to: S5
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && t=$(mktemp) && ./target/release/idlefx-companion simulate --seed 7 --start 10:00 --duration 600 --trace $t && s=$(awk '{print $3}' $t | sort -u) && grep -qx idle <<<\"$s\" && grep -qx wander <<<\"$s\" && grep -qx sit <<<\"$s\" && grep -qx lie <<<\"$s\" && grep -qx sleep <<<\"$s\""
- isc: ISC-5.2
  anchors_to: S5
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && t=$(mktemp) && ./target/release/idlefx-companion simulate --seed 7 --start 23:30 --duration 720 --trace $t && awk '$3==\"sleep\" && s==\"\" {s=$1; next} s!=\"\" && $2==\"sleep\" {b=1} END{exit !(s!=\"\" && s<=\"23:32:00\" && !b)}' $t"
- isc: ISC-5.3.1
  anchors_to: S5
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && out=$(XDG_RUNTIME_DIR=$d script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --state sleep --clock 23:30 --duration 10 --stats' /dev/null) && grep -ao 'cpu_pct=[0-9.]*' <<<\"$out\" | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v<1)}'"
- isc: ISC-5.3.2
  anchors_to: S5
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && out=$(XDG_RUNTIME_DIR=$d script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --state idle --clock 14:00 --duration 10 --stats' /dev/null) && grep -ao 'cpu_pct=[0-9.]*' <<<\"$out\" | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v<3)}'"
- isc: ISC-5.3.3
  anchors_to: S5
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && out=$(XDG_RUNTIME_DIR=$d script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --state walk --clock 14:00 --duration 10 --stats' /dev/null) && grep -ao 'cpu_pct=[0-9.]*' <<<\"$out\" | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v<6)}'"
- isc: ISC-5.4
  anchors_to: S5
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && t=$(mktemp) && ./target/release/idlefx-companion simulate --seed 7 --start 10:00 --duration 600 --trace $t && test \"$(wc -l <$t)\" -ge 5 && test -z \"$(awk '{print $4}' $t | sort -u | grep -vxE 'start|timer|night|morning|done|arrived|wake')\""
- isc: ISC-6.1
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && t=$(mktemp) && (sleep 1; printf '\\e[<0;33;12M\\e[<0;33;12m'; sleep 1.5) | XDG_RUNTIME_DIR=$d script -qec \"stty rows 16 cols 64; ./target/release/idlefx-companion --state idle --clock 14:00 --at 32 --duration 2.5 --trace $t\" /dev/null >/dev/null; awk '$3==\"happy\" && $4==\"pet\"' $t | grep -o 'latency_ms=[0-9.]*' | head -1 | awk -F= '{v=$2} END{exit !(NR>0 && v<100)}'"
- isc: ISC-6.2
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && t=$(mktemp) && (sleep 1; printf '\\e[<0;46;15M\\e[<0;46;15m'; sleep 5) | XDG_RUNTIME_DIR=$d script -qec \"stty rows 16 cols 64; ./target/release/idlefx-companion --state idle --clock 14:00 --at 32 --duration 6 --trace $t\" /dev/null >/dev/null; awk '$4==\"arrived\"' $t | grep -o 'x=[0-9.]*' | head -1 | awk -F= '{v=$2} END{exit !(NR>0 && v>43 && v<47)}'"
- isc: ISC-6.3
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && t=$(mktemp) && (sleep 1; printf '\\e[<0;10;3M\\e[<0;10;3m'; sleep 1.5) | XDG_RUNTIME_DIR=$d script -qec \"stty rows 16 cols 64; ./target/release/idlefx-companion --state sleep --clock 23:30 --at 32 --duration 2.5 --trace $t\" /dev/null >/dev/null; awk '$2==\"sleep\" && ($4==\"click\" || $4==\"pet\")' $t | grep -q ."
- isc: ISC-6.4
  anchors_to: S6
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && out=$( (sleep 0.7; printf q) | XDG_RUNTIME_DIR=$d timeout 1.7 script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --state idle --clock 14:00' /dev/null) && grep -aq $'\\e\\[?1049l' <<<\"$out\" && grep -aq $'\\e\\[?25h' <<<\"$out\""
- isc: ISC-7.1
  anchors_to: S7
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && t=$(mktemp) && (XDG_RUNTIME_DIR=$d script -qec \"stty rows 16 cols 64; ./target/release/idlefx-companion --state idle --clock 14:00 --duration 6 --trace $t\" /dev/null </dev/null >/dev/null &) && sleep 1 && XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke hello && sleep 0.8 && XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke thinking && sleep 0.8 && XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke success && sleep 0.8 && XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke failure && sleep 0.8 && XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke commit && sleep 2.5 && awk '$4 ~ /^poke:/ {for (i=5;i<=NF;i++) if ($i ~ /^latency_ms=/) {split($i,a,\"=\"); if (a[2]<100) ok[$4]=1}} END{exit !((\"poke:hello\" in ok) && (\"poke:thinking\" in ok) && (\"poke:success\" in ok) && (\"poke:failure\" in ok) && (\"poke:commit\" in ok))}' $t"
- isc: ISC-7.2
  anchors_to: S7
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && t=$(mktemp) && (XDG_RUNTIME_DIR=$d script -qec \"stty rows 16 cols 64; ./target/release/idlefx-companion --state idle --clock 14:00 --duration 3 --trace $t\" /dev/null </dev/null >/dev/null &) && sleep 1 && XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke bogus && sleep 0.5 && XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke hello && sleep 2.5 && ! grep -q 'poke:bogus' $t && grep -q 'poke:hello' $t"
- isc: ISC-7.3
  anchors_to: S7
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && ./target/release/idlefx-companion --help 2>&1 | grep -qw poke && d=$(mktemp -d) && s=$(date +%s%N) && out=$(XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke hello 2>&1) && e=$(( ($(date +%s%N)-s)/1000000 )) && test -z \"$out\" && test $e -lt 200"
- isc: ISC-7.4
  anchors_to: S7
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && t=$(mktemp) && python3 -c 'import socket,sys; socket.socket(socket.AF_UNIX).bind(sys.argv[1])' $d/idlefx-companion.sock && test -S $d/idlefx-companion.sock && (XDG_RUNTIME_DIR=$d script -qec \"stty rows 16 cols 64; ./target/release/idlefx-companion --state idle --clock 14:00 --duration 2.5 --trace $t\" /dev/null </dev/null >/dev/null &) && sleep 1 && XDG_RUNTIME_DIR=$d ./target/release/idlefx-companion poke hello && sleep 2 && grep -q 'poke:hello' $t"
- isc: ISC-7.5
  anchors_to: S7
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && (XDG_RUNTIME_DIR=$d script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --state sleep --clock 23:30 --duration 13' /dev/null </dev/null >/dev/null &) && sleep 1.5 && pid=$(pgrep -n -x idlefx-companio) && tid=$(grep -lx idlefx-socket /proc/$pid/task/*/comm | head -1 | cut -d/ -f5) && test -n \"$tid\" && a=$(awk '/^voluntary_ctxt_switches/{print $2}' /proc/$pid/task/$tid/status) && sleep 10 && b=$(awk '/^voluntary_ctxt_switches/{print $2}' /proc/$pid/task/$tid/status) && test $((b-a)) -le 2"
- isc: ISC-8.1
  anchors_to: S8
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && f=$(mktemp -d)/kitty && printf '#!/bin/sh\\nexec sleep 30\\n' >$f && chmod +x $f && IDLEFX_TERMINAL=$f ./target/release/idlefx-companion launch && sleep 0.5 && v=$(systemctl --user show idlefx-companion -p CPUQuotaPerSecUSec -p MemoryMax -p ActiveState); systemctl --user stop idlefx-companion 2>/dev/null; grep -qx 'CPUQuotaPerSecUSec=100ms' <<<\"$v\" && grep -qx 'MemoryMax=134217728' <<<\"$v\" && grep -qx 'ActiveState=active' <<<\"$v\""
- isc: ISC-8.2
  anchors_to: S8
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && a=$(./target/release/idlefx-companion launch --dry-run) && b=$(./target/release/idlefx-companion launch --window 80x20 --dry-run) && grep -q -- '--class=idlefx-companion' <<<\"$a\" && grep -q 'initial_window_width=64c' <<<\"$a\" && grep -q 'initial_window_height=16c' <<<\"$a\" && grep -q 'initial_window_width=80c' <<<\"$b\" && grep -q 'initial_window_height=20c' <<<\"$b\" && grep -qE '(^| )[^ ]*kitty ' <<<\"$a\""
- isc: ISC-8.3
  anchors_to: S8
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && f=$(mktemp -d)/kitty && printf '#!/bin/sh\\nexec sleep 30\\n' >$f && chmod +x $f && IDLEFX_TERMINAL=$f ./target/release/idlefx-companion launch && sleep 0.5 && IDLEFX_TERMINAL=$f ./target/release/idlefx-companion launch; rc=$?; n=$(systemctl --user list-units --all --no-legend 'idlefx-companion*' | wc -l); systemctl --user stop idlefx-companion 2>/dev/null; test $rc -eq 0 -a $n -eq 1"
- isc: ISC-8.4
  anchors_to: S8
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && f=crates/companion/contrib/kitty.conf && grep -qE '^\\s*background_opacity\\s+0(\\.0+)?\\s*$' $f && grep -qE '^\\s*hide_window_decorations\\s+yes\\s*$' $f && grep -qE '^\\s*window_padding_width\\s+0\\s*$' $f"
- isc: ISC-8.5
  anchors_to: S8
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && m=$(ls crates/companion/contrib/gnome/*/metadata.json | head -1) && jq -e '.uuid and (.[\"shell-version\"] | length > 0)' $m >/dev/null && test -f \"$(dirname $m)/extension.js\" && grep -q 'idlefx-companion' \"$(dirname $m)/extension.js\" && test -f crates/companion/contrib/hyprland/idlefx-companion.conf && grep -q 'idlefx-companion' crates/companion/contrib/hyprland/idlefx-companion.conf"
- isc: ISC-8.6
  anchors_to: S8
  kind: manual
- isc: ISC-8.7
  anchors_to: S8
  kind: manual
- isc: ISC-8.8
  anchors_to: S8
  kind: manual
- isc: ISC-9.1
  anchors_to: S9
  kind: doc
  tool: "cd /home/jolo/dev/idlefx && f=crates/ship/AGENTS.md && grep -qi 'gamepad' $f && grep -qiE 'slider|knob' $f && grep -qi 'cockpit' $f && grep -qiE 'top view|top camera|\\btop\\b' $f && grep -qiE 'side view|side camera|\\bside\\b' $f && grep -qi 'realistic' $f && grep -qiE 'accelerat' $f"
  fails-when: "one of gamepad, sliders/knobs, cockpit/top/side cameras, realistic feel or accelerated travel is missing"
- isc: ISC-9.2
  anchors_to: S9
  kind: doc
  tool: "cd /home/jolo/dev/idlefx && f=crates/companion/AGENTS.md && grep -q '64x16' $f && grep -q -- '--window' $f && grep -q '10%' $f && grep -qw hello $f && grep -qw thinking $f && grep -qw success $f && grep -qw failure $f && grep -qw commit $f && grep -q 'idlefx-companion launch' $f && grep -qi 'hyprland' $f && grep -qi 'gnome' $f"
  fails-when: "the window size, --window, 10%, an event name, the launch command, or the Hyprland or GNOME setup is missing"
- isc: ISC-9.3
  anchors_to: S9
  kind: doc
  tool: "cd /home/jolo/dev/idlefx && grep -qiE 'session' AGENTS.md && grep -qiE 'canvas' AGENTS.md && grep -qiE 'frame loop' AGENTS.md && grep -qiE 'cpu budget' AGENTS.md"
  fails-when: "root AGENTS.md lacks one of session, canvas, frame loop or CPU budget"
- isc: ISC-10
  anchors_to: Constraints
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && out=$(XDG_RUNTIME_DIR=$d script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --state walk --clock 14:00 --duration 5 --stats' /dev/null) && grep -ao 'fps=[0-9.]*' <<<\"$out\" | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v>=55 && v<=62)}'"
  source: context
  why: "Constraints: up to 60fps while walking; no spec section states it as an outcome"
- isc: ISC-11
  anchors_to: Constraints
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && out=$(XDG_RUNTIME_DIR=$d script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --state idle --clock 14:00 --duration 10 --stats' /dev/null) && grep -ao 'fps=[0-9.]*' <<<\"$out\" | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v>0 && v<=6)}'"
  source: context
  why: "Constraints: at most 6fps for small idle motion; no spec section states it as an outcome"
- isc: ISC-12
  anchors_to: Constraints
  kind: behaviour
  tool: "cd /home/jolo/dev/idlefx && d=$(mktemp -d) && out=$(XDG_RUNTIME_DIR=$d script -qec 'stty rows 16 cols 64; ./target/release/idlefx-companion --state walk --clock 14:00 --duration 3 --stats' /dev/null) && grep -ao 'max_rss_kb=[0-9]*' <<<\"$out\" | tail -1 | awk -F= '{v=$2} END{exit !(NR>0 && v>0 && v<32768)}'"
  source: context
  why: "Constraints: resident memory under 32 MB; no spec section states it as an outcome"
- isc: ISC-13
  anchors_to: Constraints
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && cargo clippy -q --release --workspace --all-targets -- -D warnings"
  fails-when: "clippy prints a warning on any crate, test or example and exits non-zero"
- isc: ISC-14
  anchors_to: Constraints
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && f='/^name = /{n=$3} /^version = /{print n, $3}' && test -z \"$(comm -23 <(git show 9e953fe:Cargo.lock | awk \"$f\" | grep -v '^\"idlefx' | sort) <(awk \"$f\" Cargo.lock | grep -v '^\"idlefx' | sort))\""
  fails-when: a name/version pair locked at 9e953fe is missing from the new Cargo.lock
- isc: ISC-16
  anchors_to: Goal
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && ! grep -q 'idlefx-companion' ~/.claude/settings.json && ! test -e .git/hooks/post-commit"
  fails-when: ~/.claude/settings.json mentions idlefx-companion or .git/hooks/post-commit exists
- isc: ISC-17
  anchors_to: Goal
  kind: regression
  tool: "cd /home/jolo/dev/idlefx && ! test -e crates/companion/contrib/foot.ini && ! grep -qi 'foot' crates/companion/src/launch.rs crates/companion/src/main.rs mise.toml && test -z \"$(grep -i 'foot' crates/companion/AGENTS.md | grep -vi 'foot sliding')\""
  fails-when: "contrib/foot.ini exists, or launch.rs, main.rs, mise.toml or the companion AGENTS.md mention foot as a terminal"
```

## Decisions

- 2026-10-14 18:21: App::event returns Flow::Continue, Flow::Redraw or Flow::Quit: the loop redraws only when the app says so (and on resize). A pty under script injects Ctrl-D when its stdin closes; redrawing on every input made an Asleep app draw a second frame (ISC-3.3).
- 2026-10-14 18:21: No-allocation rule scoped to the canvas (ISC-2.3): ratatui's Terminal::draw allocates its diff list each frame internally; avoiding that would mean replacing ratatui's diffing, out of scope for v1.
- 2026-10-14 18:32: ISC-7.5 probe fixed: Linux truncates process names to 15 characters, so pgrep -x idlefx-companion never matched; it now matches idlefx-companio. The code was right (0 voluntary switches of idlefx-socket in 10 s, measured by hand).
- 2026-10-15 22:32: ISC-8.2 now runs launch with IDLEFX_TERMINAL=foot: since 3fe6731 launch defaults to kitty (--class, initial_window_width), while S8's line describes foot's flags; the foot path is what it checks.

## Review

Reviewed 2026-10-15 22:34 · criteria #391d963b · Jev: 1 request, 69 questions
- shared: ISC-0.1–0.4 (release binaries, testapp stats, companion --duration/--stats, the trace format) are what the S1, S3, S5, S6, S7 and S8 probes all run on, so they come first.
- prerequisites: S3's loop needs S1's session and S2's canvas; S4–S7 need S3's App/Waker; ISC-0.1 must build before any pty probe; S8's Hyprland rules need the user's `hyprctl version` before they are written.
- contradictions: none
- drift: ISC-8.1 and 8.3 run launch with a stand-in binary named kitty (no kitty or Wayland here), so they prove the unit and its limits but not that kitty opens; ISC-8.5 only proves the Hyprland rules and GNOME extension exist and name the app id, whether they work is the manual ISC-8.6/8.7.
- gaps: S4's turn-by-squash, S5's morning stretch and S7's 60 s cap on `thinking` have no probe of their own; they are covered only by the manual look (ISC-4.6) and the trace-based reaction checks.
- context: ISC-10 (walking 55–62 fps), ISC-11 (idle at most 6 fps), ISC-12 (RSS under 32 MB) come from Constraints; ISC-16 (no installed hooks) and ISC-17 (no foot left in the companion) anchor to the Goal; ISC-15 was dropped as stricter than the spec.
- redundant: ISC-5.3.1 (sleeping CPU) and ISC-7.5 (socket thread blocked) both look at the sleeping dog but prove different things; ISC-1.4.2–1.4.4 repeat the plasma checks of the workspace ISA on purpose, as regressions.

Flags (each needs an answer before the ISA ack):
- R1: covered S8 — no 0.78 ≥ 0.70 — Do the criteria under ISC-8 in `criteria` together cover everything section S8 ("Window: launch on Hyprland and GNOME") asks for in its Accepted-when lines? Answer no when an Accepted-when outcome has no criterion that would prove it. — answer: keep: no launch outcome would go unproven — the quota and 128M memory cap (ISC-8.1), kitty's class and window size in cells (ISC-8.2), the single running unit (ISC-8.3), and the floating, on-top, transparent window and its real CPU on the user's desktop (ISC-8.6–8.8, manual since this machine has no Wayland) each have a criterion
- R2: serves ISC-14 — no 0.81 ≥ 0.70 — Does criterion ISC-14 ("Anti: a third-party version already in Cargo.lock changes", see `criteria`) help solve the spec's stated Problem and reach its Goal? Answer yes when the Problem needs what it claims; no when it is unrelated to the Problem and the Goal, or only restates a detail no part of the spec asks for. — answer: keep: without it, adding a crate could silently bump a version already locked, which the Constraints forbid and no other probe would notice
- R3: serves ISC-16 — no 0.86 ≥ 0.70 — Does criterion ISC-16 ("Anti: the build installs hooks in Claude Code settings or git", see `criteria`) help solve the spec's stated Problem and reach its Goal? Answer yes when the Problem needs what it claims; no when it is unrelated to the Problem and the Goal, or only restates a detail no part of the spec asks for. — answer: keep: without it, writing into the user's Claude Code settings or the repo's git hooks during the build would go unnoticed; the hooks are meant as snippets only

## Verification

- Goal: yes — a procedural spring-animated dog lives in its own transparent kitty window under systemd-run (CPUQuota=10%, MemoryMax=128M); it idles, wanders, sits, lies and sleeps by itself with a 22:00–07:00 night, reacts to clicks (pet, call) and to the 5 poke events in under 100 ms; measured here at under 1% asleep, under 3% idle and 2–2.5% walking at 55–62 fps; the user confirmed the look in kitty, the window on Hyprland and on GNOME, and the unit under 10% CPU while walking; it runs on idlefx-core's session, transparent canvas and frame loop with a CPU budget
- Never seen failing: ISC-7.5, ISC-8.1, ISC-8.2, ISC-8.3, ISC-8.4
