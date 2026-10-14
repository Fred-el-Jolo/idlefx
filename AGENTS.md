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

- `crates/core` — `idlefx-core`, the shared library: terminal setup and
  restore, half-block framebuffer, frame loop, input. Its API is not designed
  yet. The plasma prototype is its example: `cargo run --release --example plasma`.
- `crates/companion` — `idlefx-companion`, the idle companions (binary).
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

Next: design the core API (to brainstorm before any spec).
