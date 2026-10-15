# idlefx

Things that live in a terminal with **stunning, smooth animations**: idle
companions, and a spaceship game with realistic flight (Star Citizen-like
6DOF), both built on one shared rendering core.

## Goals

- **Look great in a terminal.** Truecolor, sub-cell resolution, fluid motion.
  The bar is "people stop and watch it", not "it works".
- **Smooth.** 60fps when things move, no tearing, no flicker, no stutter.
- **Idle-friendly.** Runs all day in a spare pane. When nothing moves, frame
  rate drops (~2fps or sleep). CPU usage at rest must be negligible.
- **One renderer, many ideas.** Build a solid rendering core first; the
  companion and the ship game are content on top of it.

## Layout

One Cargo workspace, one repo:

- `crates/core` — `idlefx-core`, the shared library, in three pieces:
  - **session** (`Session`): raw mode, alternate screen, hidden cursor,
    optional SGR mouse and Kitty keyboard protocol; restored on exit and from
    the panic hook (before the panic message); every frame wrapped in
    synchronized output.
  - **canvas** (`Canvas`): a half-block pixel canvas, 1 pixel wide and 2 tall
    per cell, where a pixel is a color or transparent (drawn with the default
    background, so a translucent terminal shows the desktop). Hard-edged
    ellipse, capsule, rectangle and pixel primitives.
  - **frame loop** (`Runner`, `App`): a fixed 1/120 s simulation step with
    interpolation, input on its own thread, a `Waker` for events from other
    threads, and frame rates by `Activity`: `Active` (up to 60fps), `Idle(n)`
    (n ≤ 6 fps) or `Asleep` (no frames). An optional **CPU budget** measures
    CPU use (the systemd `.service` cgroup when the app runs in one, else
    `/proc/self/stat`) and steps the active rate 60 → 30 → 20 to stay under it.
  Examples: `plasma` (`cargo run --release --example plasma`) and `testapp`
  (drives the loop for the tests).
- `crates/companion` — `idlefx-companion`, the dog companion (binary).
- `crates/ship` — `idlefx-ship`, the spaceship game (binary).

Each sub-project has its own `AGENTS.md` for its scope and rules; this file
holds what they share. Shared dependency versions live in the root
`Cargo.toml` under `[workspace.dependencies]`.

## Stack

- **Rust** (stable, edition 2024, pinned in `mise.toml`), one static binary
  per sub-project.
- **ratatui** — immediate-mode TUI, cell buffer with frame diffing.
- **crossterm** — backend: raw mode, alternate screen, input, synchronized output.
- **tachyonfx** — shader-like effects and transitions on ratatui buffers.
- **Custom half-block framebuffer** for the art layer: `▀` with fg = top pixel,
  bg = bottom pixel, so each cell is 2 truecolor pixels. Braille / octants are
  options for finer, monochrome-ish detail.

Target terminals: foot, Kitty, Ghostty, WezTerm (truecolor, DEC mode 2026).
foot is the daily terminal and renders on the CPU, so it is the performance
stress test; Kitty is the reference for the Kitty keyboard protocol (key
release events). Kitty graphics protocol is out of scope unless decided
otherwise.

## Rendering rules

- Wrap every frame in synchronized output (begin/end sync update).
- Use the alternate screen; restore the terminal on exit **and** on panic.
- Animation is time-based (`dt` from a monotonic clock), never per-frame steps.
- Simulation runs on a fixed timestep; rendering interpolates between states.
- Motion uses easing or springs, not linear jumps.
- Adaptive frame rate: render only when something changed or an animation
  is active.
- No per-frame heap allocation in the hot path; reuse buffers.

## Ideas backlog

- **Art:** shader-style `f(x, y, t) -> rgb` scenes — plasma, flow fields,
  boids, reaction–diffusion, starfields.
- **Ship:** realistic spaceship flight — see `crates/ship/AGENTS.md`.
- **Companion:** a sprite with a state machine (idle / sleep / react) — see
  `crates/companion/AGENTS.md`.

## First milestone (done)

A ~150-line prototype: half-block framebuffer → ratatui buffer → diffed,
synchronized output, rendering a plasma effect at 60fps. Smoothness judged
fine in the target terminals; it now lives in `crates/core/examples/plasma.rs`.

## Second milestone: the dog companion

The core's first pieces and a procedural dog in its own floating window:
spec `docs/2026-10-14-dog-companion-01-spec.md`, details in
`crates/companion/AGENTS.md`.
